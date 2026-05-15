// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `validate_debounce` — per-edit debounced path validation (H11).
//
// Per Phase 4 P4.T11b:
//   - `mark_dirty(state, field)` is called from each `path_row::on_change`
//     callback in `tab_paths` / `tab_tools`. It writes the current `Instant`
//     into `SettingsScreenState::path_edit_debounce`.
//   - `tick(orchestrator, now)` runs once per frame from
//     `OrchestratorApp::update`. For every dirty field whose timestamp is
//     ≥500ms ago, it runs `validate_now::run_for_field` and updates
//     `path_validation_results.fields[<field>]`. The dirty mark is cleared.
//
// Per-keystroke typing does NOT fire a validation per keystroke; only the
// 500ms-idle pause does.

// rationale: doc-paragraph-length is a subjective style lint; the wording is
// intentional (Cat 3).
#![allow(clippy::too_long_first_doc_paragraph)]

use std::time::{Duration, Instant};

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::validate_now;

/// Debounce window before per-field re-validation fires. Short enough that
/// browse-picker selections and folder-picks feel snappy, long enough to
/// avoid validating on every keystroke while the user is typing a path.
pub const DEBOUNCE_MS: u64 = 200;

/// Mark a path field dirty. Called by `tab_paths` / `tab_tools` from each
/// row's `on_change` callback.
pub fn mark_dirty(orchestrator: &mut OrchestratorApp, field: &'static str) {
    orchestrator
        .settings_screen_state
        .path_edit_debounce
        .insert(field, Instant::now());
}

/// Per-frame tick — runs at the start of `OrchestratorApp::update`. When any
/// dirty field's debounce has elapsed, re-run the full validation pass so
/// per-field hints, the aggregate `overall_ok` / `issue_count`, and the
/// rail-status `step1_path_check` all stay in sync without requiring a
/// Validate-now click.
pub fn tick(orchestrator: &mut OrchestratorApp, now: Instant) {
    let threshold = Duration::from_millis(DEBOUNCE_MS);

    // Are any fields ready to re-validate?
    let any_due = orchestrator
        .settings_screen_state
        .path_edit_debounce
        .values()
        .any(|at| now.saturating_duration_since(*at) >= threshold);

    if !any_due {
        return;
    }

    // Full pass: cheaper at this scale (~10 fields) than chasing per-field
    // deltas, and it guarantees the aggregates + rail status reflect the
    // new state.
    orchestrator.settings_screen_state.path_validation_results =
        validate_now::run_now(&orchestrator.wizard_state.step1);
    orchestrator.wizard_state.step1_path_check = Some(
        crate::app::state_validation::run_path_check(&orchestrator.wizard_state.step1),
    );

    // Clear debounce entries whose timers elapsed; any still-pending fields
    // stay queued until their own timers fire on a later tick.
    orchestrator
        .settings_screen_state
        .path_edit_debounce
        .retain(|_, at| now.saturating_duration_since(*at) < threshold);
}
