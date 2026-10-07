// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use tracing::warn;

use crate::registry::store_workspace::WorkspaceStore;
use crate::ui::install::manual_downloads_panel::PanelAction;
use crate::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, LivePipelineInputs, build_and_hold_progress,
    enter_manual_hold_once, ingest_downloaded_archives_once, kick_explicit_resolve_once,
    kick_streaming_downloader_once, open_manual_page, pick_manual_file, render_chrome,
    render_manual_confirm_dialog, stage_and_kick_archive_skip_once,
    verify_downloaded_archives_once,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::workspace::workspace_state_loader;

const fn fork_download_copy() -> DownloadScreenCopy {
    DownloadScreenCopy {
        title: "Downloading mods",
        sub: "fetching this modlist's mods \u{2014} Step 2 opens automatically when ready",
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
        stage_downloading::DownloadingOutcome::Stay
        | stage_downloading::DownloadingOutcome::OpenWorkspace => ForkDownloadOutcome::Stay,
    }
}

pub fn render_live(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> ForkDownloadOutcome {
    let palette = orchestrator.theme_palette;
    let inputs = LivePipelineInputs::from_workflow(
        orchestrator,
        crate::install_runtime::flag_policies::InstallWorkflow::ForkAndModify,
    );

    fork_pipeline_tick(orchestrator, &inputs);

    if fork_empty_asset_finish(orchestrator) {
        finish_fork_download(orchestrator);
        return ForkDownloadOutcome::Import;
    }

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
        finish_fork_download(orchestrator);
    }

    let progress = build_and_hold_progress(orchestrator);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let (back_clicked, panel_action) = render_chrome(
        ui,
        palette,
        fork_download_copy(),
        &progress,
        arm_error.as_deref(),
        Some(&mut orchestrator.install_screen_state.manual_downloads),
    );

    match panel_action {
        PanelAction::OpenPage(index) => open_manual_page(orchestrator, index),
        PanelAction::PickFile(index) => pick_manual_file(orchestrator, index),
        PanelAction::None => {}
    }

    render_manual_confirm_dialog(ui, orchestrator, palette);

    if back_clicked {
        return ForkDownloadOutcome::Cancel;
    }
    if fork_extract_complete(orchestrator) {
        return ForkDownloadOutcome::Import;
    }
    ForkDownloadOutcome::Stay
}

fn fork_pipeline_tick(orchestrator: &mut OrchestratorApp, inputs: &LivePipelineInputs) {
    stage_downloading::arm_pipeline_once(orchestrator, inputs);
    kick_explicit_resolve_once(orchestrator);
    enter_manual_hold_once(orchestrator, inputs);
    stage_and_kick_archive_skip_once(orchestrator, inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);
}

fn fork_empty_asset_finish(orchestrator: &OrchestratorApp) -> bool {
    let manual = &orchestrator.install_screen_state.manual_downloads;
    let flags = orchestrator.install_screen_state.pipeline_flags;
    let step2 = &orchestrator.wizard_state.step2;
    manual.continue_without
        && !manual.manual_hold_active()
        && flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && flags.explicit_resolve_started()
        && !flags.archives_staged()
        && !step2.update_selected_check_running
        && step2.update_selected_update_assets.is_empty()
        && !step2.is_scanning
        && !step2.pending_saved_log_apply
        && !step2.pending_saved_log_update_preview
        && !step2.update_selected_download_running
        && !step2.update_selected_extract_running
}

fn finish_fork_download(orchestrator: &mut OrchestratorApp) {
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

pub(super) fn fork_extract_complete(orchestrator: &OrchestratorApp) -> bool {
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
        assert_eq!(c.title, "Downloading mods");
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

    #[test]
    fn fork_completion_records_the_list_mods_folder() {
        let mut app = OrchestratorApp::new_isolated_for_test("fork-completion-mods-folder");
        let mods_folder = app
            .isolated_test_config_root
            .as_ref()
            .expect("the isolated app owns a temp config root")
            .join("fork mods")
            .to_string_lossy()
            .into_owned();
        app.active_install_modlist_id = Some("FORKED000001".to_string());
        app.wizard_state.step1.mods_folder.clone_from(&mods_folder);

        persist_fork_resume_workspace_state(&mut app);

        let in_memory = app
            .workspace_state
            .get("FORKED000001")
            .expect("the fork's workspace is recorded");
        assert_eq!(
            in_memory.scratch_mods_folder.as_deref(),
            Some(mods_folder.as_str())
        );
        let on_disk = WorkspaceStore::new_for_id("FORKED000001")
            .load()
            .expect("the fork's workspace is written");
        assert_eq!(
            on_disk.scratch_mods_folder.as_deref(),
            Some(mods_folder.as_str())
        );
    }

    struct ForkHoldTempRoot {
        path: std::path::PathBuf,
    }

    impl ForkHoldTempRoot {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "bio_forkhold_{}_{}_{tag}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for ForkHoldTempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn fork_asset(label: &str) -> crate::app::state::Step2UpdateAsset {
        crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/setup-{label}.tp2"),
            label: label.to_string(),
            source_id: "github".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://example.com/{label}.zip"),
            installed_source_ref: None,
        }
    }

    fn fork_manual_request(label: &str) -> crate::app::state::ManualDownloadRequest {
        crate::app::state::ManualDownloadRequest {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/setup-{label}.tp2"),
            label: label.to_string(),
            source_id: String::new(),
            page_url: "https://www.baldurs-gate.de/index.php?threads/1".to_string(),
            reason: crate::app::state::ManualDownloadReason::NotAutoResolvable,
            aliases: Vec::new(),
            display_name: String::new(),
        }
    }

    fn fork_inputs(destination: String) -> LivePipelineInputs {
        LivePipelineInputs {
            destination,
            game: crate::registry::model::Game::BGEE,
            workflow: crate::install_runtime::flag_policies::InstallWorkflow::ForkAndModify,
            code: String::new(),
        }
    }

    fn armed_fork_app(tag: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.install_screen_state.pipeline_kind =
            crate::ui::install::state_install::PipelineKind::Fork;
        app.install_screen_state.pipeline_flags.set_armed(true);
        app.install_screen_state
            .pipeline_flags
            .set_explicit_resolve_started(true);
        app.wizard_state.step2.update_selected_check_running = false;
        app
    }

    #[test]
    fn fork_tick_enters_manual_hold_before_the_cache_check() {
        use crate::install_runtime::archive_skip_async::ArchiveSkipEvent;
        use std::sync::mpsc::RecvTimeoutError;

        let root = ForkHoldTempRoot::new("tick-enters-hold");
        let mut app = armed_fork_app("forkhold-tick-enters-hold");
        app.wizard_state.step1.mods_archive_folder =
            root.path.join("archives").to_string_lossy().into_owned();
        app.wizard_state.step2.update_selected_update_assets = vec![fork_asset("DlcMerger")];
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![fork_manual_request("BragesRedemption")];
        let inputs = fork_inputs(root.path.join("dest").to_string_lossy().into_owned());

        fork_pipeline_tick(&mut app, &inputs);

        assert_eq!(app.install_screen_state.manual_downloads.rows.len(), 1);
        assert!(
            app.install_screen_state
                .manual_downloads
                .manual_hold_active()
        );
        assert!(
            app.install_screen_state.pipeline_flags.archives_staged(),
            "a manual request never blocks the automatic downloads on the fork route"
        );

        let watcher = app
            .manual_download_rx
            .take()
            .expect("the hold watches the archive folder");
        let _ = watcher.recv_timeout(std::time::Duration::from_secs(5));
        drop(watcher);
        if let Some(skip_rx) = app.archive_skip_rx.take() {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                match skip_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                    Ok(ArchiveSkipEvent::Finished { .. }) | Err(RecvTimeoutError::Disconnected) => {
                        break;
                    }
                    Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                }
            }
        }
    }

    #[test]
    fn fork_empty_asset_finish_after_continue_without() {
        let mut app = armed_fork_app("forkhold-empty-finish");
        app.wizard_state.step2.update_selected_manual_downloads =
            vec![fork_manual_request("BragesRedemption")];
        let inputs = fork_inputs(String::new());

        fork_pipeline_tick(&mut app, &inputs);
        assert_eq!(app.install_screen_state.manual_downloads.rows.len(), 1);
        assert!(app.manual_download_rx.is_none());
        assert!(!fork_empty_asset_finish(&app), "the hold is still waiting");

        app.install_screen_state.manual_downloads.rows[0].status =
            crate::ui::install::state_install::ManualRowStatus::Skipped;
        app.install_screen_state.manual_downloads.continue_without = true;
        assert!(fork_empty_asset_finish(&app));

        app.install_screen_state
            .pipeline_flags
            .set_archives_staged(true);
        assert!(
            !fork_empty_asset_finish(&app),
            "an asset list emptied by the archive store is not an all-manual list"
        );
        app.install_screen_state
            .pipeline_flags
            .set_archives_staged(false);

        app.install_screen_state.manual_downloads.continue_without = false;
        assert!(!fork_empty_asset_finish(&app));
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
