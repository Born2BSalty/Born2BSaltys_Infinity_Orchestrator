// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::log_files::{
    TargetPrepResult, begin_new_run, copy_weidu_logs_for_diagnostics,
    prepare_target_dirs_before_install, validate_resume_paths, validate_runtime_prep_paths,
    verify_targets_prepared,
};
use crate::app::compat_step3_rules;
use crate::app::state::{Step1State, WizardState};
use crate::app::step5::command_config::build_install_command_config;
use crate::app::terminal::EmbeddedTerminal;
use crate::install::step5_command_install::build_install_invocation;
use crate::install::step5_command_resume::{build_resume_invocation, capture_resume_targets};

pub(crate) struct PendingInstallStart {
    program: String,
    args: Vec<String>,
    resume_mode: bool,
    restart_mode: bool,
}

pub(crate) fn step3_install_block_reason(state: &WizardState) -> Option<String> {
    let show_primary_game = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_secondary_game = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let mut blocked_tabs = Vec::<String>::new();

    for (tab, show, mods, items) in [
        (
            "BGEE",
            show_primary_game,
            &state.step2.bgee_mods,
            &state.step3.bgee_items,
        ),
        (
            "BG2EE",
            show_secondary_game,
            &state.step2.bg2ee_mods,
            &state.step3.bg2ee_items,
        ),
    ] {
        if !show {
            continue;
        }
        let count =
            compat_step3_rules::collect_step3_compat_markers(&state.step1, tab, mods, items)
                .values()
                .filter(|marker| {
                    marker.kind.eq_ignore_ascii_case("missing_dep")
                        || marker.kind.eq_ignore_ascii_case("order_block")
                })
                .count();
        if count > 0 {
            blocked_tabs.push(format!("{tab}: {count}"));
        }
    }

    if blocked_tabs.is_empty() {
        None
    } else {
        Some(format!(
            "Resolve Step 3 dependency/order issues before install ({}).",
            blocked_tabs.join(", ")
        ))
    }
}

pub(crate) fn prepare_start_request(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
) -> Option<(PendingInstallStart, bool)> {
    state.step5.start_install_requested = false;
    state.step5.prep_running = false;
    state.step5.prep_ran = false;
    state.step5.prep_used_backup = false;
    state.step5.prep_backup_paths.clear();
    state.step5.prep_cleaned_paths.clear();
    state.step5.resolved_bg1_game_dir.clear();
    state.step5.resolved_bg2_game_dir.clear();
    state.step5.resolved_game_dir.clear();

    if let Some(reason) = step3_install_block_reason(state) {
        state.step5.last_status_text.clone_from(&reason);
        term.append_marker(&reason);
        return None;
    }

    let scripted = super::scripted_inputs::load_from_step1(&state.step1);
    let scripted_count = term.set_scripted_inputs(scripted);
    if scripted_count > 0 {
        term.append_marker(&format!("Loaded {scripted_count} @wlb-inputs token(s)"));
    }
    let resume_mode = state.step5.resume_available;
    let restart_mode = !resume_mode && state.step5.has_run_once;
    let install_config = build_install_command_config(&state.step1);
    let (program, args) = if resume_mode {
        build_resume_invocation(&install_config, &state.step5.resume_targets)
    } else {
        build_install_invocation(&install_config)
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

    if !runtime_ready {
        return None;
    }

    Some((
        PendingInstallStart {
            program,
            args,
            resume_mode,
            restart_mode,
        },
        !resume_mode && state.step1.prepare_target_dirs_before_install,
    ))
}

pub(crate) fn spawn_target_prep_worker(
    step1: Step1State,
) -> Receiver<Result<TargetPrepResult, String>> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let result = prepare_target_dirs_before_install(&step1)
            .map_err(|err| format!("Target prep failed: {err}"))
            .and_then(|prep| {
                verify_targets_prepared(&step1)
                    .map_err(|err| format!("Target prep verify failed: {err}"))?;
                Ok(prep)
            });
        let _ = tx.send(result);
    });
    rx
}

pub(crate) fn apply_completed_prep(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    result: Result<TargetPrepResult, String>,
) -> bool {
    state.step5.prep_running = false;
    state.step5.prep_ran = state.step1.prepare_target_dirs_before_install;
    state.step5.prep_used_backup = state.step1.backup_targets_before_eet_copy;

    match result {
        Ok(prep) => {
            state.step5.prep_backup_paths = prep
                .backups
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            state.step5.prep_cleaned_paths = prep
                .cleaned
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            for path in prep.backups {
                state.step5.last_status_text =
                    format!("Backed up target dir to {}", path.display());
                term.append_marker(&format!("Backup created: {}", path.display()));
            }
            for path in prep.cleaned {
                state.step5.last_status_text = format!("Cleaned target dir {}", path.display());
                term.append_marker(&format!("Target cleaned: {}", path.display()));
            }
            if state.step5.prep_backup_paths.is_empty() && state.step5.prep_cleaned_paths.is_empty()
            {
                state.step5.last_status_text = "Target prep finished".to_string();
                term.append_marker("Target prep finished");
            }
            true
        }
        Err(err) => {
            state.step5.prep_backup_paths.clear();
            state.step5.prep_cleaned_paths.clear();
            state.step5.last_status_text.clone_from(&err);
            term.append_marker(&err);
            false
        }
    }
}

pub(crate) fn start_install_process(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    pending: PendingInstallStart,
) {
    let PendingInstallStart {
        program,
        args,
        resume_mode,
        restart_mode,
    } = pending;

    let run_id = begin_new_run(&mut state.step5);
    term.configure_from_step1(&state.step1, Some(&run_id));
    copy_weidu_logs_for_diagnostics(&state.step1, &run_id);

    match term.start_process(program.as_str(), &args) {
        Ok(()) => {
            apply_start_metadata(state, resume_mode, restart_mode, &args);
            state.step5.prep_running = false;
            state.step5.run_counter = state.step5.run_counter.saturating_add(1);
            state.step5.active_run_id = Some(state.step5.run_counter);
            term.append_marker(&format!("Run #{} started", state.step5.run_counter));
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
            state.step5.last_scripted_fallback_signature = None;
            state.step5.last_scripted_cycle_signature = None;
            state.step5.last_scripted_send_unix_ms = None;
            state.step5.last_scripted_prompt_key = None;
            state.step5.paused_scripted_component_key = None;
            state.step5.prompt_ready_signature = None;
            state.step5.prompt_ready_seen_count = 0;
            state.step5.prompt_ready_first_seen_unix_ms = None;
            state.step5.prompt_required_sound_latched = false;
            state.step5.restart_program = program;
            state.step5.restart_args = args;
            let install_config = build_install_command_config(&state.step1);
            state.step5.resume_targets = capture_resume_targets(&install_config);
        }
        Err(err) => {
            state.step2.scan_status = format!("Install start failed: {err}");
            state.step5.last_status_text = "Start failed".to_string();
            state.step5.prep_running = false;
        }
    }
}

pub(crate) fn confirm_cancel_request(
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
) {
    state.step5.cancel_requested = true;
    if state.step5.cancel_force_checked {
        if let Some(term) = terminal {
            term.force_terminate();
        }
        state.step5.last_status_text = "Force cancel requested".to_string();
        state.step5.cancel_pending = false;
        state.step5.cancel_pending_started_unix_secs = None;
        state.step5.cancel_pending_output_len = None;
        state.step5.cancel_pending_boundary_count = None;
        state.step5.cancel_signal_sent_unix_secs = None;
        state.step5.cancel_attempt_count = 0;
        state.step5.cancel_was_graceful = false;
        state.step5.last_cancel_mode = "force".to_string();
        state.step5.resume_available = false;
        state.step5.resume_targets = crate::app::state::ResumeTargets::default();
    } else {
        state.step5.cancel_pending = true;
        state.step5.cancel_pending_started_unix_secs = Some(now_unix_secs());
        state.step5.cancel_pending_output_len =
            terminal.as_ref().map(|term| term.console_text().len());
        state.step5.cancel_pending_boundary_count =
            terminal.as_ref().map(|term| term.boundary_event_count());
        state.step5.cancel_signal_sent_unix_secs = None;
        state.step5.cancel_attempt_count = 0;
        state.step5.last_status_text =
            "Cancel pending (waiting for component boundary)".to_string();
    }
    state.step5.cancel_confirm_open = false;
    state.step5.cancel_force_checked = false;
}

pub(crate) const fn dismiss_cancel_request(state: &mut WizardState) {
    state.step5.cancel_confirm_open = false;
    state.step5.cancel_force_checked = false;
    state.step5.cancel_requested = false;
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
        .map_or(0, |d| d.as_secs())
}
