// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_popup_nav::RelatedJumpOutcome;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WorkspaceStep;

pub(crate) fn apply_related_jump_outcome(
    orchestrator: &mut OrchestratorApp,
    outcome: Option<RelatedJumpOutcome>,
) {
    match outcome {
        Some(RelatedJumpOutcome::HoppedToStep2) => {
            orchestrator.workspace_view.current_step = WorkspaceStep::Step2;
        }
        Some(RelatedJumpOutcome::NotInMods {
            related_mod,
            related_component,
        }) => {
            orchestrator
                .notification_manager
                .warn(not_in_mods_notice(&related_mod, related_component));
        }
        Some(RelatedJumpOutcome::LandedOnStep3 | RelatedJumpOutcome::LandedOnStep2) | None => {}
    }
}

#[must_use]
pub(crate) fn not_in_mods_notice(related_mod: &str, related_component: Option<u32>) -> String {
    related_component.map_or_else(
        || format!("Nothing to jump to: {related_mod} is not among this modlist's mods."),
        |id| format!("Nothing to jump to: {related_mod} #{id} is not among this modlist's mods."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hop_switches_the_workspace_to_mods_and_components() {
        let mut app = OrchestratorApp::new_isolated_for_test("relatedjumphop");
        app.workspace_view.current_step = WorkspaceStep::Step3;

        apply_related_jump_outcome(&mut app, Some(RelatedJumpOutcome::HoppedToStep2));

        assert_eq!(app.workspace_view.current_step, WorkspaceStep::Step2);
        assert_eq!(app.notification_manager.history().len(), 0);
    }

    #[test]
    fn not_in_mods_raises_one_toast_and_keeps_the_step() {
        let mut app = OrchestratorApp::new_isolated_for_test("relatedjumpnotinmods");
        app.workspace_view.current_step = WorkspaceStep::Step3;

        apply_related_jump_outcome(
            &mut app,
            Some(RelatedJumpOutcome::NotInMods {
                related_mod: "Spell Revisions".to_string(),
                related_component: None,
            }),
        );

        assert_eq!(app.workspace_view.current_step, WorkspaceStep::Step3);
        let history = app.notification_manager.history();
        assert_eq!(history.len(), 1);
        assert_eq!(
            history.back().map(|record| record.text.as_str()),
            Some("Nothing to jump to: Spell Revisions is not among this modlist's mods.")
        );
    }

    #[test]
    fn a_step3_landing_changes_nothing() {
        let mut app = OrchestratorApp::new_isolated_for_test("relatedjumplanded");
        app.workspace_view.current_step = WorkspaceStep::Step3;

        apply_related_jump_outcome(&mut app, Some(RelatedJumpOutcome::LandedOnStep3));

        assert_eq!(app.workspace_view.current_step, WorkspaceStep::Step3);
        assert_eq!(app.notification_manager.history().len(), 0);
    }

    #[test]
    fn not_in_mods_notice_names_the_component_when_there_is_one() {
        let text = not_in_mods_notice("Spell Revisions", Some(0));
        assert_eq!(
            text,
            "Nothing to jump to: Spell Revisions #0 is not among this modlist's mods."
        );
    }
}
