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

    let game_folder_due = orchestrator
        .settings_screen_state
        .path_edit_debounce
        .iter()
        .any(|(field, at)| {
            now.saturating_duration_since(*at) >= threshold
                && validate_now::is_game_folder_field(field)
        });

    orchestrator.settings_screen_state.path_validation_results =
        validate_now::run_now(&orchestrator.wizard_state.step1);
    orchestrator.wizard_state.step1_path_check = Some(
        crate::app::state_validation::run_path_check(&orchestrator.wizard_state.step1),
    );
    if game_folder_due {
        crate::app::compat_dlc_source::invalidate_source_check(
            &mut orchestrator.wizard_state.step1,
        );
    }

    orchestrator
        .settings_screen_state
        .path_edit_debounce
        .retain(|_, at| now.saturating_duration_since(*at) < threshold);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::compat_dlc_source::refresh_source_check;
    use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

    fn app_with_settled_edit(field: &'static str) -> (OrchestratorApp, Instant) {
        let mut app = OrchestratorApp::new_isolated_for_test("validate_debounce");
        app.wizard_state.step1.bgee_game_folder = std::env::temp_dir()
            .join(format!("bio_debounce_test_{}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        assert!(refresh_source_check(&mut app.wizard_state.step1));
        mark_dirty(&mut app, field);
        (
            app,
            Instant::now() + Duration::from_millis(DEBOUNCE_MS * 10),
        )
    }

    #[test]
    fn a_settled_game_folder_edit_invalidates_the_source_probes() {
        let (mut app, later) = app_with_settled_edit(validate_now::FIELD_BGEE_GAME_FOLDER);
        tick(&mut app, later);
        assert!(refresh_source_check(&mut app.wizard_state.step1));
    }

    #[test]
    fn a_settled_edit_to_another_path_keeps_the_source_probes() {
        let (mut app, later) = app_with_settled_edit(validate_now::FIELD_MODS_ARCHIVE_FOLDER);
        tick(&mut app, later);
        assert!(!refresh_source_check(&mut app.wizard_state.step1));
    }
}
