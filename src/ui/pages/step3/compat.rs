// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod jump;
mod modal;
mod model;

use eframe::egui;

use crate::ui::state::WizardState;
use model::CompatJumpAction;

pub(super) fn render_modal(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    jump_to_selected_requested: &mut bool,
) {
    let jump_request = modal::render(ui, state);
    if let Some(jump_request) = jump_request
        && let issue_id = match &jump_request {
            CompatJumpAction::Auto(id)
            | CompatJumpAction::Affected(id)
            | CompatJumpAction::Related(id) => id.as_str(),
        }
        && let Some(issue) = state
            .compat
            .issues
            .iter()
            .find(|i| i.issue_id == issue_id)
            .cloned()
        && match jump_request {
            CompatJumpAction::Auto(_) => jump::jump_to_compat_issue(state, &issue),
            CompatJumpAction::Affected(_) => jump::jump_to_affected_issue(state, &issue),
            CompatJumpAction::Related(_) => jump::jump_to_related_issue(state, &issue),
        }
    {
        *jump_to_selected_requested = true;
        state.step3.compat_modal_open = false;
    }
}
