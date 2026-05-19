// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::auto_build_driver` — Phase 7 P7.T17 piece 3
// (SPEC §13.12a "Pipeline-reuse contract").
//
// Drives BIO's existing **import → auto-build** pipeline for share-code-
// consuming workflows (Install Modlist paste / Create-import / Load-Draft)
// **without modifying any BIO source**. The pipeline itself is BIO's:
//
//   `import_modlist_share_code` (writes the code's baked-in `weidu.log` +
//   `mod_downloads_user.toml` + pinned `installed_refs`)
//     → the saved-log / auto-build flow (`app_step2_saved_log_flow::
//       advance_pending_saved_log_flow`): scan → apply the log →
//       update-check (resolve each mod from the imported source config
//       into a concrete download asset) → download → extract → rescan
//       → install (`start_auto_build_install` flips `current_step = 4`
//       **and** `start_install_requested = true` ITSELF).
//
// The orchestrator already ticks `advance_pending_saved_log_flow` **every
// frame** via `OrchestratorApp::poll_step2_channels` (P6.T2c — the exact
// narrower mirror of `app_update_cycle::poll_before_render`'s Step-2
// portion, with the orchestrator-owned 6 receivers) and already ticks the
// Step-5 runtime via `poll_step5_before_render` / `start_step5_after_
// render` (P7.T1). So this module only has to **arm** the pipeline; the
// existing per-frame poll runs it to completion exactly as `WizardApp`
// does. No new channel infrastructure, no re-architecture.
//
// **Arming = BIO's own `start_modlist_auto_build` recipe, minus the
// install flip.** BIO's `src/ui/step1/page_step1.rs::start_modlist_auto_
// build` (the canonical reference) sets: `modlist_auto_build_active =
// true`, `current_step = 1`, the BGEE/BG2EE active tab from the imported
// game, `pending_saved_log_apply` + `pending_saved_log_update_preview` +
// `pending_saved_log_download` = `true`, and the status text. It does
// **not** set `start_install_requested` — the pipeline's own
// `start_auto_build_install` does that only after download/extract/scan.
// Mirroring it exactly (not pre-flipping `start_install_requested`) is
// what makes the install start *after* the archives are staged, not
// before (verified: `app_step5_flow::start_if_requested` gates only on
// `start_install_requested && !prep_running` — a premature flip would
// install an empty per-install Mods folder).
//
// **The content-addressed staging layer** (`archive_store`) interposes at
// the download/extract boundary purely on the orchestrator side — see
// `live_progress::tick_pipeline`, which calls `archive_store::
// stage_known_archives` before BIO's download fires and `ingest_
// downloaded_archives` after it lands. This module's job is the import +
// arm; `live_progress` owns the per-frame boundary interposition + the
// §4.3 screen feed.
//
// **Per-install dirs are derived here too** (the trigger point per plan
// P7.T3 step 2b / the `start_hooks` P7.T17 placeholder): a fresh
// Create → New skips the import but STILL gets the per-install dirs
// (SPEC §13.12a — clone is forced for every install). So
// `prepare_install_dirs_and_maybe_import` always derives the dirs and
// only runs the import for a share-code-consuming workflow.
//
// SPEC: §13.12a (pipeline-reuse contract + per-install dirs), §13.12 #5.

use tracing::warn;

use crate::app::modlist_share::import_modlist_share_code;
use crate::app::state::WizardState;
use crate::install_runtime::flag_policies::InstallWorkflow;
use crate::install_runtime::per_install_dirs::{self, PerInstallDirs};
use crate::registry::model::Game;

/// Outcome of the prep step — what the trigger site (the `start_hooks`
/// P7.T17 placeholder) and the live Downloading screen need to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepOutcome {
    /// A **share-code-consuming** workflow: per-install dirs derived AND
    /// BIO's import → auto-build pipeline armed. The caller must **NOT**
    /// flip `start_install_requested` — the auto-build flow does that
    /// itself after download/extract/scan (the existing per-frame poll
    /// drives it). The Downloading screen ticks the boundary interposition
    /// + feeds the §4.3 grid.
    PipelineArmed { dirs: PerInstallDirs },
    /// A **fresh Create → New** (no share code): per-install dirs derived,
    /// no import, no auto-build. The caller flips `start_install_requested`
    /// normally (BIO scans the per-install Mods folder the user populated)
    /// — i.e. today's `page_workspace_step5` behavior is correct for this
    /// arm.
    DirsOnly { dirs: PerInstallDirs },
}

/// **Trigger point (plan P7.T3 step 2b / the `start_hooks` P7.T17
/// placeholder).** Derive the per-install directories inside `destination`
/// + force the clone flags (always — SPEC §13.12a); then, for a
/// share-code-consuming workflow, run `import_modlist_share_code` and arm
/// BIO's auto-build pipeline.
///
/// `share_code` is the code to import for a share-code-consuming workflow
/// (the pasted Install-Modlist code / the Create-import / Load-Draft
/// code). It is ignored for `FreshCreate` (no import).
///
/// Returns `Err` if the per-install dirs cannot be created (install-
/// critical — the caller must surface it and not start the install) or
/// the import fails (a bad/incompatible share code — the pipeline cannot
/// run). On `Ok` the existing per-frame poll runs the rest.
///
/// **Borrow note.** `per_install_dirs::derive_per_install_dirs` mutates
/// `wizard_state.step1`; it runs *before* `import_modlist_share_code`
/// (which clones+rewrites `state.step1` from the payload then
/// `reset_workflow_keep_step1`). Order matters: import must NOT clobber
/// the derived per-install targets. BIO's `import_modlist_share_code`
/// only overwrites `game_install` / `install_mode` / the flag bits and
/// `reset_workflow_keep_step1` — it preserves the rest of `step1`
/// (including `mods_folder` / `eet_pre_dir` / `generate_directory` /
/// `weidu_log_folder`), so deriving first is correct. (`game_install`
/// from the payload is authoritative for an import — that is intended;
/// the dirs were derived for the registry entry's game, which for a
/// share-code-consuming modlist equals the payload's game by
/// construction.)
pub fn prepare_install_dirs_and_maybe_import(
    wizard_state: &mut WizardState,
    destination: &str,
    game: Game,
    workflow: InstallWorkflow,
    share_code: &str,
) -> Result<PrepOutcome, String> {
    // ── 1. Per-install dirs — ALWAYS (SPEC §13.12a: clone forced for
    //    every install; a fresh Create → New still gets these). Runs
    //    BEFORE import so import's `reset_workflow_keep_step1` keeps the
    //    derived targets. ──
    let dirs =
        per_install_dirs::derive_per_install_dirs(&mut wizard_state.step1, destination, game)
            .map_err(|err| format!("per-install directory derivation failed: {err}"))?;

    // ── 2. Import + arm — only for a share-code-consuming workflow. ──
    if !is_share_code_consuming(workflow) {
        return Ok(PrepOutcome::DirsOnly { dirs });
    }
    if share_code.trim().is_empty() {
        return Err(
            "share-code-consuming workflow has no share code to import (SPEC §13.12a)".to_string(),
        );
    }

    // BIO's canonical importer — writes the code's baked-in `weidu.log` +
    // `mod_downloads_user.toml` + pinned `installed_refs`, rewrites
    // `state.step1` game/mode from the payload, and
    // `reset_workflow_keep_step1` (preserving the per-install targets
    // derived in step 1). Reused **unchanged** (`pub(crate)`, same-crate
    // reachable per the carve-out-#3 lib+bin split — the SAME reachability
    // `poll_step2_channels` already uses). NEVER patched.
    import_modlist_share_code(wizard_state, share_code.trim())
        .map_err(|err| format!("import_modlist_share_code failed: {err}"))?;

    // Arm BIO's auto-build pipeline — BIO's own `start_modlist_auto_build`
    // recipe, MINUS `start_install_requested` (the pipeline's
    // `start_auto_build_install` sets that itself, only after
    // download/extract/scan).
    arm_auto_build(wizard_state);

    Ok(PrepOutcome::PipelineArmed { dirs })
}

/// `true` for the workflows SPEC §13.12 #5 lists as share-code-consuming
/// (they import a code + drive the auto-build download pipeline).
/// `FreshCreate` is the only non-importing arm.
#[must_use]
pub fn is_share_code_consuming(workflow: InstallWorkflow) -> bool {
    match workflow {
        InstallWorkflow::ShareCodeConsuming
        | InstallWorkflow::ContinuePartialInstall
        | InstallWorkflow::Reinstall => true,
        InstallWorkflow::FreshCreate => false,
    }
}

/// **Arm the download-archive policy for the Install-Modlist-paste /
/// Reinstall pipeline path (the §13.12a always-content-addressed-stage
/// model + §13.12 #5 `--download`-forced-ON).**
///
/// BIO's `app_step2_update_download::start_step2_update_download`
/// early-returns unless **both** `step1.download_archive == true` **and**
/// `step1.mods_archive_folder` is non-empty (BIO defaults: `false` / `""`).
/// The Install-Modlist-paste / Reinstall pipeline reaches the download tick
/// via `stage_downloading::render_live` → this module — it **never** runs
/// the workspace `on_install_start` (the sole `flag_policies::apply_flags`
/// caller, which only sets `step1.download`, never the archive fields) and
/// **never** runs the workspace-open `sync_paths_from_settings` (which
/// copies `mods_archive_folder` but never `download_archive`). So on this
/// path all three are unset and BIO's downloader silently no-ops ("the
/// downloading never starts"). This sets them so BIO's reused-unchanged
/// downloader runs:
///
///   - `download_archive = true` — the orchestrator's SPEC §13.12a
///     model: the global Mods-archive stage is **always** used (the
///     content-addressed staging layer interposes unconditionally — "all
///     downloaded mod archives for all modlists always land here, never
///     per-install"). It is **not** a user toggle in the redesign; it is
///     the fixed staging model, so it is set unconditionally here (NOT
///     gated on a Settings field).
///   - `mods_archive_folder = <global Mods-archive folder>` — the value
///     **must** be sourced by the caller exactly as
///     `sync_paths_from_settings` reads it (Settings → Paths, the
///     `Step1Settings → Step1State` conversion's `mods_archive_folder`).
///     Passed in as `mods_archive_folder` so this fn stays
///     `WizardState`-only + unit-testable (no `SettingsStore` coupling) —
///     the caller mirrors the mapping, this fn never invents a path.
///     `archive_store::{stage_known_archives,…}` + BIO's downloader both
///     read this exact field, so they stay consistent.
///   - `download = true` — SPEC §13.12 #5: `--download` is forced ON for
///     share-code-consuming workflows. `flag_policies::apply_flags`
///     already sets this on the workspace path, but it is **not** invoked
///     on this pipeline path (verified: `apply_flags`'s only call site is
///     `start_hooks::on_install_start`), so it is set here too.
///
/// Idempotent + side-effect-free beyond these three `step1` fields — safe
/// to call inside the one-shot `pipeline_armed` latch (the caller does).
/// Runs *after* `prepare_install_dirs_and_maybe_import`'s import so it is
/// the final word before the per-frame poll's first
/// `advance_pending_saved_log_flow` download tick (and survives
/// `import_modlist_share_code`, which clones `step1`, mutates only
/// game/mode, and `reset_workflow_keep_step1` — it never touches these
/// three; verified — but ordering it last removes any doubt).
///
/// SPEC: §13.12a (always-content-addressed Mods-archive stage), §13.12 #5
/// (`--download` forced ON for share-code-consuming workflows).
pub fn arm_download_archive_policy(state: &mut WizardState, mods_archive_folder: &str) {
    // SPEC §13.12a — the Mods-archive stage is always used (not a toggle).
    state.step1.download_archive = true;
    // The global Mods-archive folder, sourced by the caller exactly as
    // `sync_paths_from_settings` does (Settings → Paths). Trimmed to match
    // BIO's own `start_step2_update_download` emptiness check + the
    // `archive_store` reads (both `.trim()` this field).
    state.step1.mods_archive_folder = mods_archive_folder.trim().to_string();
    // SPEC §13.12 #5 — `--download` forced ON for share-code-consuming
    // workflows (the pipeline path does not run `apply_flags`).
    state.step1.download = true;
}

/// Arm BIO's saved-log / auto-build pipeline — the **exact**
/// `bio::ui::step1::page_step1::start_modlist_auto_build` field set
/// (`page_step1.rs:250-265`) **minus** `start_install_requested` (the
/// pipeline's `app_step2_saved_log_flow::start_auto_build_install` flips
/// that itself after download/extract/scan — pre-flipping it would make
/// `app_step5_flow::start_if_requested` install an empty Mods folder).
///
/// The active game tab is derived from the (post-import) `step1.
/// game_install` exactly as BIO does (BGEE ⇒ "BGEE"; everything else ⇒
/// "BG2EE" — EET's BG1 phase runs under the BG2EE-side flow in BIO's
/// model, matching `page_step1.rs:254-258`). After this, the
/// orchestrator's existing per-frame `poll_step2_channels`
/// (`advance_pending_saved_log_flow`) runs scan → apply-log →
/// update-preview → download → extract → rescan → `start_auto_build_
/// install`, and `start_step5_after_render` then kicks the install —
/// identical to BIO's Step-1 import path.
fn arm_auto_build(state: &mut WizardState) {
    state.modlist_auto_build_active = true;
    state.modlist_auto_build_waiting_for_install = false;
    state.current_step = 1;
    state.step2.active_game_tab = if state.step1.game_install == "BGEE" {
        "BGEE".to_string()
    } else {
        "BG2EE".to_string()
    };
    state.step2.pending_saved_log_apply = true;
    state.step2.pending_saved_log_update_preview = true;
    state.step2.pending_saved_log_download = true;
    // BIO opens the update-selected popup here so its progress is visible
    // in the legacy wizard. The redesign renders the §4.3 Downloading
    // screen instead, so DO NOT open BIO's popup (it would float over the
    // redesign chrome). The pipeline does not depend on the popup being
    // open — `advance_pending_saved_log_flow` is popup-agnostic
    // (`app_step2_saved_log_flow.rs` reads only the `pending_*` flags +
    // `modlist_auto_build_active`). Verified: `start_auto_build_install`
    // *clears* `update_selected_popup_open`, so leaving it false is the
    // pipeline's own post-state anyway.
    state.step2.scan_status = "Auto Build: preparing imported modlist".to_string();
    state.step5.last_status_text = "Auto Build: preparing imported modlist".to_string();
}

/// `true` once BIO's auto-build pipeline has finished and handed off to
/// the install runtime: `start_auto_build_install` clears
/// `modlist_auto_build_active` + `modlist_auto_build_waiting_for_install`
/// and sets `current_step = 4` + `start_install_requested = true`. The
/// live Downloading screen uses this as the "advance to the stage-4 seam"
/// signal (SPEC §4.3 → §4.4). Also true if the pipeline stopped via
/// `stop_auto_build` (which clears `modlist_auto_build_active` + the
/// pending flags) — the screen surfaces the BIO status text in that case
/// rather than silently advancing (a stopped pipeline did not reach
/// install).
#[must_use]
pub fn pipeline_finished(state: &WizardState) -> bool {
    !state.modlist_auto_build_active
        && !state.step2.pending_saved_log_apply
        && !state.step2.pending_saved_log_update_preview
        && !state.step2.pending_saved_log_download
}

/// `true` once the pipeline reached the install hand-off specifically
/// (`start_auto_build_install` set `current_step = 4` +
/// `start_install_requested`/`install_running`). Distinguishes a
/// successful completion (advance to stage 4) from a `stop_auto_build`
/// abort (stay, show the error).
#[must_use]
pub fn pipeline_reached_install(state: &WizardState) -> bool {
    pipeline_finished(state)
        && (state.step5.start_install_requested
            || state.step5.install_running
            || state.current_step == 4)
}

/// Best-effort log of a stopped pipeline (the BIO status text already
/// carries the user-facing reason via `stop_auto_build`). Surfaced so a
/// dev-mode log shows why the §4.3 screen did not advance.
pub fn log_if_pipeline_stopped(state: &WizardState) {
    if pipeline_finished(state) && !pipeline_reached_install(state) {
        warn!(
            target = "orchestrator",
            "auto-build pipeline did not reach install: {}", state.step2.scan_status
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn td() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_auto_build_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn fresh_create_derives_dirs_no_import_no_arm() {
        // SPEC §13.12a: a fresh Create → New still gets per-install dirs
        // but no import / no auto-build (the caller flips
        // start_install_requested normally for this arm).
        let dest = td();
        let dest_s = dest.to_string_lossy().into_owned();
        let mut st = WizardState::default();
        let out = prepare_install_dirs_and_maybe_import(
            &mut st,
            &dest_s,
            Game::BGEE,
            InstallWorkflow::FreshCreate,
            "", // ignored
        )
        .expect("fresh-create prep");
        assert!(matches!(out, PrepOutcome::DirsOnly { .. }));
        assert!(
            !st.modlist_auto_build_active,
            "fresh-create must NOT arm the auto-build pipeline"
        );
        assert!(
            !st.step5.start_install_requested,
            "prep never flips start_install_requested itself"
        );
        // Dirs were still derived (clone forced — SPEC §13.12a).
        assert!(st.step1.generate_directory_enabled);
        assert!(!st.step1.mods_folder.is_empty());
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn share_code_consuming_without_code_is_error() {
        let dest = td();
        let mut st = WizardState::default();
        let r = prepare_install_dirs_and_maybe_import(
            &mut st,
            &dest.to_string_lossy(),
            Game::EET,
            InstallWorkflow::ShareCodeConsuming,
            "   ",
        );
        assert!(r.is_err(), "share-code-consuming needs a code");
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn arm_auto_build_matches_bio_recipe_minus_install_flip() {
        // The exact BIO `start_modlist_auto_build` field set
        // (`page_step1.rs:250-265`) MINUS `start_install_requested`.
        let mut st = WizardState::default();
        st.step1.game_install = "EET".to_string();
        arm_auto_build(&mut st);
        assert!(st.modlist_auto_build_active);
        assert!(!st.modlist_auto_build_waiting_for_install);
        assert_eq!(st.current_step, 1);
        assert_eq!(
            st.step2.active_game_tab,
            "BGEE".to_string().replace("BGEE", "BG2EE")
        ); // EET ⇒ BG2EE
        assert!(st.step2.pending_saved_log_apply);
        assert!(st.step2.pending_saved_log_update_preview);
        assert!(st.step2.pending_saved_log_download);
        assert!(
            !st.step5.start_install_requested,
            "arm must NOT pre-flip start_install_requested — the pipeline's \
             start_auto_build_install does that after download/extract (a \
             premature flip installs an empty Mods folder)"
        );

        // BGEE maps to the BGEE tab (the other BIO branch).
        let mut b = WizardState::default();
        b.step1.game_install = "BGEE".to_string();
        arm_auto_build(&mut b);
        assert_eq!(b.step2.active_game_tab, "BGEE");
    }

    #[test]
    fn pipeline_finished_and_reached_install_predicates() {
        let mut st = WizardState::default();
        // Armed (pending flags set) ⇒ not finished.
        arm_auto_build(&mut st);
        assert!(!pipeline_finished(&st));
        assert!(!pipeline_reached_install(&st));

        // Pipeline cleared the flags + reached the install hand-off
        // (what `start_auto_build_install` does).
        st.modlist_auto_build_active = false;
        st.step2.pending_saved_log_apply = false;
        st.step2.pending_saved_log_update_preview = false;
        st.step2.pending_saved_log_download = false;
        st.current_step = 4;
        st.step5.start_install_requested = true;
        assert!(pipeline_finished(&st));
        assert!(pipeline_reached_install(&st));

        // Stopped (flags cleared by `stop_auto_build`) but never reached
        // install ⇒ finished but not reached-install (screen stays + shows
        // the error, does not advance).
        let mut stopped = WizardState::default();
        arm_auto_build(&mut stopped);
        stopped.modlist_auto_build_active = false;
        stopped.step2.pending_saved_log_apply = false;
        stopped.step2.pending_saved_log_update_preview = false;
        stopped.step2.pending_saved_log_download = false;
        stopped.step2.scan_status =
            "Auto Build stopped: local path/tool preflight failed".to_string();
        assert!(pipeline_finished(&stopped));
        assert!(
            !pipeline_reached_install(&stopped),
            "a stopped pipeline must NOT count as reached-install"
        );
    }

    #[test]
    fn is_share_code_consuming_matches_spec_13_12_5() {
        assert!(is_share_code_consuming(InstallWorkflow::ShareCodeConsuming));
        assert!(is_share_code_consuming(
            InstallWorkflow::ContinuePartialInstall
        ));
        assert!(is_share_code_consuming(InstallWorkflow::Reinstall));
        assert!(!is_share_code_consuming(InstallWorkflow::FreshCreate));
    }

    // ───────────── FIX 1 — arm_download_archive_policy ─────────────
    //
    // The Install-Modlist-paste / Reinstall pipeline path's
    // download-archive arming. In-memory `WizardState` only (no store, no
    // `%APPDATA%\bio` — DATA-LOSS-safe by construction). Proves the three
    // `step1` fields BIO's `start_step2_update_download` guards on are set
    // (`download_archive`/`mods_archive_folder` empty by default would make
    // the downloader silently no-op — "downloading never starts").

    #[test]
    fn arm_download_archive_policy_sets_the_three_step1_fields() {
        // BIO `Step1State::default()`: `download_archive == false` AND
        // `mods_archive_folder == ""` — these two are the EXACT guards
        // `start_step2_update_download` early-returns on (so the pipeline
        // download silently never starts). (`download` defaults `true` in
        // BIO's `Step1State::default()`, verified — it is NOT one of the
        // download-blocking guards; FIX 1 still re-asserts it ON per SPEC
        // §13.12 #5 since the pipeline path skips `apply_flags`, and that
        // assertion is idempotent here.)
        let mut st = WizardState::default();
        assert!(
            !st.step1.download_archive,
            "precondition: download_archive defaults false (a download guard)"
        );
        assert!(
            st.step1.mods_archive_folder.is_empty(),
            "precondition: mods_archive_folder defaults empty (a download guard)"
        );

        arm_download_archive_policy(&mut st, r"D:\BG\ModsArchive");

        // SPEC §13.12a: the Mods-archive stage is always used (unconditional
        // — not a user toggle).
        assert!(
            st.step1.download_archive,
            "download_archive forced ON (SPEC §13.12a always-content-addressed stage)"
        );
        // Sourced value lands verbatim (the caller mirrors
        // sync_paths_from_settings; this fn never invents it).
        assert_eq!(
            st.step1.mods_archive_folder, r"D:\BG\ModsArchive",
            "mods_archive_folder = the Settings → Paths value"
        );
        // SPEC §13.12 #5: --download forced ON for share-code-consuming.
        assert!(
            st.step1.download,
            "download forced ON (SPEC §13.12 #5 — pipeline path skips apply_flags)"
        );
    }

    #[test]
    fn arm_download_archive_policy_trims_archive_folder() {
        // BIO's own `start_step2_update_download` + `archive_store` both
        // `.trim()` this field — the arming must store the trimmed value so
        // a whitespace-padded Settings path does not defeat BIO's emptiness
        // guard (or worse, a whitespace-only path passing it).
        let mut st = WizardState::default();
        arm_download_archive_policy(&mut st, "  C:\\Mods Archive  ");
        assert_eq!(
            st.step1.mods_archive_folder, "C:\\Mods Archive",
            "leading/trailing whitespace trimmed (matches BIO's trim check)"
        );

        // A whitespace-only Settings value ⇒ stored empty ⇒ BIO's
        // downloader still honestly no-ops ("Mods Archive folder is empty")
        // rather than trying to download into a junk path. (FIX 1 makes the
        // download START when a real folder is configured; an unconfigured
        // one is still a no-op with BIO's own status — not a regression.)
        let mut st2 = WizardState::default();
        arm_download_archive_policy(&mut st2, "   ");
        assert!(
            st2.step1.mods_archive_folder.is_empty(),
            "whitespace-only ⇒ empty (BIO's own empty-archive guard still applies)"
        );
        // download_archive / download are still forced ON regardless — the
        // §13.12a / §13.12 #5 policy is not contingent on the path being
        // configured (BIO's own guard handles the empty-path case).
        assert!(st2.step1.download_archive);
        assert!(st2.step1.download);
    }

    #[test]
    fn arm_download_archive_policy_survives_reset_workflow_keep_step1() {
        // FIX 1 relies on `import_modlist_share_code` preserving these
        // `step1` fields: it does `step1 = state.step1.clone()`, mutates
        // ONLY game/mode, writes it back, then `reset_workflow_keep_step1()`
        // (which never touches `self.step1`). This pins that invariant so a
        // future BIO change to the import/reset path that clobbered the
        // archive fields would fail here (FIX 1 sets the policy AFTER the
        // import as the final word, but this proves the order is also safe
        // the other way — defense in depth, zero BIO edit).
        let mut st = WizardState::default();
        arm_download_archive_policy(&mut st, r"D:\BG\ModsArchive");
        // Simulate BIO's import_modlist_share_code step1 handling.
        let cloned = st.step1.clone();
        st.step1 = cloned; // (game/mode mutation omitted — irrelevant here)
        st.reset_workflow_keep_step1();
        assert!(
            st.step1.download_archive,
            "download_archive survives the clone + reset_workflow_keep_step1"
        );
        assert_eq!(
            st.step1.mods_archive_folder, r"D:\BG\ModsArchive",
            "mods_archive_folder survives the import path's step1 handling"
        );
        assert!(
            st.step1.download,
            "download survives reset_workflow_keep_step1 (it keeps step1)"
        );
    }
}
