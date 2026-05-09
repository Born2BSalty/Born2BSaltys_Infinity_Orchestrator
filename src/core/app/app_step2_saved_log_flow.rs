// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;

pub(crate) fn queue_exact_log_update_preview(
    state: &mut WizardState,
    active_game_tab: &str,
    auto_download: bool,
) {
    state.step2.active_game_tab = active_game_tab.to_string();
    state.step2.pending_saved_log_apply = true;
    state.step2.pending_saved_log_update_preview = true;
    state.step2.pending_saved_log_download = auto_download;
    state.step2.scan_status = if auto_download {
        "Preparing missing mod download...".to_string()
    } else {
        "Preparing missing mod check...".to_string()
    };
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn advance_pending_saved_log_flow(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    step2_update_download_rx: &mut Option<
        Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>,
    >,
) {
    if state.step2.is_scanning || step2_scan_rx.is_some() {
        return;
    }

    let preflight_blocker = if state.modlist_auto_build_active
        && (state.step2.pending_saved_log_apply || state.step2.pending_saved_log_update_preview)
    {
        auto_build_preflight_blocker(state)
    } else {
        None
    };
    if let Some(reason) = preflight_blocker {
        stop_auto_build(state, reason);
        return;
    }

    if state.step2.pending_saved_log_apply || state.step2.pending_saved_log_update_preview {
        if scan_failed(state) {
            clear_pending(state);
            return;
        }
        if state.step2.last_scan_report.is_none() {
            super::app_step2_scan::start_step2_scan(
                state,
                step2_scan_rx,
                step2_cancel,
                step2_progress_queue,
            );
            return;
        }
        if state.step2.pending_saved_log_apply {
            state.step2.pending_saved_log_apply = false;
            super::app_step2_log::apply_saved_weidu_log_selection(state);
        }
        if state.step2.pending_saved_log_update_preview {
            state.step2.pending_saved_log_update_preview = false;
            let loaded = crate::app::mod_downloads::load_mod_download_sources();
            super::app_step2_update_preview::preview_update_selected(
                state,
                step2_update_check_rx,
                &loaded,
            );
        }
    }

    if state.step2.pending_saved_log_download
        && !state.step2.pending_saved_log_apply
        && !state.step2.pending_saved_log_update_preview
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
    {
        if state.modlist_auto_build_active {
            if let Some(reason) = auto_build_blocker_before_download(state) {
                stop_auto_build(state, reason);
                return;
            }
            if state.step2.update_selected_update_assets.is_empty() {
                state.step2.pending_saved_log_download = false;
                start_auto_build_install(state);
                return;
            }
            state.modlist_auto_build_waiting_for_install = true;
        }
        state.step2.pending_saved_log_download = false;
        super::app_step2_update_download::start_step2_update_download(
            state,
            step2_update_download_rx,
        );
    }

    if state.modlist_auto_build_active
        && state.modlist_auto_build_waiting_for_install
        && !state.step2.is_scanning
        && step2_scan_rx.is_none()
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
    {
        if scan_failed(state) {
            stop_auto_build(state, "scan failed after extraction".to_string());
            return;
        }
        if let Some(reason) = auto_build_blocker_before_install(state) {
            stop_auto_build(state, reason);
            return;
        }
        start_auto_build_install(state);
    }
}

fn clear_pending(state: &mut WizardState) {
    state.step2.pending_saved_log_apply = false;
    state.step2.pending_saved_log_update_preview = false;
    state.step2.pending_saved_log_download = false;
}

fn stop_auto_build(state: &mut WizardState, reason: String) {
    clear_pending(state);
    state.modlist_auto_build_active = false;
    state.modlist_auto_build_waiting_for_install = false;
    let message = format!("Auto Build stopped: {reason}");
    state.step2.scan_status = message.clone();
    state.step5.last_status_text = message;
}

fn start_auto_build_install(state: &mut WizardState) {
    state.modlist_auto_build_active = false;
    state.modlist_auto_build_waiting_for_install = false;
    state.step2.update_selected_popup_open = false;
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.mod_download_forks_popup_open = false;
    state.current_step = 4;
    state.step5.start_install_requested = true;
    state.step5.last_status_text = "Auto Build: starting install".to_string();
}

fn auto_build_blocker_before_download(state: &WizardState) -> Option<String> {
    if let Some(reason) = auto_build_preflight_blocker(state) {
        return Some(reason);
    }
    if !state
        .step2
        .update_selected_exact_version_failed_sources
        .is_empty()
        || !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
    {
        return Some("exact pinned version unavailable".to_string());
    }
    if let Some(source) = state.step2.update_selected_manual_sources.first() {
        return Some(format!("manual-only source required: {source}"));
    }
    if let Some(source) = state.step2.update_selected_unknown_sources.first() {
        return Some(format!("failed source resolution: {source}"));
    }
    if let Some(source) = state.step2.update_selected_failed_sources.first() {
        return Some(format!("failed source check: {source}"));
    }
    if state.step2.update_selected_update_assets.is_empty()
        && (!state.step2.update_selected_missing_sources.is_empty()
            || !state.step2.update_selected_update_sources.is_empty())
    {
        return Some("downloadable sources have no resolved archive".to_string());
    }
    None
}

fn auto_build_blocker_before_install(state: &WizardState) -> Option<String> {
    if let Some(reason) = auto_build_preflight_blocker(state) {
        return Some(reason);
    }
    if let Some(source) = state.step2.update_selected_download_failed_sources.first() {
        return Some(format!("failed download: {source}"));
    }
    if let Some(source) = state.step2.update_selected_extract_failed_sources.first() {
        return Some(format!("failed extraction/config restore: {source}"));
    }
    if !state.step2.update_selected_update_assets.is_empty()
        || !state.step2.update_selected_missing_sources.is_empty()
        || !state.step2.update_selected_update_sources.is_empty()
    {
        return Some("unresolved downloads remain".to_string());
    }
    None
}

fn auto_build_preflight_blocker(state: &WizardState) -> Option<String> {
    let (ok, message) = crate::app::state_validation::run_path_check(&state.step1);
    if ok {
        None
    } else {
        Some(format!("local path/tool preflight failed: {message}"))
    }
}

fn scan_failed(state: &WizardState) -> bool {
    state.step2.scan_status.starts_with("Scan failed:")
        || state.step2.scan_status == "Scan canceled"
        || state.step2.scan_status == "Scan worker disconnected"
}
