// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::state::{Step1State, WizardState};
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::settings::store::SettingsStore;

use super::app_lifecycle;

#[allow(clippy::too_many_arguments)]
pub(crate) fn poll_before_render(
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
    step2_update_extract_rx: &mut Option<
        Receiver<super::app_step2_update_extract::Step2UpdateExtractResult>,
    >,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_prep_rx: &mut Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    super::app_step2_scan::poll_step2_scan_events(
        state,
        step2_scan_rx,
        step2_cancel,
        step2_progress_queue,
    );
    super::app_step2_update_check::poll_step2_update_check(state, step2_update_check_rx);
    super::app_step2_update_download::poll_step2_update_download(
        state,
        step2_update_download_rx,
        step2_update_extract_rx,
    );
    super::app_step2_update_extract::poll_step2_update_extract(
        state,
        step2_update_extract_rx,
        step2_scan_rx,
        step2_cancel,
        step2_progress_queue,
    );
    super::app_step2_saved_log_flow::advance_pending_saved_log_flow(
        state,
        step2_scan_rx,
        step2_cancel,
        step2_progress_queue,
        step2_update_check_rx,
        step2_update_download_rx,
    );

    let mut step5_requested_repaint = false;
    step5_requested_repaint |=
        super::app_step5_flow::poll_step5_terminal(state, step5_terminal, step5_terminal_error);
    step5_requested_repaint |= super::app_step5_flow::poll_step5_prep(
        state,
        step5_prep_rx,
        step5_terminal,
        step5_terminal_error,
        step5_pending_start,
    );
    step5_requested_repaint
}

pub(crate) fn start_after_render(
    state: &mut WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_prep_rx: &mut Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    super::app_step5_flow::start_if_requested(
        state,
        step5_terminal,
        step5_terminal_error,
        step5_prep_rx,
        step5_pending_start,
    )
}

pub(crate) fn persist_step1_if_needed(
    state: &WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    if app_lifecycle::should_save_settings(state, last_saved_step1) {
        app_lifecycle::save_settings_best_effort(
            state,
            settings_store,
            last_saved_step1,
            dev_mode,
            exe_fingerprint,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn needs_repaint(
    step1_github_auth_rx: &Option<Receiver<super::app_step1_github_oauth::GitHubOAuthFlowResult>>,
    step2_scan_rx: &Option<Receiver<Step2ScanEvent>>,
    step2_progress_queue: &VecDeque<(usize, usize, String)>,
    step2_update_check_rx: &Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    step2_update_download_rx: &Option<
        Receiver<super::app_step2_update_download::Step2UpdateDownloadResult>,
    >,
    step2_update_extract_rx: &Option<
        Receiver<super::app_step2_update_extract::Step2UpdateExtractResult>,
    >,
    step5_terminal: &Option<EmbeddedTerminal>,
    step5_prep_rx: &Option<Receiver<Result<TargetPrepResult, String>>>,
    state: &WizardState,
) -> bool {
    step1_github_auth_rx.is_some()
        || step2_scan_rx.is_some()
        || step2_update_check_rx.is_some()
        || step2_update_download_rx.is_some()
        || step2_update_extract_rx.is_some()
        || !step2_progress_queue.is_empty()
        || step5_terminal
            .as_ref()
            .map(|term| term.has_new_data())
            .unwrap_or(false)
        || step5_prep_rx.is_some()
        || state.step5.prep_running
        || state.step5.install_running
}
