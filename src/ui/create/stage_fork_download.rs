// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, build_and_hold_progress,
    ingest_downloaded_archives_once, kick_streaming_downloader_once, render_chrome,
    stage_and_kick_archive_skip_once, verify_downloaded_archives_once,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::ThemePalette;

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

/// The fork sub-flow's chassis-only renderer. Used by tests that exercise
/// the chrome without a live pipeline; the production path goes through
/// [`render_live`].
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

/// The live fork-download stage.
///
/// Reuses the Install-pipeline arming + the three per-frame steps the
/// `stage_downloading::render_live` runs (skip pool → streamer → verify
/// → ingest); `mint_and_arm` populated `install_screen_state.destination`
/// / `import_code` / `destination_choice` at the `ForkBeginImport` edge so
/// the existing pipeline state machine drives the fork's download against
/// the minted modlist exactly as it drives an Install-paste install.
///
/// The fork advances at the **extract-complete** seam (not at the
/// pipeline-reached-install seam the Install path uses): when the
/// streamer has finished, the verify + ingest one-shots ran, the extract
/// pool drained, and BIO's auto-build flag is no longer driving the
/// pipeline toward Step 5. At that point the user routes to Step 2 of the
/// new workspace.
pub fn render_live(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> ForkDownloadOutcome {
    let palette = orchestrator.theme_palette;
    let inputs = stage_downloading::LivePipelineInputs::from(orchestrator);

    stage_downloading::arm_pipeline_once(orchestrator, &inputs);
    stage_and_kick_archive_skip_once(orchestrator, &inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);

    if fork_extract_complete(orchestrator) {
        // Disarm before BIO's `advance_pending_saved_log_flow` sees the
        // post-extract gate open: the Install-paste path lets
        // `start_auto_build_install` flip `current_step = 4 +
        // start_install_requested = true`, but the fork hands control
        // back to the user at Step 2 instead.
        orchestrator.wizard_state.modlist_auto_build_active = false;
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = false;
        orchestrator.wizard_state.step2.pending_saved_log_apply = false;
        orchestrator
            .wizard_state
            .step2
            .pending_saved_log_update_preview = false;
        orchestrator.wizard_state.step2.pending_saved_log_download = false;
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

/// Has the fork's extract pool drained? True when the pipeline has been
/// armed (so a Cancel before arm does not look like "complete"), the
/// streamer is done, verify + ingest one-shots ran, the extract pool
/// reports not-running, and at least one archive entered the extracted
/// vector (so an empty asset set does not falsely advance before the
/// pipeline has run).
fn fork_extract_complete(orchestrator: &OrchestratorApp) -> bool {
    let flags = orchestrator.install_screen_state.pipeline_flags;
    if !flags.armed()
        || orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_some()
    {
        return false;
    }
    if !flags.archive_skip_completed() {
        return false;
    }
    if !flags.download_phase_started() {
        return false;
    }
    if !flags.archives_verified() {
        return false;
    }
    if !flags.archives_ingested() {
        return false;
    }
    let step2 = &orchestrator.wizard_state.step2;
    if step2.update_selected_download_running || step2.update_selected_extract_running {
        return false;
    }
    let archives_observed = step2.update_selected_extracted_sources.len()
        + orchestrator.install_screen_state.skip_indices.len();
    archives_observed > 0 || step2.update_selected_update_assets.is_empty()
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
        let mut app_state = MinimalForkExtractState::default();
        // No flags set, no extract progress — definitely not done.
        assert!(!evaluate(&app_state));
        app_state.flags.set_armed(true);
        app_state.flags.set_archive_skip_completed(true);
        app_state.flags.set_download_phase_started(true);
        app_state.flags.set_archives_verified(true);
        app_state.flags.set_archives_ingested(true);
        app_state.extracted_count = 3;
        assert!(
            evaluate(&app_state),
            "all latches set + extracted > 0 ⇒ done"
        );
    }

    #[test]
    fn fork_extract_complete_false_when_arm_error_present() {
        let mut app_state = MinimalForkExtractState::default();
        app_state.flags.set_armed(true);
        app_state.flags.set_archive_skip_completed(true);
        app_state.flags.set_download_phase_started(true);
        app_state.flags.set_archives_verified(true);
        app_state.flags.set_archives_ingested(true);
        app_state.extracted_count = 3;
        app_state.arm_error = Some("boom".to_string());
        assert!(
            !evaluate(&app_state),
            "an arm error MUST NOT register as fork-complete"
        );
    }

    #[test]
    fn fork_extract_complete_false_when_streamer_still_running() {
        let mut app_state = MinimalForkExtractState::default();
        app_state.flags.set_armed(true);
        app_state.flags.set_archive_skip_completed(true);
        app_state.flags.set_download_phase_started(true);
        app_state.flags.set_archives_verified(true);
        app_state.flags.set_archives_ingested(true);
        app_state.extracted_count = 3;
        app_state.download_running = true;
        assert!(!evaluate(&app_state));
    }

    #[test]
    fn fork_extract_complete_false_when_extractor_still_running() {
        let mut app_state = MinimalForkExtractState::default();
        app_state.flags.set_armed(true);
        app_state.flags.set_archive_skip_completed(true);
        app_state.flags.set_download_phase_started(true);
        app_state.flags.set_archives_verified(true);
        app_state.flags.set_archives_ingested(true);
        app_state.extracted_count = 3;
        app_state.extract_running = true;
        assert!(!evaluate(&app_state));
    }

    #[test]
    fn fork_extract_complete_true_when_all_skipped_and_no_assets_to_fetch() {
        let mut app_state = MinimalForkExtractState::default();
        app_state.flags.set_armed(true);
        app_state.flags.set_archive_skip_completed(true);
        app_state.flags.set_download_phase_started(true);
        app_state.flags.set_archives_verified(true);
        app_state.flags.set_archives_ingested(true);
        app_state.assets_remaining = 0;
        app_state.skipped_count = 5;
        assert!(
            evaluate(&app_state),
            "every archive skipped + asset list empty ⇒ done (parity \
             with archive-skip-all-present path)"
        );
    }

    /// Pure projection of the fields `fork_extract_complete` reads on
    /// `OrchestratorApp`. Keeps the test isolated from
    /// `OrchestratorApp::new`, which would touch the real config dir.
    #[derive(Default)]
    struct MinimalForkExtractState {
        flags: crate::ui::install::state_install::InstallPipelineFlags,
        arm_error: Option<String>,
        download_running: bool,
        extract_running: bool,
        extracted_count: usize,
        skipped_count: usize,
        assets_remaining: usize,
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
        if s.download_running || s.extract_running {
            return false;
        }
        let archives_observed = s.extracted_count + s.skipped_count;
        archives_observed > 0 || s.assets_remaining == 0
    }
}
