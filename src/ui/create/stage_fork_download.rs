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

    if fork_extract_complete(orchestrator) {
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
    let complete = flags.armed()
        && !arm_error
        && flags.archive_skip_completed()
        && flags.download_phase_started()
        && flags.archives_verified()
        && flags.archives_ingested()
        && !download_running
        && !extract_running
        && (archives_observed > 0 || step2.update_selected_update_assets.is_empty());

    if flags.armed() && flags.download_phase_started() && !download_running && !extract_running {
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
            extracted_sources = step2.update_selected_extracted_sources.len(),
            skipped_archives = orchestrator.install_screen_state.skip_indices.len(),
            update_assets_remaining = step2.update_selected_update_assets.len(),
            archives_observed,
            "fork extract completion gate"
        );
    }

    complete
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
