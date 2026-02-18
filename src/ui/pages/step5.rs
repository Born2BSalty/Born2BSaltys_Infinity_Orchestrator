// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

mod cancel_flow;
mod dev_header;
mod install_row;
mod install_flow;
mod menus;
mod prompt_answers;
mod status_bar;
mod top_panels;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step5Action {
    CheckCompatBeforeInstall,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
) -> Option<Step5Action> {
    if dev_mode {
        // Dev mode always captures full internal diagnostics.
        state.step1.bio_full_debug = true;
        state.step1.log_raw_output_dev = true;
    }

    dev_header::render(ui, state, dev_mode);

    top_panels::render(ui, state);

    let action = install_row::render(ui, state, terminal.as_deref_mut(), terminal_error, dev_mode);

    ui.add_space(6.0);
    status_bar::render_console(ui, state, terminal.as_deref_mut(), terminal_error);
    ui.add_space(6.0);
    status_bar::render_status_and_input(ui, state, terminal.as_deref_mut());
    prompt_answers::render_window(ui, state, terminal.as_deref());

    action
}
