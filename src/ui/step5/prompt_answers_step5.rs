// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;

pub(crate) fn render_button(ui: &mut egui::Ui, state: &mut WizardState) {
    if ui
        .button("Prompt Answers")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_PROMPT_ANSWERS)
        .clicked()
    {
        state.step5.prompt_answers_window_open = true;
    }
}

pub(crate) fn render_window(
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
            crate::ui::step5::prompt_answers_top_bar_step5::render_top_bar(ui, state, terminal);
            ui.separator();
            crate::ui::step5::prompt_answers_table_step5::render_table(ui);
        });

    state.step5.prompt_answers_window_open = open;
    if let Some(err) = crate::ui::step5::prompt_memory::take_last_load_error() {
        state.step5.last_status_text = err;
    }
    if let Some(err) = crate::ui::step5::prompt_memory::take_last_persist_error() {
        state.step5.last_status_text = err;
    }
}
