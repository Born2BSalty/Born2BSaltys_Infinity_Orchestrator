// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync::scrub_dev_settings;
use crate::app::state::{Step1State, WizardState};
use crate::app::terminal::EmbeddedTerminal;
use crate::settings::model::AppSettings;
use crate::settings::store::SettingsStore;

pub(crate) fn should_save_settings(state: &WizardState, last_saved_step1: &Step1State) -> bool {
    state.step1 != *last_saved_step1
}

pub(crate) fn save_settings_best_effort(
    state: &WizardState,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let mut step1 = state.step1.clone();
    if !dev_mode {
        scrub_dev_settings(&mut step1);
    }
    let settings = AppSettings {
        exe_fingerprint: exe_fingerprint.to_string(),
        step1: step1.into(),
    };
    if settings_store.save(&settings).is_ok() {
        *last_saved_step1 = state.step1.clone();
    }
}

pub(crate) fn shutdown_and_exit(
    state: &WizardState,
    step5_terminal: &mut Option<EmbeddedTerminal>,
    settings_store: &SettingsStore,
    last_saved_step1: &mut Step1State,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> ! {
    if let Some(term) = step5_terminal.as_mut() {
        shutdown_terminal(term);
    }
    save_settings_best_effort(
        state,
        settings_store,
        last_saved_step1,
        dev_mode,
        exe_fingerprint,
    );
    std::process::exit(0);
}

fn shutdown_terminal(term: &mut EmbeddedTerminal) {
    term.shutdown();
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1200);
    while std::time::Instant::now() < deadline {
        term.poll_output();
        if term.process_id().is_none() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if term.process_id().is_some() {
        term.force_terminate();
        let force_deadline = std::time::Instant::now() + std::time::Duration::from_millis(700);
        while std::time::Instant::now() < force_deadline {
            term.poll_output();
            if term.process_id().is_none() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}
