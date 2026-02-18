// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

mod table;
mod top_bar;

pub(super) fn render_button(ui: &mut egui::Ui, state: &mut WizardState) {
    if ui
        .button("Prompt Answers")
        .on_hover_text("Manage saved auto-answer entries.")
        .clicked()
    {
        state.step5.prompt_answers_window_open = true;
    }
}

pub(super) fn render_window(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&EmbeddedTerminal>,
) {
    if !state.step5.prompt_answers_window_open {
        return;
    }

    let mut open = state.step5.prompt_answers_window_open;
    egui::Window::new("Saved Prompt Answers")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(980.0, 430.0))
        .show(ui.ctx(), |ui| {
            top_bar::render(ui, state, terminal);
            ui.separator();
            table::render(ui);
        });

    state.step5.prompt_answers_window_open = open;
}
