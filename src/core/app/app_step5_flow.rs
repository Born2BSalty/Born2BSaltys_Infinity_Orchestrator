// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app::state::WizardState;
use crate::app::step5::basic_install_report;
use crate::app::step5::install_flow::{
    InstallPreparation, PendingInstallStart, apply_completed_prep, prepare_start_request,
    spawn_install_preparation, start_install_process,
};
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
    step5_prep_rx: &mut Option<Receiver<Result<InstallPreparation, String>>>,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    poll_step5_prep_with(
        state,
        step5_prep_rx,
        step5_terminal,
        step5_terminal_error,
        step5_pending_start,
        start_install_process,
    )
}

fn poll_step5_prep_with<F>(
    state: &mut WizardState,
    step5_prep_rx: &mut Option<Receiver<Result<InstallPreparation, String>>>,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_pending_start: &mut Option<PendingInstallStart>,
    start: F,
) -> bool
where
    F: FnOnce(&mut WizardState, &mut EmbeddedTerminal, PendingInstallStart),
{
    let Some(rx) = step5_prep_rx.as_ref() else {
        return false;
    };

    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            Some(Err("Installer preparation worker disconnected".to_string()))
        }
    };
    let Some(result) = result else {
        return false;
    };

    *step5_prep_rx = None;
    if !step5_pending_start
        .as_ref()
        .is_some_and(|pending| pending.matches(state))
    {
        *step5_pending_start = None;
        state.step5.prep_running = false;
        return true;
    }
    if result.as_ref().is_ok_and(|prepared| {
        step5_pending_start
            .as_ref()
            .is_none_or(|pending| pending.run_token != prepared.run_token)
    }) {
        *step5_pending_start = None;
        state.step5.prep_running = false;
        return true;
    }
    state.step5.prep_running = false;
    ensure_step5_terminal(step5_terminal, step5_terminal_error);
    let Some(term) = step5_terminal.as_mut() else {
        *step5_pending_start = None;
        state.step5.last_status_text =
            "Target prep finished, but terminal is unavailable".to_string();
        basic_install_report::update(state, "start_failed", None);
        return false;
    };

    finish_prepared_start(state, term, step5_pending_start, result, start);
    true
}

fn finish_prepared_start<F>(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    step5_pending_start: &mut Option<PendingInstallStart>,
    result: Result<InstallPreparation, String>,
    start: F,
) where
    F: FnOnce(&mut WizardState, &mut EmbeddedTerminal, PendingInstallStart),
{
    match result {
        Ok(prepared) => {
            if let Some(parser) = prepared.parser.as_ref() {
                state
                    .step5
                    .basic_install_mod_installer_version
                    .clone_from(&parser.preparation.version.display);
                term.append_marker(&format!(
                    "Installer parser ready: {} phrases",
                    parser.preparation.phrases.len()
                ));
                if let Some(path) = parser.preparation.backup_path.as_ref() {
                    term.append_marker(&format!("Parser backup retained: {}", path.display()));
                }
            }
            if let Some(warning) = prepared.parser_warning.as_ref() {
                state.step5.basic_install_mod_installer_version = format!("unavailable: {warning}");
                term.append_marker(&format!("Installer parser warning: {warning}"));
            }
            let ready = prepared
                .target
                .is_none_or(|target| apply_completed_prep(state, term, Ok(target)));
            if ready {
                if let Some(pending) = step5_pending_start.take() {
                    start(state, term, pending);
                }
            } else {
                *step5_pending_start = None;
            }
            if let Some(parser) = prepared.parser {
                drop(parser.lease);
            }
        }
        Err(error) => {
            state.step5.basic_install_mod_installer_version = format!("unavailable: {error}");
            apply_completed_prep(state, term, Err(error));
            *step5_pending_start = None;
        }
    }
}

pub(crate) fn start_if_requested(
    state: &mut WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_prep_rx: &mut Option<Receiver<Result<InstallPreparation, String>>>,
    step5_pending_start: &mut Option<PendingInstallStart>,
) -> bool {
    start_if_requested_with(
        state,
        step5_terminal,
        step5_terminal_error,
        step5_prep_rx,
        step5_pending_start,
        spawn_install_preparation,
    )
}

fn start_if_requested_with<F>(
    state: &mut WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    step5_terminal_error: &mut Option<String>,
    step5_prep_rx: &mut Option<Receiver<Result<InstallPreparation, String>>>,
    step5_pending_start: &mut Option<PendingInstallStart>,
    spawn: F,
) -> bool
where
    F: FnOnce(
        crate::app::state::Step1State,
        Vec<String>,
        bool,
        u64,
    ) -> Receiver<Result<InstallPreparation, String>>,
{
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
        state.step5.basic_install_mod_installer_version =
            "unavailable: terminal unavailable".to_string();
        basic_install_report::update(state, "start_failed", None);
        return false;
    };

    let run_id = begin_new_run(&mut state.step5);
    basic_install_report::begin(state, &run_id);

    let Some((pending, needs_prep)) = prepare_start_request(state, term) else {
        state.step5.basic_install_mod_installer_version =
            "unavailable: start rejected before parser preparation".to_string();
        let report_state = state.step5.basic_install_report_state.clone();
        basic_install_report::update(state, &report_state, Some(term));
        return false;
    };

    state.step5.prep_running = true;
    state.step5.last_status_text = "Preparing installer parser...".to_string();
    term.append_marker("Preparing installer parser...");
    *step5_prep_rx = Some(spawn(
        pending.preparation_step1(),
        state.parser_choice_phrases.clone(),
        needs_prep,
        pending.run_token,
    ));
    *step5_pending_start = Some(pending);
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
        for confirmed in term.take_confirmed_answers() {
            if let Err(error) = crate::app::step5::answer_capture::apply_confirmed(state, confirmed)
            {
                term.append_marker(&format!("Answers were not saved: {error:?}"));
            }
        }
        for warning in term.take_capture_warnings() {
            state.step5.last_status_text.clone_from(&warning);
            term.append_marker(&warning);
        }
        crate::app::step5_runtime_status::process_graceful_cancel(&mut state.step5, term);
        crate::app::step5_runtime_status::process_exit_event(state, term);
        basic_install_report::poll(state, Some(term));
        needs_repaint
    } else {
        basic_install_report::poll(state, None);
        false
    }
}
#[cfg(test)]
mod parser_sharing_tests {
    use super::*;
    use crate::app::step5::install_flow::parser_sharing_pending_for_test;

    #[test]
    fn parser_sharing_cancel_and_stale_completions_never_spawn() {
        for stale in 0..5 {
            let mut state = WizardState::default();
            state.step1.mod_installer_binary.clear();
            state.step5.prep_running = true;
            state.step5.diagnostics_run_id = Some("fixture-run".to_string());
            state.parser_choice_phrases = vec!["Imported".to_string()];
            let pending = parser_sharing_pending_for_test(&state);
            let run_token = pending.run_token;
            let mut pending = Some(pending);
            match stale {
                0 => state.step5.cancel_requested = true,
                1 => state.parser_choice_phrases.clear(),
                2 => state.step1.mod_installer_binary = "changed.exe".to_string(),
                3 => state.step5.diagnostics_run_id = Some("new-run".to_string()),
                _ => {}
            }
            let (sender, receiver) = std::sync::mpsc::channel();
            sender
                .send(Ok(InstallPreparation {
                    run_token: if stale == 4 { run_token + 1 } else { run_token },
                    parser: None,
                    target: None,
                    parser_warning: None,
                }))
                .unwrap_or_else(|_| panic!("send"));
            let mut receiver = Some(receiver);
            assert!(poll_step5_prep_with(
                &mut state,
                &mut receiver,
                &mut None,
                &mut Some("must not create terminal".to_string()),
                &mut pending,
                |_, _, _| panic!("stale completion must not spawn")
            ));
            assert!(receiver.is_none());
            assert!(pending.is_none());
            assert!(!state.step5.prep_running);
            assert!(!state.step5.install_running);
        }
    }

    #[test]
    fn parser_sharing_preparation_error_never_spawns_install() {
        let mut state = WizardState::default();
        state.step1.mod_installer_binary.clear();
        state.step5.prep_running = true;
        state.parser_choice_phrases = vec!["Imported".to_string()];
        let mut pending = Some(parser_sharing_pending_for_test(&state));
        let mut terminal = EmbeddedTerminal::new().expect("terminal");
        finish_prepared_start(
            &mut state,
            &mut terminal,
            &mut pending,
            Err("Installer parser preparation failed: fixture timeout".to_string()),
            |_, _, _| panic!("failed preparation must not spawn"),
        );
        assert!(pending.is_none());
        assert!(!state.step5.install_running);
        assert!(!state.step5.prep_ran);
        assert!(state.step5.last_status_text.contains("fixture timeout"));
        assert!(
            state
                .step5
                .basic_install_mod_installer_version
                .contains("unavailable")
        );
    }
    #[test]
    fn parser_sharing_success_holds_lease_through_final_callback_without_target_prep() {
        use crate::app::mod_installer_parser::{
            ParserInitializer, VersionOutput, capture_local_choice_phrases, prepare_parser_at,
            with_parser_path_for_test,
        };
        struct Initializer(std::path::PathBuf);
        impl ParserInitializer for Initializer {
            fn run_version(&self, _: &std::path::Path) -> Result<VersionOutput, String> {
                std::fs::write(
                    &self.0,
                    "choice_phrase = ['Default']\n[metadata]\nmod_installer_version = '15.0.1'\n",
                )
                .map_err(|error| error.to_string())?;
                Ok(VersionOutput {
                    display: "mod_installer 15.0.1".to_string(),
                    version: "15.0.1".to_string(),
                })
            }
        }
        let root = std::env::temp_dir().join(format!(
            "bio_parser_lease_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir(&root).expect("fixture");
        let parser_path = root.join("parser.toml");
        let parser = prepare_parser_at(
            &parser_path,
            &root.join("never.exe"),
            &["Imported".to_string()],
            &Initializer(parser_path.clone()),
        )
        .unwrap_or_else(|error| panic!("fixture preparation: {error}"));
        let mut state = WizardState::default();
        state.step1.mod_installer_binary.clear();
        state.step5.prep_running = true;
        let pending = parser_sharing_pending_for_test(&state);
        let result = InstallPreparation {
            run_token: pending.run_token,
            parser: Some(parser),
            target: None,
            parser_warning: None,
        };
        let mut pending = Some(pending);
        let mut terminal = EmbeddedTerminal::new().expect("terminal");
        let spawned = std::cell::Cell::new(false);
        finish_prepared_start(
            &mut state,
            &mut terminal,
            &mut pending,
            Ok(result),
            |state, _, _| {
                spawned.set(true);
                assert!(!state.step5.prep_ran);
                assert!(state.step5.prep_cleaned_paths.is_empty());
                assert_eq!(
                    state.step5.basic_install_mod_installer_version,
                    "mod_installer 15.0.1"
                );
                with_parser_path_for_test(&parser_path, || {
                    assert!(capture_local_choice_phrases().is_err());
                });
            },
        );
        assert!(spawned.get());
        assert!(pending.is_none());
        std::fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn parser_sharing_all_accepted_start_modes_use_injected_common_preparation() {
        let root = std::env::temp_dir().join(format!(
            "bio_parser_start_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let source = root.join("source");
        let target = root.join("target");
        for directory in [&source, &target] {
            std::fs::create_dir_all(directory).expect("fixture directory");
        }
        std::fs::write(source.join("chitin.key"), b"fixture").expect("fixture game marker");
        for game in ["BGEE", "EET"] {
            for mode in ["fresh", "restart", "resume"] {
                for needs_target in [false, true] {
                    for imported in [false, true] {
                        let source = source.to_string_lossy().into_owned();
                        let target = target.to_string_lossy().into_owned();
                        let mut state = WizardState {
                            step1: crate::app::state::Step1State {
                                game_install: game.to_string(),
                                mod_installer_binary: root
                                    .join("never-executed.exe")
                                    .to_string_lossy()
                                    .into_owned(),
                                weidu_binary: String::new(),
                                bgee_game_folder: source.clone(),
                                bg2ee_game_folder: source.clone(),
                                eet_bgee_game_folder: source.clone(),
                                eet_bg2ee_game_folder: source,
                                generate_directory: target.clone(),
                                eet_pre_dir: target.clone(),
                                eet_new_dir: target,
                                generate_directory_enabled: true,
                                new_pre_eet_dir_enabled: true,
                                new_eet_dir_enabled: true,
                                prepare_target_dirs_before_install: needs_target,
                                mods_folder: String::new(),
                                bgee_log_file: String::new(),
                                bg2ee_log_file: String::new(),
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        state.step5.has_run_once = mode == "restart";
                        state.step5.resume_available = mode == "resume";
                        state.step5.start_install_requested = true;
                        if imported {
                            state.parser_choice_phrases = vec!["Imported".to_string()];
                        }
                        let mut terminal = Some(EmbeddedTerminal::new().expect("terminal"));
                        let mut receiver = None;
                        let mut pending = None;
                        let invoked = std::cell::Cell::new(false);
                        assert!(
                            start_if_requested_with(
                                &mut state,
                                &mut terminal,
                                &mut None,
                                &mut receiver,
                                &mut pending,
                                |step1, phrases, target, run_token| {
                                    invoked.set(true);
                                    assert_eq!(step1.game_install, game);
                                    assert_eq!(phrases.len(), usize::from(imported));
                                    assert_eq!(target, needs_target && mode != "resume");
                                    assert!(run_token > 0);
                                    let (sender, receiver) = std::sync::mpsc::channel();
                                    sender
                                        .send(Err("injected parser failure".to_string()))
                                        .unwrap_or_else(|_| panic!("send"));
                                    receiver
                                }
                            ),
                            "{game} {mode} target={needs_target} imported={imported}"
                        );
                        assert!(invoked.get());
                        assert!(state.step5.prep_running);
                        assert!(pending.is_some());
                        assert!(poll_step5_prep_with(
                            &mut state,
                            &mut receiver,
                            &mut terminal,
                            &mut None,
                            &mut pending,
                            |_, _, _| panic!("fixture error must never spawn")
                        ));
                        assert!(!state.step5.install_running);
                        assert!(!state.step5.prep_ran);
                        assert!(pending.is_none());
                    }
                }
            }
        }
        std::fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
