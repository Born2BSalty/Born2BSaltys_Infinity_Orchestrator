// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::settings::model::AppSettings;
use crate::ui::controller::step3_sync::scrub_dev_settings;

use super::WizardApp;

pub(super) fn shutdown_and_exit(app: &mut WizardApp) -> ! {
    if let Some(term) = app.step5_terminal.as_mut() {
        term.shutdown();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1200);
        while std::time::Instant::now() < deadline {
            term.poll_output();
            if term.process_id().is_none() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if term.process_id().is_some() {
            term.force_terminate();
            let force_deadline = std::time::Instant::now() + std::time::Duration::from_millis(700);
            while std::time::Instant::now() < force_deadline {
                term.poll_output();
                if term.process_id().is_none() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
    save_settings_best_effort(app);
    std::process::exit(0);
}

pub(super) fn save_settings_best_effort(app: &mut WizardApp) {
    let mut step1 = app.state.step1.clone();
    if !app.dev_mode {
        scrub_dev_settings(&mut step1);
    }
    let settings = AppSettings {
        exe_fingerprint: app.exe_fingerprint.clone(),
        step1: step1.into(),
    };
    if app.settings_store.save(&settings).is_ok() {
        app.last_saved_step1 = app.state.step1.clone();
    }
}
