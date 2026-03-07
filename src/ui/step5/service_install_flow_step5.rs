// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::WizardState;
use crate::ui::step5::service_step5::{
    begin_diagnostics_run,
    build_install_invocation, build_resume_invocation, capture_resume_targets,
    copy_weidu_logs_for_diagnostics,
    load_scripted_inputs,
    prepare_target_dirs_before_install, validate_resume_paths, validate_runtime_prep_paths,
    verify_targets_prepared,
};
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn start_if_requested(state: &mut WizardState, term: &mut EmbeddedTerminal) {
    if !state.step5.start_install_requested {
        return;
    }

    state.step5.start_install_requested = false;
    state.compat.show_pre_install_modal = false;
    state.step5.prep_ran = false;
    state.step5.prep_used_backup = false;
    state.step5.prep_backup_paths.clear();
    state.step5.prep_cleaned_paths.clear();
    state.step5.resolved_bg1_game_dir.clear();
    state.step5.resolved_bg2_game_dir.clear();
    state.step5.resolved_game_dir.clear();

    term.configure_from_step1(&state.step1);
    let scripted = load_scripted_inputs(&state.step1);
    let scripted_count = term.set_scripted_inputs(scripted);
    if scripted_count > 0 {
        term.append_marker(&format!("Loaded {scripted_count} @wlb-inputs token(s)"));
    }
    let resume_mode = state.step5.resume_available;
    let restart_mode = !resume_mode && state.step5.has_run_once;
    let (program, args) = if resume_mode {
        build_resume_invocation(&state.step1, &state.step5.resume_targets)
    } else {
        build_install_invocation(&state.step1)
    };

    let runtime_ready = match if resume_mode {
        validate_resume_paths(&state.step1, &state.step5.resume_targets)
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
                state.step5.prep_ran = state.step1.prepare_target_dirs_before_install;
                state.step5.prep_used_backup = state.step1.backup_targets_before_eet_copy;
                state.step5.prep_backup_paths =
                    prep.backups.iter().map(|p| p.display().to_string()).collect();
                state.step5.prep_cleaned_paths =
                    prep.cleaned.iter().map(|p| p.display().to_string()).collect();
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
                state.step5.prep_ran = state.step1.prepare_target_dirs_before_install;
                state.step5.prep_used_backup = state.step1.backup_targets_before_eet_copy;
                state.step5.prep_backup_paths.clear();
                state.step5.prep_cleaned_paths.clear();
                state.step5.last_status_text = format!("Backup target dirs failed: {err}");
                false
            }
        }
    };

    if !can_start {
        return;
    }

    let run_id = begin_diagnostics_run(state);
    copy_weidu_logs_for_diagnostics(&state.step1, &run_id);

    match term.start_process(program.as_str(), &args) {
        Ok(()) => {
            apply_start_metadata(state, resume_mode, restart_mode, &args);
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
            state.step5.last_cancel_mode = "none".to_string();
            state.step5.resume_available = false;
            state.step5.last_scripted_skip_signature = None;
            state.step5.prompt_ready_signature = None;
            state.step5.prompt_ready_seen_count = 0;
            state.step5.prompt_ready_first_seen_unix_ms = None;
            state.step5.prompt_required_sound_latched = false;
            state.step5.restart_program = program;
            state.step5.restart_args = args;
            state.step5.resume_targets = capture_resume_targets(&state.step1);
        }
        Err(err) => {
            state.step2.scan_status = format!("Install start failed: {err}");
            state.step5.last_status_text = "Start failed".to_string();
        }
    }
}

fn apply_start_metadata(
    state: &mut WizardState,
    resume_mode: bool,
    restart_mode: bool,
    args: &[String],
) {
    state.step5.last_start_mode = if resume_mode {
        "resume".to_string()
    } else if restart_mode {
        "restart".to_string()
    } else {
        "fresh".to_string()
    };
    if resume_mode {
        state.step5.prep_ran = false;
        state.step5.prep_used_backup = false;
        state.step5.prep_backup_paths.clear();
        state.step5.prep_cleaned_paths.clear();
    }
    state.step5.resolved_bg1_game_dir = arg_value(args, "--bg1-game-directory").unwrap_or_default();
    state.step5.resolved_bg2_game_dir = arg_value(args, "--bg2-game-directory").unwrap_or_default();
    state.step5.resolved_game_dir = arg_value(args, "--game-directory").unwrap_or_default();
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let idx = args.iter().position(|arg| arg == flag)?;
    args.get(idx + 1).cloned()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
