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

pub(crate) struct PollBeforeRenderContext<'a> {
    pub(crate) state: &'a mut WizardState,
    pub(crate) step2_scan_rx: &'a mut Option<Receiver<Step2ScanEvent>>,
    pub(crate) step2_cancel: &'a mut Option<Arc<AtomicBool>>,
    pub(crate) step2_progress_queue: &'a mut VecDeque<(usize, usize, String)>,
    pub(crate) step2_update_check_rx:
        &'a mut Option<Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>>,
    pub(crate) step2_update_download_rx:
        &'a mut Option<Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>>,
    pub(crate) step2_update_extract_rx:
        &'a mut Option<Receiver<super::app_step2_update_extract::Step2UpdateExtractEvent>>,
    pub(crate) step5_terminal: &'a mut Option<EmbeddedTerminal>,
    pub(crate) step5_terminal_error: &'a mut Option<String>,
    pub(crate) step5_prep_rx: &'a mut Option<Receiver<Result<TargetPrepResult, String>>>,
    pub(crate) step5_pending_start: &'a mut Option<PendingInstallStart>,
}

pub(crate) fn poll_before_render(ctx: &mut PollBeforeRenderContext<'_>) -> bool {
    super::app_step2_scan::poll_step2_scan_events(
        ctx.state,
        ctx.step2_scan_rx,
        ctx.step2_cancel,
        ctx.step2_progress_queue,
    );
    super::app_step2_update_check::poll_step2_update_check(ctx.state, ctx.step2_update_check_rx);
    super::app_step2_update_download::poll_step2_update_download(
        ctx.state,
        ctx.step2_update_download_rx,
        ctx.step2_update_extract_rx,
    );
    super::app_step2_update_extract::poll_step2_update_extract(
        ctx.state,
        ctx.step2_update_extract_rx,
        ctx.step2_scan_rx,
        ctx.step2_cancel,
        ctx.step2_progress_queue,
    );
    super::app_step2_saved_log_flow::advance_pending_saved_log_flow(
        ctx.state,
        ctx.step2_scan_rx,
        ctx.step2_cancel,
        ctx.step2_progress_queue,
        ctx.step2_update_check_rx,
        ctx.step2_update_download_rx,
    );

    let mut step5_requested_repaint = false;
    step5_requested_repaint |= super::app_step5_flow::poll_step5_terminal(
        ctx.state,
        ctx.step5_terminal,
        ctx.step5_terminal_error,
    );
    step5_requested_repaint |= super::app_step5_flow::poll_step5_prep(
        ctx.state,
        ctx.step5_prep_rx,
        ctx.step5_terminal,
        ctx.step5_terminal_error,
        ctx.step5_pending_start,
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

#[derive(Clone, Copy)]
pub(crate) struct RepaintContext<'a> {
    pub(crate) step1_github_auth_rx:
        Option<&'a Receiver<super::app_step1_github_oauth::GitHubOAuthFlowResult>>,
    pub(crate) step2_scan_rx: Option<&'a Receiver<Step2ScanEvent>>,
    pub(crate) step2_progress_queue: &'a VecDeque<(usize, usize, String)>,
    pub(crate) step2_update_check_rx:
        Option<&'a Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>>,
    pub(crate) step2_update_download_rx:
        Option<&'a Receiver<super::app_step2_update_download::Step2UpdateDownloadEvent>>,
    pub(crate) step2_update_extract_rx:
        Option<&'a Receiver<super::app_step2_update_extract::Step2UpdateExtractEvent>>,
    pub(crate) step5_terminal: Option<&'a EmbeddedTerminal>,
    pub(crate) step5_prep_rx: Option<&'a Receiver<Result<TargetPrepResult, String>>>,
    pub(crate) state: &'a WizardState,
}

pub(crate) fn needs_repaint(ctx: RepaintContext<'_>) -> bool {
    ctx.step1_github_auth_rx.is_some()
        || ctx.step2_scan_rx.is_some()
        || ctx.step2_update_check_rx.is_some()
        || ctx.step2_update_download_rx.is_some()
        || ctx.step2_update_extract_rx.is_some()
        || !ctx.step2_progress_queue.is_empty()
        || ctx
            .step5_terminal
            .is_some_and(EmbeddedTerminal::has_new_data)
        || ctx.step5_prep_rx.is_some()
        || ctx.state.step5.prep_running
        || ctx.state.step5.install_running
        || ctx.state.modlist_auto_build_active
}
