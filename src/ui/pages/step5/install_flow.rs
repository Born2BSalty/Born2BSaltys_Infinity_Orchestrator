// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::app::step5_flow::copy_weidu_logs_for_diagnostics;
use crate::ui::state::WizardState;
use crate::ui::step5::command::{
    build_install_invocation, build_resume_invocation,
};
use crate::ui::step5::log_files::{
    prepare_target_dirs_before_install, validate_resume_paths, validate_runtime_prep_paths,
    verify_targets_prepared,
};
use crate::ui::step5::scripted_inputs;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn start_if_requested(state: &mut WizardState, term: &mut EmbeddedTerminal) {
    if !state.step5.start_install_requested {
        return;
    }

    state.step5.start_install_requested = false;
    state.compat.show_pre_install_modal = false;

    term.configure_from_step1(&state.step1);
    let scripted = scripted_inputs::load_from_step1(&state.step1);
    let scripted_count = term.set_scripted_inputs(scripted);
    if scripted_count > 0 {
        term.append_marker(&format!("Loaded {scripted_count} @wlb-inputs token(s)"));
    }
    copy_weidu_logs_for_diagnostics(&state.step1);

    let resume_mode = state.step5.resume_available;
    let (program, args) = if resume_mode {
        build_resume_invocation(&state.step1)
    } else if state.step5.has_run_once && !state.step5.restart_program.trim().is_empty() {
        (state.step5.restart_program.clone(), state.step5.restart_args.clone())
    } else {
        build_install_invocation(&state.step1)
    };

    let runtime_ready = match if resume_mode {
        validate_resume_paths(&state.step1)
    } else {
        validate_runtime_prep_paths(&state.step1)
    } {
        Ok(()) => true,
        Err(err) => {
            state.step5.last_status_text = format!("Preflight check failed: {err}");
            term.append_marker(&format!("Preflight check failed: {err}"));
            false
        }
    };

    let can_start = if !runtime_ready {
        false
    } else if resume_mode {
        true
    } else {
        match prepare_target_dirs_before_install(&state.step1) {
            Ok(prep) => {
                for path in prep.backups {
                    state.step5.last_status_text = format!("Backed up target dir to {}", path.display());
                    term.append_marker(&format!("Backup created: {}", path.display()));
                }
                for path in prep.cleaned {
                    state.step5.last_status_text = format!("Cleaned target dir {}", path.display());
                    term.append_marker(&format!("Target cleaned: {}", path.display()));
                }
                match verify_targets_prepared(&state.step1) {
                    Ok(()) => true,
                    Err(err) => {
                        state.step5.last_status_text = format!("Target prep verify failed: {err}");
                        term.append_marker(&format!("Target prep verify failed: {err}"));
                        false
                    }
                }
            }
            Err(err) => {
                state.step5.last_status_text = format!("Backup target dirs failed: {err}");
                false
            }
        }
    };

    if !can_start {
        return;
    }

    match term.start_process(program.as_str(), &args) {
        Ok(()) => {
            state.step5.run_counter = state.step5.run_counter.saturating_add(1);
            state.step5.active_run_id = Some(state.step5.run_counter);
            term.append_marker(&format!("Run #{} started", state.step5.run_counter));
            term.focus();
            state.step5.hide_top_frames_after_install = true;
            state.step5.install_running = true;
            state.step5.last_install_failed = false;
            state.step5.last_exit_code = None;
            state.step5.last_status_text = "Running".to_string();
            state.step5.install_started_unix_secs = Some(now_unix_secs());
            state.step5.last_runtime_secs = None;
            state.step5.has_run_once = true;
            state.step5.cancel_confirm_open = false;
            state.step5.cancel_requested = false;
            state.step5.cancel_pending = false;
            state.step5.cancel_pending_started_unix_secs = None;
            state.step5.cancel_pending_output_len = None;
            state.step5.cancel_pending_boundary_count = None;
            state.step5.cancel_signal_sent_unix_secs = None;
            state.step5.cancel_attempt_count = 0;
            state.step5.cancel_force_checked = false;
            state.step5.cancel_was_graceful = false;
            state.step5.resume_available = false;
            state.step5.last_scripted_skip_signature = None;
            state.step5.prompt_ready_signature = None;
            state.step5.prompt_ready_seen_count = 0;
            state.step5.prompt_ready_first_seen_unix_ms = None;
            state.step5.restart_program = program;
            state.step5.restart_args = args;
        }
        Err(err) => {
            state.step2.scan_status = format!("Install start failed: {err}");
            state.step5.last_status_text = "Start failed".to_string();
        }
    }
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
