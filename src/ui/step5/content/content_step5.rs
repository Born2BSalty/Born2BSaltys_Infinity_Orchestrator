// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::layout_tokens_global::STEP5_SECTION_GAP;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::content_dev_header_step5::render_dev_header;
use crate::ui::step5::content_install_row_step5::render_install_row;
use crate::ui::step5::prompt_answers_step5 as prompt_answers;
use crate::ui::step5::service_diagnostics_support_step5::apply_dev_defaults;
use crate::ui::step5::state_step5::{Step5ConsoleViewState, install_in_progress};
use crate::ui::step5::status_bar_step5 as status_bar;
use crate::ui::step5::top_panels_step5 as top_panels;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step5Action> {
    apply_dev_defaults(state, dev_mode);

    let _running = install_in_progress(state);

    render_dev_header(ui, state, terminal.as_deref(), dev_mode);

    top_panels::render(ui, state);

    let action = render_install_row(
        ui,
        state,
        console_view,
        terminal.as_deref_mut(),
        terminal_error,
        dev_mode,
        exe_fingerprint,
    );

    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_console(
        ui,
        state,
        console_view,
        terminal.as_deref_mut(),
        terminal_error,
    );
    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_status_and_input(ui, state, console_view, terminal.as_deref_mut());
    prompt_answers::render_window(ui, state, terminal.as_deref());

    action
}

pub fn section_gap(ui: &mut egui::Ui, size: f32) {
    ui.add_space(size);
}
