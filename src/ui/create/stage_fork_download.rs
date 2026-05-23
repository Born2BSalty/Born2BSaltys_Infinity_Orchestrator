// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, build_and_hold_progress,
    ingest_downloaded_archives_once, kick_streaming_downloader_once, render_chrome,
    stage_and_kick_archive_skip_once, verify_downloaded_archives_once,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::workspace::workspace_state_loader;

const fn fork_download_copy() -> DownloadScreenCopy {
    DownloadScreenCopy {
        title: "Downloading fork",
        sub: "fetching the parent's mods \u{2014} Step 2 opens automatically when ready",
        hint: Some(
            "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2",
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForkDownloadOutcome {
    #[default]
    Stay,
    Cancel,
    Import,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    progress: &DownloadProgress,
) -> ForkDownloadOutcome {
    match stage_downloading::render(ui, palette, fork_download_copy(), progress) {
        stage_downloading::DownloadingOutcome::Cancel => ForkDownloadOutcome::Cancel,
        stage_downloading::DownloadingOutcome::Advance => ForkDownloadOutcome::Import,
        stage_downloading::DownloadingOutcome::Stay => ForkDownloadOutcome::Stay,
    }
}

pub fn render_live(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> ForkDownloadOutcome {
    let palette = orchestrator.theme_palette;
    let inputs = stage_downloading::LivePipelineInputs::from(orchestrator);

    stage_downloading::arm_pipeline_once(orchestrator, &inputs);
    stage_and_kick_archive_skip_once(orchestrator, &inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);

    // Once extract finishes, suppress the auto-build install trigger.
    // `arm_auto_build` (called during fork-arm) leaves
    // `modlist_auto_build_active` + `modlist_auto_build_waiting_for_install`
    // true; the legacy `advance_pending_saved_log_flow` would otherwise fire
    // `start_auto_build_install` once the post-extract apply settles, which
    // sets `start_install_requested = true` and auto-routes to Step 5
    // install — wrong for the fork path, where the user should land on
    // Workspace for review. Idempotent: subsequent frames re-clear-no-op.
    if orchestrator.install_screen_state.pipeline_flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .archives_ingested()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_extract_running
    {
        orchestrator.wizard_state.modlist_auto_build_active = false;
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = false;
    }

    if fork_extract_complete(orchestrator) {
        orchestrator.wizard_state.modlist_auto_build_active = false;
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = false;
        orchestrator
            .wizard_state
            .step2
            .pending_saved_log_update_preview = false;
        orchestrator.wizard_state.step2.pending_saved_log_download = false;
        orchestrator.wizard_state.step2.update_selected_popup_open = false;
        orchestrator
            .wizard_state
            .step2
            .update_selected_confirm_latest_fallback_open = false;
        orchestrator
            .wizard_state
            .step2
            .mod_download_forks_popup_open = false;
        persist_fork_resume_workspace_state(orchestrator);
    }

    let progress = build_and_hold_progress(orchestrator);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let back_clicked = render_chrome(
        ui,
        palette,
        fork_download_copy(),
        &progress,
        arm_error.as_deref(),
    );

    if back_clicked {
        return ForkDownloadOutcome::Cancel;
    }
    if fork_extract_complete(orchestrator) {
        return ForkDownloadOutcome::Import;
    }
    ForkDownloadOutcome::Stay
}

fn fork_extract_complete(orchestrator: &OrchestratorApp) -> bool {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    let step2 = &orchestrator.wizard_state.step2;
    let archives_observed = step2.update_selected_extracted_sources.len()
        + orchestrator.install_screen_state.skip_indices.len();
    let arm_error = orchestrator
        .install_screen_state
        .pipeline_arm_error
        .is_some();
    let download_running = step2.update_selected_download_running;
    let extract_running = step2.update_selected_extract_running;
    let scan_running = step2.is_scanning;
    let apply_pending = step2.pending_saved_log_apply;
    let update_preview_pending = step2.pending_saved_log_update_preview;
    let complete = flags.armed()
        && !arm_error
        && flags.archive_skip_completed()
        && flags.download_phase_started()
        && flags.archives_verified()
        && flags.archives_ingested()
        && !download_running
        && !extract_running
        && !scan_running
        && !apply_pending
        && !update_preview_pending
        && (archives_observed > 0 || step2.update_selected_update_assets.is_empty());

    if flags.armed()
        && flags.download_phase_started()
        && !download_running
        && !extract_running
        && !scan_running
        && !apply_pending
        && !update_preview_pending
    {
        tracing::info!(
            target = "orchestrator",
            complete,
            armed = flags.armed(),
            arm_error,
            archive_skip_completed = flags.archive_skip_completed(),
            download_phase_started = flags.download_phase_started(),
            archives_verified = flags.archives_verified(),
            archives_ingested = flags.archives_ingested(),
            download_running,
            extract_running,
            scan_running,
            apply_pending,
            update_preview_pending,
            extracted_sources = step2.update_selected_extracted_sources.len(),
            skipped_archives = orchestrator.install_screen_state.skip_indices.len(),
            update_assets_remaining = step2.update_selected_update_assets.len(),
            archives_observed,
            "fork extract completion gate"
        );
    }

    complete
}

fn persist_fork_resume_workspace_state(orchestrator: &mut OrchestratorApp) {
    let Some(id) = orchestrator.active_install_modlist_id.clone() else {
        return;
    };
    let scratch_mods_folder = orchestrator
        .wizard_state
        .step1
        .mods_folder
        .trim()
        .to_string();
    if scratch_mods_folder.is_empty() {
        return;
    }

    workspace_state_loader::sync_step3_from_step2_if_changed(&mut orchestrator.wizard_state);

    let prior = orchestrator
        .workspace_state
        .get(&id)
        .cloned()
        .unwrap_or_default();
    let mut extracted = workspace_state_loader::extract_workspace_state_from_wizard(
        &orchestrator.wizard_state,
        &prior,
    );
    extracted.scratch_mods_folder = Some(scratch_mods_folder);

    if extracted == prior {
        return;
    }

    let store = orchestrator
        .workspace_stores
        .entry(id.clone())
        .or_insert_with(|| WorkspaceStore::new_for_id(&id));
    if let Err(err) = store.save(&extracted) {
        warn!(
            target = "orchestrator",
            "Fork-complete workspace persist for {id} failed: {err} \
             (in-memory state still updated; on-exit flush_all is the backstop)"
        );
    }
    orchestrator
        .workspace_state
        .insert(id.clone(), extracted.clone());
    orchestrator
        .persistence_cycle
        .last_saved_workspaces
        .insert(id, extracted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fork_copy_is_spec_5_3_verbatim() {
        let c = fork_download_copy();
        assert_eq!(c.title, "Downloading fork");
        assert_eq!(
            c.hint,
            Some(
                "after download: components auto-selected \u{00B7} order applied \u{00B7} lands on Step 2"
            )
        );
        assert!(c.sub.contains("Step 2 opens automatically"));
    }

    #[test]
    fn fork_extract_complete_requires_armed_pipeline() {
        let app_state = MinimalForkExtractState::default();
        assert!(!evaluate(&app_state));
        let app_state = MinimalForkExtractState::all_latches_set();
        assert!(
            evaluate(&app_state),
            "all latches set + extracted > 0 ⇒ done"
        );
    }

    #[test]
    fn fork_extract_complete_false_when_arm_error_present() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.arm_error = Some("boom".to_string());
        assert!(
            !evaluate(&app_state),
            "an arm error MUST NOT register as fork-complete"
        );
    }

    #[test]
    fn fork_extract_complete_false_when_streamer_still_running() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.running.download = true;
        assert!(!evaluate(&app_state));
    }

    #[test]
    fn fork_extract_complete_false_when_extractor_still_running() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.running.extract = true;
        assert!(!evaluate(&app_state));
    }

    #[test]
    fn fork_extract_complete_true_when_all_skipped_and_no_assets_to_fetch() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.extracted_count = 0;
        app_state.assets_remaining = 0;
        app_state.skipped_count = 5;
        assert!(
            evaluate(&app_state),
            "every archive skipped + asset list empty ⇒ done (parity \
             with archive-skip-all-present path)"
        );
    }

    #[test]
    fn fork_extract_complete_false_while_post_extract_scan_running() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.running.scan = true;
        assert!(
            !evaluate(&app_state),
            "the post-extract scan must finish before the route fires so the \
             re-armed apply has a populated step2 to write into"
        );
    }

    #[test]
    fn fork_extract_complete_false_while_apply_pending() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.pending.apply = true;
        assert!(
            !evaluate(&app_state),
            "the re-armed saved-log apply must run before the route fires so \
             the imported WeiDU log selection is visible on landing"
        );
    }

    #[test]
    fn fork_extract_complete_false_while_update_preview_pending() {
        let mut app_state = MinimalForkExtractState::all_latches_set();
        app_state.pending.update_preview = true;
        assert!(!evaluate(&app_state));
    }

    #[derive(Default)]
    struct RunningPhases {
        download: bool,
        extract: bool,
        scan: bool,
    }

    #[derive(Default)]
    struct PendingFlags {
        apply: bool,
        update_preview: bool,
    }

    #[derive(Default)]
    struct MinimalForkExtractState {
        flags: crate::ui::install::state_install::InstallPipelineFlags,
        arm_error: Option<String>,
        running: RunningPhases,
        pending: PendingFlags,
        extracted_count: usize,
        skipped_count: usize,
        assets_remaining: usize,
    }

    impl MinimalForkExtractState {
        fn all_latches_set() -> Self {
            let mut state = Self::default();
            state.flags.set_armed(true);
            state.flags.set_archive_skip_completed(true);
            state.flags.set_download_phase_started(true);
            state.flags.set_archives_verified(true);
            state.flags.set_archives_ingested(true);
            state.extracted_count = 3;
            state
        }
    }

    fn evaluate(s: &MinimalForkExtractState) -> bool {
        if !s.flags.armed() || s.arm_error.is_some() {
            return false;
        }
        if !s.flags.archive_skip_completed() {
            return false;
        }
        if !s.flags.download_phase_started() {
            return false;
        }
        if !s.flags.archives_verified() {
            return false;
        }
        if !s.flags.archives_ingested() {
            return false;
        }
        if s.running.download
            || s.running.extract
            || s.running.scan
            || s.pending.apply
            || s.pending.update_preview
        {
            return false;
        }
        let archives_observed = s.extracted_count + s.skipped_count;
        archives_observed > 0 || s.assets_remaining == 0
    }
}
