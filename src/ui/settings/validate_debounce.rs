// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{Duration, Instant};

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::validate_now;

pub const DEBOUNCE_MS: u64 = 200;

pub fn mark_dirty(orchestrator: &mut OrchestratorApp, field: &'static str) {
    orchestrator
        .settings_screen_state
        .path_edit_debounce
        .insert(field, Instant::now());
}

pub fn tick(orchestrator: &mut OrchestratorApp, now: Instant) {
    let threshold = Duration::from_millis(DEBOUNCE_MS);

    let any_due = orchestrator
        .settings_screen_state
        .path_edit_debounce
        .values()
        .any(|at| now.saturating_duration_since(*at) >= threshold);

    if !any_due {
        return;
    }

    orchestrator.settings_screen_state.path_validation_results =
        validate_now::run_now(&orchestrator.wizard_state.step1);
    orchestrator.wizard_state.step1_path_check = Some(
        crate::app::state_validation::run_path_check(&orchestrator.wizard_state.step1),
    );

    orchestrator
        .settings_screen_state
        .path_edit_debounce
        .retain(|_, at| now.saturating_duration_since(*at) < threshold);
}
