// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::state::{ResumeTargets, Step5State};
use crate::app::terminal::EmbeddedTerminal;

pub(crate) fn process_graceful_cancel(step5: &mut Step5State, term: &mut EmbeddedTerminal) {
    if !(step5.install_running && step5.cancel_pending) {
        return;
    }

    if let Some(start) = step5.cancel_pending_output_len
        && start > term.output_len()
    {
        step5.cancel_pending_output_len = Some(0);
    }
    let boundary_counter = term.boundary_event_count();
    let last_seen = step5
        .cancel_pending_boundary_count
        .unwrap_or(boundary_counter);
    let boundary = boundary_counter > last_seen;
    if boundary {
        step5.cancel_pending_boundary_count = Some(boundary_counter);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let can_retry = step5
            .cancel_signal_sent_unix_secs
            .is_none_or(|last| now.saturating_sub(last) >= 1);
        if can_retry {
            term.graceful_terminate();
            step5.cancel_was_graceful = true;
            step5.last_cancel_mode = "graceful".to_string();
            step5.cancel_signal_sent_unix_secs = Some(now);
            step5.cancel_attempt_count = step5.cancel_attempt_count.saturating_add(1);
            step5.cancel_pending_output_len = Some(term.output_len());
            step5.cancel_pending_boundary_count = Some(boundary_counter);
            step5.last_status_text = format!(
                "Graceful cancel signal sent at SUCCESSFULLY INSTALLED (attempt #{})",
                step5.cancel_attempt_count
            );
        }
    } else {
        step5.last_status_text = if step5.cancel_attempt_count == 0 {
            "Graceful pending: waiting for SUCCESSFULLY INSTALLED boundary".to_string()
        } else {
            format!(
                "Graceful pending: waiting next boundary (attempts={})",
                step5.cancel_attempt_count
            )
        };
    }
}

pub(crate) fn process_exit_event(step5: &mut Step5State, term: &mut EmbeddedTerminal) {
    if !term.take_exit_event() {
        return;
    }

    let finished_exit = term.take_exit_code();
    if let Some(start) = step5.install_started_unix_secs.take() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        step5.last_runtime_secs = Some(now.saturating_sub(start));
    }
    step5.last_install_failed = term.likely_failure_visible();
    step5.last_exit_code = finished_exit;
    step5.install_running = false;
    step5.cancel_requested = false;
    step5.cancel_pending = false;
    step5.cancel_pending_started_unix_secs = None;
    step5.cancel_pending_output_len = None;
    step5.cancel_pending_boundary_count = None;
    step5.cancel_signal_sent_unix_secs = None;
    step5.cancel_attempt_count = 0;
    step5.resume_available = step5.cancel_was_graceful;
    if !step5.resume_available {
        step5.resume_targets = ResumeTargets::default();
    }
    step5.cancel_was_graceful = false;
    step5.last_scripted_skip_signature = None;
    step5.prompt_ready_signature = None;
    step5.prompt_ready_seen_count = 0;
    step5.prompt_ready_first_seen_unix_ms = None;
    step5.prompt_required_sound_latched = false;
    if let Some(run_id) = step5.active_run_id.take() {
        let suffix = finished_exit.map_or_else(String::new, |code| format!(" (exit {code})"));
        term.append_marker(&format!("Run #{run_id} finished{suffix}"));
    }
    step5.last_status_text = if step5.last_install_failed {
        step5.last_exit_code.map_or_else(
            || "Install failed".to_string(),
            |code| format!("Install failed (exit {code})"),
        )
    } else {
        step5.last_exit_code.map_or_else(
            || "Install finished".to_string(),
            |code| {
                if code == 0 {
                    "Install finished (exit 0)".to_string()
                } else {
                    format!("Install finished (exit {code})")
                }
            },
        )
    };
}
