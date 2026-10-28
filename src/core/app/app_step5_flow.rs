// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app::state::WizardState;
use crate::app::step5::basic_install_report;
use crate::app::step5::install_flow::{
    PendingInstallStart, apply_completed_prep, prepare_start_request, spawn_target_prep_worker,
    start_install_process,
};
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::step5::log_files::begin_new_run;
use crate::app::terminal::EmbeddedTerminal;

pub(crate) fn ensure_step5_terminal(
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
) {
    if step5_terminal.is_some() || step5_terminal_error.is_some() {
        return;
    }
    match EmbeddedTerminal::new() {
        Ok(term) => {
            *step5_terminal = Some(term);
        }
        Err(err) => {
            *step5_terminal_error = Some(err.to_string());
        }
    }
}

pub(crate) fn poll_step5_prep(
    state: &mut WizardState,
    step5_prep_rx: &mut Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    let Some(rx) = step5_prep_rx.as_ref() else {
        return false;
    };

    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(Err("Target prep worker disconnected".to_string())),
    };
    let Some(result) = result else {
        return false;
    };

    *step5_prep_rx = None;
    state.step5.prep_running = false;
    ensure_step5_terminal(step5_terminal, step5_terminal_error);
    let Some(term) = step5_terminal.as_mut() else {
        *step5_pending_start = None;
        state.step5.last_status_text =
            "Target prep finished, but terminal is unavailable".to_string();
        basic_install_report::update(state, "start_failed", None);
        return false;
    };

    if apply_completed_prep(state, term, result) {
        if let Some(pending) = step5_pending_start.take() {
            start_install_process(state, term, pending);
        }
    } else {
        *step5_pending_start = None;
    }
    true
}

pub(crate) fn start_if_requested(
    state: &mut WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_prep_rx: &mut Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    if !state.step5.start_install_requested || state.step5.prep_running {
        return false;
    }

    reset_attempt_report_metadata(state);
    ensure_step5_terminal(step5_terminal, step5_terminal_error);
    let Some(term) = step5_terminal.as_mut() else {
        state.step5.start_install_requested = false;
        state.step5.last_status_text = "Install start failed: terminal unavailable".to_string();
        let run_id = begin_new_run(&mut state.step5);
        basic_install_report::begin(state, &run_id);
        basic_install_report::update(state, "start_failed", None);
        return false;
    };

    let run_id = begin_new_run(&mut state.step5);
    basic_install_report::begin(state, &run_id);

    let Some((pending, needs_prep)) = prepare_start_request(state, term) else {
        return false;
    };

    if needs_prep {
        state.step5.prep_running = true;
        state.step5.last_status_text = "Target prep in progress...".to_string();
        term.append_marker("Target prep started");
        *step5_pending_start = Some(pending);
        *step5_prep_rx = Some(spawn_target_prep_worker(state.step1.clone()));
    } else {
        start_install_process(state, term, pending);
    }
    true
}

fn reset_attempt_report_metadata(state: &mut WizardState) {
    state.step5.install_choice = state.pending_install_choice.clone().unwrap_or_else(|| {
        if state.step5.has_run_once {
            state.step5.install_choice.clone()
        } else {
            "Not selected from gallery".to_string()
        }
    });
    state.step5.install_source = state.pending_install_source.clone().unwrap_or_else(|| {
        if state.step5.has_run_once {
            state.step5.install_source.clone()
        } else {
            "Workspace / manual start".to_string()
        }
    });
    state.step5.last_start_mode = "not started".to_string();
    state.step5.last_status_text = "Starting".to_string();
    state.step5.last_exit_code = None;
    state.step5.last_runtime_secs = None;
    state.step5.install_started_unix_secs = None;
    state.step5.last_cancel_mode = "none".to_string();
    state.step5.last_install_failed = false;
}

pub(crate) fn poll_step5_terminal(
    state: &mut WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
) -> bool {
    ensure_step5_terminal(step5_terminal, step5_terminal_error);
    if let Some(term) = step5_terminal.as_mut() {
        term.poll_output();
        let needs_repaint = term.has_new_data();
        crate::app::step5_runtime_status::process_graceful_cancel(&mut state.step5, term);
        crate::app::step5_runtime_status::process_exit_event(state, term);
        basic_install_report::poll(state, Some(term));
        needs_repaint
    } else {
        basic_install_report::poll(state, None);
        false
    }
}
