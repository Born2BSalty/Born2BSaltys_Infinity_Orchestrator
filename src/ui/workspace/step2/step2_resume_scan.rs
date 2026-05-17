// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_resume_scan` — **cold-resume Step-2 restore** (Phase 6 / Run 2b —
// the #1 fix). Net-new orchestrator glue; BIO read-only.
//
// ## The bug this fixes (a Run-1 MISS vs P6.T1)
//
// `workspace_state_loader::populate_wizard_state_from_workspace` applies the
// persisted `order_<tab>` onto `wizard_state.step2.<tab>_mods` (via
// `apply_order_to_mods`) but **never restores that scanned mod set**. On a
// cold resume (save draft → quit → relaunch → resume) the scan has not run
// this session, so the mod vectors are empty, nothing matches, and Step 2 /
// Step 3 come up empty — every selection appears lost.
//
// P6.T1's file inventory specified `wizard_state.step2.*` is reconstructed
// from the order arrays **+ cached scan results** — *"reuse [BIO's] existing
// scan-cache machinery"*. This module is the missing half.
//
// ## Why re-trigger BIO's scan (not re-implement the cache read)
//
// Premise-checked against BIO source:
//   - BIO's scan path **does persist the cache**: the scan worker
//     (`bio::app::scan::worker::scan_impl`,
//     `worker.rs:244-247`) calls `save_scan_cache(&cache)` at the end of
//     every scan, writing `%APPDATA%\bio\bio_scan_cache.json`.
//   - The cache is keyed per-tp2 (`cache_key(tp2)`), each entry tagged with
//     the `cache_context(weidu, game_dir, mods_root)` string + the tp2's
//     mtime/size signature.
//   - BIO's `scan_impl` is itself the canonical cache *consumer*: per tp2
//     (`worker_scan_group.rs:42-60`) a `cache_get` hit **skips WeiDU
//     entirely** (`continue`). So re-running BIO's scan on a fresh cache +
//     unchanged tp2 files reads from cache (fast, no WeiDU subprocess) and
//     rebuilds the full `Step2ModState` set via BIO's own
//     `to_mod_states` — **no reimplementation**, no drift. (Re-implementing
//     the cache→`Step2ModState` reconstruction would duplicate
//     discovery + `cache_get` orchestration + `to_mod_states` — exactly the
//     complex duplication the CRITICAL DIRECTIVE forbids; SPEC §1
//     decision-order step 1: direct reuse of BIO's `pub(crate)` scan path.)
//
// `cache_context` is rebuilt on resume because all three inputs are
// recoverable: `weidu` + `game_dir` come from Settings (synced into
// `wizard_state.step1` by `sync_paths_from_settings` on workspace open), and
// `mods_root` is the **recorded dev-scan folder** — persisted into
// `ModlistWorkspaceState.dev_scanned_mods_folder` by the dev-scan trigger
// (`step2_dev_scan::pick_folder_and_scan`) precisely so resume can re-point
// it. Without that recording the folder was lost on relaunch
// (`sync_paths_from_settings` overwrites `mods_folder` from settings, which
// has no per-install mods folder pre-Phase-7 — SPEC §13.12a).
//
// ## The async ordering (reuses the rescan-reconcile seam)
//
// The loader is synchronous; BIO's scan is async (worker thread, drained by
// `OrchestratorApp::poll_step2_channels`). This module reuses the existing
// `step2_rescan_reconcile` snapshot/complete machinery: at workspace open it
// builds a `RescanSnapshot` from the **persisted order** (not the in-memory
// selection — there is none yet), stashes it + sets `resume_pending`, and
// dispatches `StartScan`. On the scan-completion edge,
// `reconcile_on_scan_complete` re-applies the snapshot onto the fresh mods
// (the existing path) and — because `resume_pending` is set (no prior Step-3
// reorder to protect) — also rebuilds Step 3 via BIO's
// `step3_sync::build_step3_items`, the loader's own recipe.
//
// ## Production (pre-Phase-7) is a legitimate no-op
//
// There is no production dev-scan, so `dev_scanned_mods_folder` is `None`
// and this is a no-op — production pre-Phase-7 still legitimately finds
// nothing (the §13.12a deferral, not this bug). Gated on `dev_mode` for the
// same reason (the recording only exists on the dev path).
//
// SPEC: §13.1 (per-modlist workspace state), §13.12a (per-install
//       extracted-mods folder — dev-scan is the functional path pre-Phase-7),
//       §6.3 (reuses the rescan-reconcile completion seam), §1 (decision
//       order — direct reuse of BIO's scan + cache, BIO read-only).

use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};
use crate::ui::workspace::step_action_dispatch;

/// Called from `page_router::render_workspace` **immediately after**
/// `populate_wizard_state_from_workspace`, with the just-loaded workspace
/// state. If a cold-resume scan is needed, re-point `step1.mods_folder` to
/// the recorded dev-scan folder, stash a `RescanSnapshot` built from the
/// persisted order, set `resume_pending`, and dispatch `StartScan` through
/// the **existing** Step-2 dispatch path. No-op otherwise.
///
/// Conditions (all must hold):
///   - `orchestrator.dev_mode` — the only path that records a dev-scan
///     folder (production has none pre-Phase-7; SPEC §13.12a).
///   - `workspace.dev_scanned_mods_folder` is `Some(non-empty)`.
///   - The workspace has a persisted order on some tab (otherwise there is
///     nothing to restore — a brand-new modlist legitimately scans fresh).
///   - The freshly-populated Step-2 mod set is empty (nothing scanned this
///     session yet) — so we don't stomp an in-session scan / dev-rescan.
///   - No Step-2 scan is already in flight.
pub fn maybe_trigger_resume_scan(
    orchestrator: &mut OrchestratorApp,
    workspace: &ModlistWorkspaceState,
) {
    if !orchestrator.dev_mode {
        return;
    }
    let Some(folder) = workspace
        .dev_scanned_mods_folder
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
    else {
        return;
    };
    let has_order = !workspace.order_bgee.is_empty()
        || !workspace.order_bg2ee.is_empty()
        || !workspace.order_iwdee.is_empty();
    if !has_order {
        return;
    }
    let step2_empty = orchestrator.wizard_state.step2.bgee_mods.is_empty()
        && orchestrator.wizard_state.step2.bg2ee_mods.is_empty();
    if !step2_empty {
        return;
    }
    if orchestrator.wizard_state.step2.is_scanning {
        return;
    }

    // Re-point BIO's scan source to the recorded folder (the exact field
    // BIO's scan worker reads — `step1.mods_folder`). `sync_paths_from_
    // settings` (run by `populate`) just set this from settings (empty
    // pre-Phase-7); this overrides it for the dev-resume path only.
    orchestrator.wizard_state.step1.mods_folder = folder.to_string();

    // Build the snapshot from the **persisted order** (not the in-memory
    // selection — there is none on a cold resume). Mirrors
    // `workspace_state_loader::apply_order_to_mods`: match by upper-cased
    // `tp2` + `component_id`, `selected_order = 1-based position`. IWDEE
    // shares BIO's BGEE bucket (the loader's own routing), so its order
    // restores onto the BGEE snapshot tab.
    let mut bgee = snapshot_from_order(&workspace.order_bgee);
    if !workspace.order_iwdee.is_empty() {
        // Re-base IWDEE positions after the BGEE ones so a (rare) mixed file
        // doesn't collide order indices; in practice only one is populated.
        let base = bgee.len();
        for (i, sel) in snapshot_from_order(&workspace.order_iwdee)
            .into_iter()
            .enumerate()
        {
            bgee.push(RescanSelection {
                selected_order: Some(base + i + 1),
                ..sel
            });
        }
    }
    let snapshot = RescanSnapshot {
        bgee,
        bg2ee: snapshot_from_order(&workspace.order_bg2ee),
    };

    let step2 = &mut orchestrator.workspace_view.step2;
    step2.rescan_snapshot = Some(snapshot);
    step2.rescan_drop_warning = None;
    step2.resume_pending = true;
    // Seed the completion-edge detector with the live value (mirrors
    // `step2_rescan_reconcile::snapshot_current_selection`).
    step2.was_scanning = orchestrator.wizard_state.step2.is_scanning;

    // Kick the scan through the **existing** dispatch path — identical to
    // what the dev-scan button / toolbar Rescan does (BIO's
    // `Step2Action::StartScan` → `bio::app::app_step2_router::
    // handle_step2_action` → `app_step2_scan::start_step2_scan`). BIO's scan
    // reads its own persisted cache; on a cache hit per tp2 it skips WeiDU.
    step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);
}

/// Build a per-tab `RescanSelection` list from a persisted `order` vector.
/// Each entry's `selected_order` is its **1-based position** in the order
/// (the same contract `workspace_state_loader::apply_order_to_mods` uses);
/// `tp2` is upper-cased (BIO matches `tp_file` case-insensitively — the
/// `apply_order_to_mods` precedent).
fn snapshot_from_order(order: &[ComponentRef]) -> Vec<RescanSelection> {
    order
        .iter()
        .enumerate()
        .map(|(i, c)| RescanSelection {
            tp2_upper: c.tp2.to_ascii_uppercase(),
            component_id: c.id.to_string(),
            selected_order: Some(i + 1),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_order_is_1_based_and_upper_tp2() {
        let order = vec![
            ComponentRef {
                tp2: "BG1UB/SETUP-BG1UB.TP2".to_string(),
                id: 0,
                language: 0,
            },
            ComponentRef {
                tp2: "EeFixPack/EeFixPack.TP2".to_string(),
                id: 2,
                language: 0,
            },
        ];
        let snap = snapshot_from_order(&order);
        assert_eq!(snap.len(), 2);
        // Upper-cased tp2 (matches BIO's case-insensitive `tp_file` match).
        assert_eq!(snap[0].tp2_upper, "BG1UB/SETUP-BG1UB.TP2");
        assert_eq!(snap[0].component_id, "0");
        assert_eq!(snap[0].selected_order, Some(1));
        assert_eq!(snap[1].tp2_upper, "EEFIXPACK/EEFIXPACK.TP2");
        assert_eq!(snap[1].component_id, "2");
        // 1-based position preserves the persisted install order.
        assert_eq!(snap[1].selected_order, Some(2));
    }

    #[test]
    fn empty_order_yields_empty_snapshot() {
        assert!(snapshot_from_order(&[]).is_empty());
    }

    // ── The cold-resume reconstruction (the #1 fix) end-to-end, on pure
    //    state — no `OrchestratorApp::new`, no store (test-hygiene rule;
    //    the `step2_rescan_reconcile` / `workspace_state_loader` pure-fn
    //    precedent). It reproduces the exact sequence the fix performs:
    //
    //      1. cold resume: `populate` runs on an EMPTY scanned mod set
    //         (the scan hasn't run this session) → Step 2 + Step 3 come up
    //         empty — the bug.
    //      2. the resume snapshot is built from the **persisted order**
    //         (`snapshot_from_order`).
    //      3. BIO's scan completes and lands the fresh mod set (here we
    //         supply it directly — equivalent to BIO's cache-fed scan).
    //      4. the resume reconcile re-marks the snapshot onto the fresh
    //         mods (the exact 3-line `reapply_snapshot` body) + rebuilds
    //         Step 3 via BIO's `build_step3_items` (the loader's recipe).
    //      5. assert Step 2 selections + order AND Step 3 items are fully
    //         restored, in the persisted order. ──
    use crate::app::controller::step3_sync;
    use crate::app::state::{Step2ComponentState, Step2ModState, Step3ItemState, WizardState};
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use crate::settings::store::SettingsStore;
    use crate::ui::workspace::workspace_state_loader;

    fn comp(id: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: format!("Component {id}"),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: format!("~BG1UB/BG1UB.TP2~ #0 #{id} // Component {id}"),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    fn mod_state(name: &str, tp: &str, comps: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: tp.to_string(),
            tp2_path: format!("/mods/{tp}"),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: comps,
        }
    }

    /// The make-or-break test for the #1 fix: a cold resume with an empty
    /// scanned set + a persisted order restores Step 2 selections AND the
    /// Step 3 order once the (resume-triggered) scan lands the mods.
    #[test]
    fn cold_resume_restores_step2_and_step3_after_scan_lands() {
        // (1) Cold resume: empty Step-2 mods + a persisted order. `populate`
        //     applies the order onto nothing → Step 2/3 empty (the bug this
        //     fix exists to repair).
        let mut ws = WizardState::default();
        let mut workspace = ModlistWorkspaceState::default();
        workspace.order_bgee = vec![
            ComponentRef {
                tp2: "BG1UB/BG1UB.TP2".to_string(),
                id: 11,
                language: 0,
            },
            ComponentRef {
                tp2: "BG1UB/BG1UB.TP2".to_string(),
                id: 0,
                language: 0,
            },
        ];
        workspace.dev_scanned_mods_folder = Some("/some/scanned/mods".to_string());
        let entry = ModlistEntry {
            id: "RESUMETEST00".to_string(),
            name: "Resume".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            ..Default::default()
        };

        workspace_state_loader::populate_wizard_state_from_workspace(
            &workspace,
            &entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        // The bug, demonstrated: nothing restored on a cold resume.
        assert!(
            ws.step2.bgee_mods.is_empty(),
            "cold resume: no scanned mods yet"
        );
        assert!(ws.step3.bgee_items.is_empty(), "cold resume: Step 3 empty");

        // (2) The resume snapshot is built from the persisted order.
        let snapshot = snapshot_from_order(&workspace.order_bgee);
        assert_eq!(snapshot.len(), 2);

        // (3) BIO's resume-triggered scan completes and lands the fresh mod
        //     set (BIO's cache-fed scan; supplied directly here).
        ws.step2.bgee_mods = vec![mod_state(
            "BG1UB",
            "BG1UB/BG1UB.TP2",
            vec![comp("0"), comp("11"), comp("5")],
        )];

        // (4) The resume reconcile: re-mark the snapshot onto the fresh
        //     mods (the exact `step2_rescan_reconcile::reapply_snapshot`
        //     body) + rebuild Step 3 via BIO's `build_step3_items` (the
        //     `workspace_state_loader::populate` recipe, run by the
        //     `resume_pending` branch of `reconcile_on_scan_complete`).
        for sel in &snapshot {
            for m in &mut ws.step2.bgee_mods {
                if m.tp_file.to_ascii_uppercase() != sel.tp2_upper {
                    continue;
                }
                for c in &mut m.components {
                    if c.component_id == sel.component_id {
                        c.checked = true;
                        c.selected_order = sel.selected_order;
                    }
                }
            }
        }
        for m in &mut ws.step2.bgee_mods {
            m.checked = m.components.iter().any(|c| c.checked);
        }
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);

        // (5) Step 2 restored: #0 (order 2) and #11 (order 1) checked; #5
        //     stays off. The mod tri-state is re-derived.
        let m = &ws.step2.bgee_mods[0];
        assert!(m.components[0].checked, "#0 restored checked");
        assert_eq!(m.components[0].selected_order, Some(2));
        assert!(m.components[1].checked, "#11 restored checked");
        assert_eq!(m.components[1].selected_order, Some(1));
        assert!(!m.components[2].checked, "#5 was never selected");
        assert!(m.checked, "mod tri-state re-derived");

        // Step 3 restored in the **persisted order**: #11 (order 1) before
        // #0 (order 2), with a synthetic parent header row.
        let leaves: Vec<&Step3ItemState> = ws
            .step3
            .bgee_items
            .iter()
            .filter(|i| !i.is_parent)
            .collect();
        assert_eq!(leaves.len(), 2, "two component rows restored");
        assert_eq!(leaves[0].component_id, "11");
        assert_eq!(leaves[1].component_id, "0");
        assert!(
            ws.step3.bgee_items.iter().any(|i| i.is_parent),
            "BIO's build_step3_items emitted the parent header row"
        );
    }
}
