// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use crate::ui::step3::tabs;
use crate::ui::state::WizardState;
mod compat;
mod list;
mod prompt_setup;
mod toolbar;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step3Action {
    Revalidate,
}

pub fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<Step3Action> {
    let mut action: Option<Step3Action> = None;
    tabs::normalize_active_tab(state);

    ui.heading("Step 3: Reorder Components");
    ui.label("Arrange components into a valid install order.");
    ui.add_space(8.0);

    toolbar::render(ui, state, &mut action);

    ui.add_space(6.0);
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    list::render(ui, state, &mut jump_to_selected_requested);

    if state.step3.compat_modal_open {
        compat::render_modal(ui, state, &mut jump_to_selected_requested);
    }
    if state.step3.prompt_setup_open {
        prompt_setup::render(ui, state);
    }
    state.step3.jump_to_selected_requested = jump_to_selected_requested;

    action
}
