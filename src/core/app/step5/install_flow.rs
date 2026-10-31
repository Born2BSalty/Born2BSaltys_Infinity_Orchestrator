// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::log_files::{
    TargetPrepResult, begin_new_run, copy_weidu_logs_for_diagnostics,
    prepare_target_dirs_before_install, prune_old_diagnostics, validate_resume_paths,
    validate_runtime_prep_paths, verify_targets_prepared,
};
use crate::app::compat_step3_rules;
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_installer_parser::{PreparedParser, prepare_local_parser};
use crate::app::state::{Step1State, WizardState};
use crate::app::step5::basic_install_report;
use crate::app::step5::command_config::build_install_command_config;
use crate::app::terminal::EmbeddedTerminal;
use crate::install::step5_command_install::build_install_invocation;
use crate::install::step5_command_resume::{build_resume_invocation, capture_resume_targets};
use std::path::{Path, PathBuf};

use crate::platform_defaults::resolve_mod_installer_binary;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct PendingInstallStart {
    program: String,
    args: Vec<String>,
    resume_mode: bool,
    restart_mode: bool,
    pub(crate) run_token: u64,
    step1: Step1State,
    phrases: Vec<String>,
    diagnostics_run_id: Option<String>,
    executable_identity: Option<(PathBuf, u64, Option<SystemTime>)>,
}

fn executable_identity(configured: &str) -> Option<(PathBuf, u64, Option<SystemTime>)> {
    let mut paths = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(executable) = std::env::current_exe()
            && let Some(parent) = executable.parent()
        {
            paths.push(parent.to_path_buf());
        }
        if let Ok(current) = std::env::current_dir() {
            paths.push(current);
        }
        if let Some(windows) = std::env::var_os("SystemRoot") {
            let windows = PathBuf::from(windows);
            paths.push(windows.join("System32"));
            paths.push(windows.join("System"));
            paths.push(windows);
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    executable_identity_with_paths(configured, &paths)
}

fn executable_identity_with_paths(
    configured: &str,
    paths: &[PathBuf],
) -> Option<(PathBuf, u64, Option<SystemTime>)> {
    let program = resolve_mod_installer_binary(configured);
    let program = Path::new(&program);
    if program.is_absolute() || program.components().count() > 1 {
        return executable_file_identity(program);
    }
    paths
        .iter()
        .find_map(|directory| executable_file_identity(&directory.join(program)))
}

fn executable_file_identity(path: &Path) -> Option<(PathBuf, u64, Option<SystemTime>)> {
    #[cfg(target_os = "windows")]
    let path = if path.extension().is_none() {
        path.with_extension("exe")
    } else {
        path.to_path_buf()
    };
    let path = path.canonicalize().ok()?;
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    Some((path, metadata.len(), metadata.modified().ok()))
}

impl PendingInstallStart {
    fn new(
        state: &WizardState,
        program: String,
        args: Vec<String>,
        resume_mode: bool,
        restart_mode: bool,
    ) -> Self {
        static NEXT_ATTEMPT: AtomicU64 = AtomicU64::new(1);
        let executable_identity = executable_identity(&program);
        let program = executable_identity.as_ref().map_or(program, |identity| {
            identity.0.to_string_lossy().into_owned()
        });
        Self {
            program,
            args,
            resume_mode,
            restart_mode,
            run_token: NEXT_ATTEMPT.fetch_add(1, Ordering::Relaxed),
            step1: state.step1.clone(),
            phrases: state.parser_choice_phrases.clone(),
            diagnostics_run_id: state.step5.diagnostics_run_id.clone(),
            executable_identity,
        }
    }
    pub(crate) fn preparation_step1(&self) -> Step1State {
        let mut step1 = self.step1.clone();
        step1.mod_installer_binary.clone_from(&self.program);
        step1
    }

    pub(crate) fn matches(&self, state: &WizardState) -> bool {
        state.step5.prep_running
            && !state.step5.cancel_requested
            && self.executable_identity == executable_identity(&state.step1.mod_installer_binary)
            && self.step1 == state.step1
            && self.phrases == state.parser_choice_phrases
            && self.diagnostics_run_id == state.step5.diagnostics_run_id
    }
}

pub(crate) struct InstallPreparation {
    pub(crate) run_token: u64,
    pub(crate) parser: Option<PreparedParser>,
    pub(crate) target: Option<TargetPrepResult>,
    pub(crate) parser_warning: Option<String>,
}

#[must_use]
pub fn step3_gate_tabs(game_install: &str) -> [(&'static str, bool); 2] {
    [
        (
            game_authority::first_slot_tab(game_install),
            game_authority::has_slot(game_install, GameSlot::First),
        ),
        (
            game_authority::TAB_BG2EE,
            game_authority::has_slot(game_install, GameSlot::Second),
        ),
    ]
}

#[must_use]
pub fn step3_install_block_reason(state: &WizardState) -> Option<String> {
    let [
        (first_tab, has_first_game_tab),
        (second_tab, has_second_game_tab),
    ] = step3_gate_tabs(&state.step1.game_install);
    let mut blocked_tabs = Vec::<String>::new();

    for (tab, show, mods, items) in [
        (
            first_tab,
            has_first_game_tab,
            &state.step2.bgee_mods,
            &state.step3.bgee_items,
        ),
        (
            second_tab,
            has_second_game_tab,
            &state.step2.bg2ee_mods,
            &state.step3.bg2ee_items,
        ),
    ] {
        if !show {
            continue;
        }
        let count = compat_step3_rules::collect_step3_compat_markers(
            &state.step1,
            tab,
            mods,
            items,
            &state.step3.bgee_items,
        )
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

pub fn prepare_start_request(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
) -> Option<(PendingInstallStart, bool)> {
    state.step5.start_install_requested = false;
    state.step5.cancel_requested = false;
    state.step5.prep_running = false;
    state.step5.last_install_failed = false;
    state.step5.last_exit_code = None;
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
        basic_install_report::update(state, "blocked", Some(term));
        return None;
    }

    term.invalidate_capture();
    state.step5.capture_run = None;
    let scripted = super::scripted_inputs::load_from_step1(&state.step1).unwrap_or_else(|error| {
        term.append_marker(&format!("Scripted inputs disabled: {error}"));
        super::answer_capture::ReplayPlan::default()
    });
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
            basic_install_report::update(state, "preflight_failed", Some(term));
            false
        }
    };

    if !runtime_ready {
        return None;
    }

    Some((
        PendingInstallStart::new(state, program, args, resume_mode, restart_mode),
        !resume_mode && state.step1.prepare_target_dirs_before_install,
    ))
}

pub(crate) fn spawn_install_preparation(
    step1: Step1State,
    phrases: Vec<String>,
    needs_target_prep: bool,
    run_token: u64,
) -> Receiver<Result<InstallPreparation, String>> {
    spawn_install_preparation_with(
        step1,
        phrases,
        needs_target_prep,
        run_token,
        prepare_local_parser,
    )
}

fn spawn_install_preparation_with<F>(
    step1: Step1State,
    phrases: Vec<String>,
    needs_target_prep: bool,
    run_token: u64,
    prepare: F,
) -> Receiver<Result<InstallPreparation, String>>
where
    F: FnOnce(&Path, &[String]) -> Result<PreparedParser, String> + Send + 'static,
{
    let (tx, rx) = channel();
    let _ = thread::Builder::new()
        .name("bio-install-preparation".to_string())
        .spawn(move || {
            let program = resolve_mod_installer_binary(&step1.mod_installer_binary);
            let parser_result = prepare(Path::new(&program), &phrases);
            let result = complete_install_preparation(
                &step1,
                &phrases,
                needs_target_prep,
                run_token,
                parser_result,
            );
            let _ = tx.send(result);
        });
    rx
}

fn complete_install_preparation(
    step1: &Step1State,
    phrases: &[String],
    needs_target_prep: bool,
    run_token: u64,
    parser_result: Result<PreparedParser, String>,
) -> Result<InstallPreparation, String> {
    let (parser, parser_warning) = match parser_result {
        Ok(parser) => (Some(parser), None),
        Err(error) if phrases.is_empty() => (None, Some(error)),
        Err(error) => return Err(format!("Installer parser preparation failed: {error}")),
    };
    let target = if needs_target_prep {
        let prep = prepare_target_dirs_before_install(step1)
            .map_err(|error| format!("Target prep failed: {error}"))?;
        verify_targets_prepared(step1)
            .map_err(|error| format!("Target prep verify failed: {error}"))?;
        Some(prep)
    } else {
        None
    };
    Ok(InstallPreparation {
        run_token,
        parser,
        target,
        parser_warning,
    })
}

pub fn apply_completed_prep(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    result: Result<TargetPrepResult, String>,
) -> bool {
    state.step5.prep_running = false;

    match result {
        Ok(prep) => {
            state.step5.prep_ran = true;
            state.step5.prep_used_backup = state.step1.backup_targets_before_eet_copy;
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
            state.step5.prep_ran = false;
            state.step5.prep_used_backup = false;
            state.step5.prep_backup_paths.clear();
            state.step5.prep_cleaned_paths.clear();
            state.step5.last_status_text.clone_from(&err);
            term.append_marker(&err);
            basic_install_report::update(state, "preparation_failed", Some(term));
            false
        }
    }
}

pub fn start_install_process(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    pending: PendingInstallStart,
) {
    let PendingInstallStart {
        program,
        args,
        resume_mode,
        restart_mode,
        ..
    } = pending;

    let run_id = state
        .step5
        .diagnostics_run_id
        .clone()
        .unwrap_or_else(|| begin_new_run(&mut state.step5));
    term.configure_from_step1(&state.step1, Some(&run_id));
    copy_weidu_logs_for_diagnostics(&state.step1, &run_id);
    basic_install_report::update(state, "starting", Some(term));

    match term.start_process(program.as_str(), &args) {
        Ok(()) => {
            prune_old_diagnostics(Some(&run_id));
            apply_start_metadata(state, resume_mode, restart_mode, &args);
            state.pending_install_choice = None;
            state.pending_install_source = None;
            state.pending_install_choice_modlist_id = None;
            state.step5.prep_running = false;
            state.step5.run_counter = state.step5.run_counter.saturating_add(1);
            state.step5.active_run_id = Some(state.step5.run_counter);
            let capture_run = super::answer_capture::CaptureRunKey {
                run_id: state.step5.run_counter,
                workspace_id: None,
                generation: next_capture_generation(),
                game_install: state.step1.game_install.clone(),
            };
            term.set_capture_session(super::answer_capture::CaptureSession::new(
                capture_run.clone(),
                super::answer_capture::frozen_components(state),
            ));
            state.step5.capture_run = Some(capture_run);
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
            basic_install_report::update(state, "running", Some(term));
        }
        Err(err) => {
            state.step2.scan_status = format!("Install start failed: {err}");
            state.step5.last_status_text = format!("Install start failed: {err}");
            state.step5.prep_running = false;
            basic_install_report::update(state, "start_failed", Some(term));
        }
    }
}

pub fn confirm_cancel_request(state: &mut WizardState, terminal: Option<&mut EmbeddedTerminal>) {
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

pub const fn dismiss_cancel_request(state: &mut WizardState) {
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

fn next_capture_generation() -> u64 {
    static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step3_blockers_gate_an_iwdee_install() {
        assert_eq!(step3_gate_tabs("BGEE"), [("BGEE", true), ("BG2EE", false)]);
        assert_eq!(step3_gate_tabs("BG2EE"), [("BGEE", false), ("BG2EE", true)]);
        assert_eq!(step3_gate_tabs("EET"), [("BGEE", true), ("BG2EE", true)]);
        assert_eq!(
            step3_gate_tabs("IWDEE"),
            [("IWDEE", true), ("BG2EE", false)]
        );
    }

    #[test]
    fn prepare_start_request_preserves_previous_console_run_output() {
        let mut state = WizardState::default();
        let mut term = EmbeddedTerminal::new().expect("embedded terminal");
        term.append_marker("previous run line that must persist");
        assert!(term.output_text().contains("previous run line"));

        let _ = prepare_start_request(&mut state, &mut term);

        assert!(
            term.output_text().contains("previous run line"),
            "starting a new install attempt must not clear the Step 5 console"
        );
    }

    #[test]
    fn prepare_start_request_invalidates_previous_clean_exit() {
        let mut state = WizardState::default();
        state.step5.last_exit_code = Some(0);
        state.step5.last_install_failed = false;
        let mut term = EmbeddedTerminal::new().expect("embedded terminal");

        let _ = prepare_start_request(&mut state, &mut term);

        assert_eq!(
            state.step5.last_exit_code, None,
            "a new attempt must not inherit the previous successful exit code"
        );
        assert!(
            !crate::ui::workspace::step5::success_banner::clean_exit(&state),
            "after a new attempt starts, nav-away reset must not see stale success"
        );
    }
}

#[cfg(test)]
#[path = "install_flow_retention_tests.rs"]
mod retention_tests;
#[cfg(test)]
pub(crate) fn parser_sharing_pending_for_test(state: &WizardState) -> PendingInstallStart {
    PendingInstallStart::new(
        state,
        "never-executed.exe".to_string(),
        Vec::new(),
        false,
        false,
    )
}

#[cfg(test)]
mod parser_sharing_tests {
    use super::*;

    #[test]
    fn parser_sharing_required_error_blocks_target_preparation_but_legacy_is_warning() {
        let state = Step1State::default();
        let error = complete_install_preparation(
            &state,
            &["Imported".to_string()],
            true,
            71,
            Err("fixture timeout".to_string()),
        )
        .err()
        .expect("blocked");
        assert_eq!(
            error,
            "Installer parser preparation failed: fixture timeout"
        );
        let result = complete_install_preparation(
            &state,
            &[],
            false,
            72,
            Err("fixture bad exit".to_string()),
        )
        .unwrap_or_else(|error| panic!("legacy remains available: {error}"));
        assert_eq!(result.run_token, 72);
        assert!(result.target.is_none());
        assert!(result.parser.is_none());
        assert_eq!(result.parser_warning.as_deref(), Some("fixture bad exit"));
    }

    #[test]
    fn parser_sharing_worker_receives_snapshots_and_cannot_prepare_target_after_error() {
        let step1 = Step1State {
            mod_installer_binary: "fixture-not-real.exe".to_string(),
            ..Default::default()
        };
        let receiver = spawn_install_preparation_with(
            step1,
            vec!["Imported".to_string()],
            true,
            73,
            |executable, phrases| {
                let expected = if cfg!(windows) {
                    "fixture-not-real.exe"
                } else {
                    "fixture-not-real"
                };
                assert_eq!(executable, Path::new(expected));
                assert_eq!(phrases, ["Imported"]);
                Err("injected failure before cleanup".to_string())
            },
        );
        let error = receiver
            .recv_timeout(std::time::Duration::from_secs(2))
            .expect("worker result")
            .err()
            .expect("blocked");
        assert_eq!(
            error,
            "Installer parser preparation failed: injected failure before cleanup"
        );
    }
}

#[cfg(test)]
#[path = "install_flow_parser_tests.rs"]
mod parser_sharing_resolution_tests;
