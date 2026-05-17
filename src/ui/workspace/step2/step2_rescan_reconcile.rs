// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step2_rescan_reconcile` — the **rescan-reconcile** logic (P6.T2c; the #2
// fix). Net-new orchestrator logic implementing SPEC §6.3: a rescan is
// **non-destructive — it never wipes the user's choices**. It re-scans the
// mods folder, then **re-applies the current selection set onto the
// freshly-scanned mod list** (matched by `tp2` + component id, preserving
// each component's `selected_order`), **dropping only selections whose mod
// or component is no longer present**; when any are dropped, a warning is
// surfaced in the scan-status footer.
//
// ## Why net-new orchestrator logic (premise-checked)
//
// BIO has **no** reusable rescan-preserves-selection mechanism — BIO's
// `Step2ScanEvent::Finished` handler (`app_step2_scan_events.rs:73-117`)
// unconditionally replaces `state.step2.{bgee,bg2ee}_mods` and resets
// `selected = None` / `next_selection_order = 1`; the old selection is
// gone. So this preservation is net-new orchestrator work. It is **not** a
// BIO edit and **not** a reimplementation of BIO's scan — it runs *after*
// BIO's own scan-event handler has landed the fresh mod set, and only
// re-marks `checked` / `selected_order` on it.
//
// ## The async ordering (the main hazard — handled here)
//
// BIO's scan is async (a worker thread; events drained by
// `OrchestratorApp::poll_step2_channels`). At scan-**trigger** time the
// scan has not run — so we cannot reconcile synchronously. Instead:
//
//   1. **Trigger time** (`snapshot_current_selection`, called from
//      `step2_dev_scan::pick_folder_and_scan` *before* `StartScan` is
//      dispatched): capture the current selection — every checked component
//      on both tabs as `(tp2.to_ascii_uppercase(), component_id,
//      selected_order)` — into orchestrator-owned `workspace_view.step2`.
//      (The loader's `apply_order_to_mods` is the precedent for these
//      fields: it matches `tp_file` upper-cased + `component_id` and writes
//      `checked` + `selected_order`.)
//   2. **Scan-completion edge** (`reconcile_on_scan_complete`, called from
//      `OrchestratorApp::update` *immediately after* `poll_step2_channels`):
//      when `is_scanning` transitions `true → false` AND the scan finished
//      **successfully** (`last_scan_report.is_some()` — distinguishes
//      `Finished` from `Canceled`/`Failed`/`Disconnected`, on which BIO
//      does **not** replace the mod vectors so there is nothing to
//      reconcile) AND a snapshot is pending: re-apply the snapshot onto the
//      now-fresh mods, recompute per-mod tri-state + `next_selection_order`
//      (exactly as `workspace_state_loader` does on workspace open — BIO
//      exposes no public per-mod tri-state recompute, so these mirror the
//      loader's own private `recompute_mod_checked` / `max_selected_order`),
//      and compute the drop / missing-mods warning.
//
// No confirmation dialog (the reconcile is non-destructive by
// construction — SPEC §6.3 / §6.10).
//
// ## Cold-resume reuse (Phase 6 / Run 2b — the #1 fix)
//
// `step2_resume_scan` reuses this exact completion seam: on workspace open
// it builds the snapshot from the **persisted order** (not an in-memory
// selection) + sets `workspace_view.step2.resume_pending`, then dispatches
// `StartScan`. `reconcile_on_scan_complete` re-applies that snapshot the
// same way — and, **only** when `resume_pending` is set, additionally
// rebuilds Step 3 via BIO's `step3_sync::build_step3_items` (the loader's
// recipe) because a cold resume's `populate` built Step 3 on an empty
// Step-2 set. A dev *rescan* leaves `resume_pending == false` and keeps
// BIO's Step-3 clobber-protection (the user's reorder is preserved).
//
// SPEC: §6.3 (rescan reconcile + missing-mods warning), §13.1 (per-modlist
//       workspace state — the #1 cold-resume path), §1 (decision order:
//       net-new sibling, BIO read-only), §13.12a (dev-scan is the
//       functional rescan path pre-Phase-7).

use crate::app::controller::step3_sync;
use crate::app::state::Step2ModState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};

/// (a) Capture the current selection BEFORE the scan (SPEC §6.3). Called
/// from the scan trigger (`step2_dev_scan::pick_folder_and_scan`) *before*
/// `Step2Action::StartScan` is dispatched, so the snapshot reflects the
/// pre-scan choices. Also resets `was_scanning`/`rescan_drop_warning` so the
/// completion-edge detector and the footer start clean for this rescan.
pub fn snapshot_current_selection(orchestrator: &mut OrchestratorApp) {
    let snapshot = RescanSnapshot {
        bgee: capture_tab(&orchestrator.wizard_state.step2.bgee_mods),
        bg2ee: capture_tab(&orchestrator.wizard_state.step2.bg2ee_mods),
    };
    let step2 = &mut orchestrator.workspace_view.step2;
    step2.rescan_snapshot = Some(snapshot);
    step2.rescan_drop_warning = None;
    // Seed the edge detector with the live value so a scan that is *already*
    // running (e.g. re-trigger) doesn't mis-fire a completion edge.
    step2.was_scanning = orchestrator.wizard_state.step2.is_scanning;
}

/// Snapshot every checked component on one tab as
/// `(tp2.to_ascii_uppercase(), component_id, selected_order)`.
fn capture_tab(mods: &[Step2ModState]) -> Vec<RescanSelection> {
    let mut out = Vec::new();
    for mod_state in mods {
        let tp2_upper = mod_state.tp_file.to_ascii_uppercase();
        for component in &mod_state.components {
            if component.checked {
                out.push(RescanSelection {
                    tp2_upper: tp2_upper.clone(),
                    component_id: component.component_id.clone(),
                    selected_order: component.selected_order,
                });
            }
        }
    }
    out
}

/// (b)+(c)+(d) Run on the scan-completion edge — called from
/// `OrchestratorApp::update` immediately after `poll_step2_channels`, so the
/// fresh mod set has already landed. No-op unless `is_scanning` just
/// transitioned `true → false`, the scan finished **successfully**
/// (`last_scan_report.is_some()`), and a snapshot is pending.
pub fn reconcile_on_scan_complete(orchestrator: &mut OrchestratorApp) {
    let scanning_now = orchestrator.wizard_state.step2.is_scanning;
    let was_scanning = orchestrator.workspace_view.step2.was_scanning;
    // Keep the edge detector current for the next frame regardless.
    orchestrator.workspace_view.step2.was_scanning = scanning_now;

    // Only the `true → false` edge is the completion moment.
    if !(was_scanning && !scanning_now) {
        return;
    }

    // A pending snapshot is required (no rescan in flight ⇒ nothing to do).
    let Some(snapshot) = orchestrator.workspace_view.step2.rescan_snapshot.take() else {
        return;
    };

    // Distinguish a *successful* completion (`Step2ScanEvent::Finished`,
    // which set `last_scan_report = Some(..)` and replaced the mod vectors)
    // from a `Canceled`/`Failed`/`Disconnected` terminal (which set
    // `last_scan_report = None` and did **not** replace the mods — so the
    // existing selection is still intact and there is nothing to
    // reconcile). On a non-success terminal we simply drop the snapshot.
    if orchestrator.wizard_state.step2.last_scan_report.is_none() {
        return;
    }

    // (c) Re-apply onto the fresh mods + recompute, per tab.
    let bgee_dropped = reapply_snapshot(
        &snapshot.bgee,
        &mut orchestrator.wizard_state.step2.bgee_mods,
    );
    let bg2ee_dropped = reapply_snapshot(
        &snapshot.bg2ee,
        &mut orchestrator.wizard_state.step2.bg2ee_mods,
    );
    recompute_mod_checked(&mut orchestrator.wizard_state.step2.bgee_mods);
    recompute_mod_checked(&mut orchestrator.wizard_state.step2.bg2ee_mods);

    // Keep `next_selection_order` ahead of the largest restored order so a
    // subsequent user check appends after the restored list (BIO's contract
    // for `selected_order`; same as `workspace_state_loader`).
    let max_order = max_selected_order(&orchestrator.wizard_state.step2.bgee_mods).max(
        max_selected_order(&orchestrator.wizard_state.step2.bg2ee_mods),
    );
    orchestrator.wizard_state.step2.next_selection_order = max_order + 1;

    // (c′) **Cold-resume only** (the #1 fix): rebuild Step 3 from the
    // re-marked Step-2 mods via BIO's canonical `step3_sync::build_step3_
    // items` — the exact recipe `workspace_state_loader::populate` uses on
    // workspace open. On a cold resume `populate` ran `build_step3_items`
    // on an *empty* Step-2 set (the scan hadn't run yet), so Step 3 is
    // empty; now that the snapshot has re-marked the freshly-scanned mods,
    // Step 3 must be rebuilt. This is gated on `resume_pending` so a dev
    // *rescan* (`resume_pending == false`) keeps BIO's clobber-protection —
    // it deliberately does **not** rebuild Step 3, preserving the user's
    // Step-3 reorder (SPEC §6.3). The Step-3 group-collapse vectors set by
    // `populate` survive (`build_step3_items` only builds `*_items`).
    if std::mem::take(&mut orchestrator.workspace_view.step2.resume_pending) {
        orchestrator.wizard_state.step3.bgee_items =
            step3_sync::build_step3_items(&orchestrator.wizard_state.step2.bgee_mods);
        orchestrator.wizard_state.step3.bg2ee_items =
            step3_sync::build_step3_items(&orchestrator.wizard_state.step2.bg2ee_mods);
    }

    // (d) Drop / missing-mods accounting across both tabs.
    let mut dropped: Vec<&RescanSelection> = Vec::new();
    dropped.extend(bgee_dropped.iter());
    dropped.extend(bg2ee_dropped.iter());
    if dropped.is_empty() {
        orchestrator.workspace_view.step2.rescan_drop_warning = None;
        return;
    }
    let dropped_components = dropped.len();
    let missing_mods = distinct_tp2_count(&dropped);
    // SPEC §6.3 wording, verbatim.
    orchestrator.workspace_view.step2.rescan_drop_warning = Some(format!(
        "{dropped_components} component(s) dropped \u{2014} {missing_mods} mod(s) no longer present"
    ));
}

/// Re-apply one tab's snapshot onto its freshly-scanned mods: for each
/// snapshot entry, find the matching scanned component by
/// `tp_file`(upper) + `component_id`, set `checked = true` and restore
/// `selected_order`. Returns the snapshot entries with **no** match (the
/// dropped selections — their mod or component is no longer present).
///
/// The fresh mods come straight from BIO's scan-event handler with
/// `checked = false` / `selected_order = None` everywhere (BIO's `Finished`
/// handler builds them fresh), so this only needs to *set* matches — it
/// does not pre-clear (unlike the workspace loader, which clears first
/// because it reuses an already-populated state across a modlist swap).
fn reapply_snapshot<'a>(
    snapshot: &'a [RescanSelection],
    mods: &mut [Step2ModState],
) -> Vec<&'a RescanSelection> {
    let mut dropped = Vec::new();
    for entry in snapshot {
        let mut matched = false;
        for mod_state in mods.iter_mut() {
            if mod_state.tp_file.to_ascii_uppercase() != entry.tp2_upper {
                continue;
            }
            for component in &mut mod_state.components {
                if component.component_id == entry.component_id {
                    component.checked = true;
                    component.selected_order = entry.selected_order;
                    matched = true;
                }
            }
        }
        if !matched {
            dropped.push(entry);
        }
    }
    dropped
}

/// Re-derive each mod's tri-state `checked`: a mod is `checked` iff at least
/// one of its components is checked. Mirrors
/// `workspace_state_loader::recompute_mod_checked` (BIO exposes no public
/// per-mod tri-state recompute; this is the established precedent for these
/// fields).
fn recompute_mod_checked(mods: &mut [Step2ModState]) {
    for mod_state in mods.iter_mut() {
        mod_state.checked = mod_state.components.iter().any(|c| c.checked);
    }
}

/// Largest `selected_order` across all components (0 if none). Mirrors
/// `workspace_state_loader::max_selected_order`.
fn max_selected_order(mods: &[Step2ModState]) -> usize {
    mods.iter()
        .flat_map(|m| m.components.iter())
        .filter_map(|c| c.selected_order)
        .max()
        .unwrap_or(0)
}

/// Distinct `tp2` count among the dropped entries — the "M mod(s) no longer
/// present" figure (SPEC §6.3).
fn distinct_tp2_count(dropped: &[&RescanSelection]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    for d in dropped {
        seen.insert(d.tp2_upper.as_str());
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{Step2ComponentState, Step2ModState};

    fn comp(id: &str, checked: bool, order: Option<usize>) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: String::new(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: String::new(),
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
            checked,
            selected_order: order,
        }
    }

    fn mod_state(tp: &str, comps: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: tp.to_string(),
            tp_file: tp.to_string(),
            tp2_path: String::new(),
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

    #[test]
    fn capture_only_grabs_checked_components_upper_tp2() {
        let mods = vec![mod_state(
            "EeFixPack.tp2",
            vec![comp("0", true, Some(2)), comp("5", false, None)],
        )];
        let snap = capture_tab(&mods);
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].tp2_upper, "EEFIXPACK.TP2");
        assert_eq!(snap[0].component_id, "0");
        assert_eq!(snap[0].selected_order, Some(2));
    }

    #[test]
    fn reapply_restores_checked_and_order_and_reports_drops() {
        // Snapshot had EEFIXPACK#0 (order 2) + GONEMOD#1 (order 1).
        let snapshot = vec![
            RescanSelection {
                tp2_upper: "EEFIXPACK.TP2".to_string(),
                component_id: "0".to_string(),
                selected_order: Some(2),
            },
            RescanSelection {
                tp2_upper: "GONEMOD.TP2".to_string(),
                component_id: "1".to_string(),
                selected_order: Some(1),
            },
        ];
        // Fresh scan: only EEFIXPACK is present (GONEMOD removed); its
        // components arrive unchecked, as BIO's Finished handler builds them.
        let mut mods = vec![mod_state(
            "eefixpack.tp2",
            vec![comp("0", false, None), comp("5", false, None)],
        )];
        let dropped = reapply_snapshot(&snapshot, &mut mods);
        // EEFIXPACK#0 re-checked, order preserved.
        assert!(mods[0].components[0].checked);
        assert_eq!(mods[0].components[0].selected_order, Some(2));
        // #5 untouched.
        assert!(!mods[0].components[1].checked);
        // GONEMOD#1 had no match ⇒ reported dropped.
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].component_id, "1");
        assert_eq!(distinct_tp2_count(&dropped), 1);
    }

    #[test]
    fn recompute_mod_checked_is_tri_state_any() {
        let mut mods = vec![
            mod_state("a.tp2", vec![comp("0", true, Some(1))]),
            mod_state("b.tp2", vec![comp("0", false, None)]),
        ];
        recompute_mod_checked(&mut mods);
        assert!(mods[0].checked);
        assert!(!mods[1].checked);
    }

    #[test]
    fn max_selected_order_tracks_largest() {
        let mods = vec![mod_state(
            "a.tp2",
            vec![comp("0", true, Some(3)), comp("1", true, Some(7))],
        )];
        assert_eq!(max_selected_order(&mods), 7);
    }
}
