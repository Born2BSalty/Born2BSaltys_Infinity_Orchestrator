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
        Receiver<super::app_step2_update_download::Step2UpdateDownloadResult>,
    >,
) {
    if state.step2.is_scanning || step2_scan_rx.is_some() {
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
        state.step2.pending_saved_log_download = false;
        super::app_step2_update_download::start_step2_update_download(
            state,
            step2_update_download_rx,
        );
    }
}

fn clear_pending(state: &mut WizardState) {
    state.step2.pending_saved_log_apply = false;
    state.step2.pending_saved_log_update_preview = false;
    state.step2.pending_saved_log_download = false;
}

fn scan_failed(state: &WizardState) -> bool {
    state.step2.scan_status.starts_with("Scan failed:")
        || state.step2.scan_status == "Scan canceled"
        || state.step2.scan_status == "Scan worker disconnected"
}
