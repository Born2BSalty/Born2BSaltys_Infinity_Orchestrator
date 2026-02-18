// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod auto_answer;
mod console;
mod input_row;
mod phase;
mod process_line;
mod timefmt;

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render_console(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
) {
    console::render_console(ui, state, terminal, terminal_error);
}

pub(super) fn render_status_and_input(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
) {
    let waiting_for_input = terminal
        .as_deref()
        .map(|t| t.likely_input_needed_visible() || t.current_prompt_info().is_some())
        .unwrap_or(false);
    auto_answer::try_auto_answer_prompt(state, terminal.as_deref_mut(), waiting_for_input);
    let phase_info = phase::compute_phase(state, waiting_for_input);

    // Chat bar directly under console frame.
    ui.horizontal(|ui| {
        input_row::render_input(ui, state, terminal.as_deref_mut());
    });

    // Single-line phase + runtime row.
    ui.horizontal_centered(|ui| {
        phase::render_phase(ui, state, &phase_info);
        process_line::render_process_runtime_inline(ui, state, terminal.as_deref());
        process_line::render_error_copy(ui, state, terminal.as_deref());
    });

    // Show detailed status text in its own wrapped row so it never pushes controls outside layout.
    if !state.step5.last_status_text.trim().is_empty() {
        let status = state.step5.last_status_text.clone();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let width = ui.available_width().max(320.0);
            ui.add_sized(
                [width, 0.0],
                egui::Label::new(egui::RichText::new(status).weak()),
            );
        });
    }
}
