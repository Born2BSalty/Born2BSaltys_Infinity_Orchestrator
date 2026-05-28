// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::layout_tokens_global::STEP5_SECTION_GAP;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::content_dev_header_step5::render_dev_header;
use crate::ui::step5::content_install_row_step5::render_install_row;
use crate::ui::step5::prompt_answers_step5 as prompt_answers;
use crate::ui::step5::service_diagnostics_support_step5::apply_dev_defaults;
use crate::ui::step5::state_step5::{Step5ConsoleViewState, install_in_progress};
use crate::ui::step5::status_bar_step5 as status_bar;
use crate::ui::step5::top_panels_step5 as top_panels;

/// Render-time configuration threaded from the caller into the Step 5 sub-renderers.
#[derive(Clone, Copy)]
pub struct Step5RenderCtx<'a> {
    pub dev_mode: bool,
    pub exe_fingerprint: &'a str,
    pub palette: ThemePalette,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    ctx: Step5RenderCtx<'_>,
) -> Option<Step5Action> {
    apply_dev_defaults(state, ctx.dev_mode);

    let _running = install_in_progress(state);

    render_dev_header(ui, state, terminal.as_deref(), ctx.dev_mode, ctx.palette);

    top_panels::render(ui, state, ctx.palette);

    let action = render_install_row(
        ui,
        state,
        console_view,
        terminal.as_deref_mut(),
        terminal_error,
        ctx,
    );

    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_console(
        ui,
        state,
        console_view,
        terminal.as_deref_mut(),
        terminal_error,
        ctx.palette,
    );
    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_status_and_input(
        ui,
        state,
        console_view,
        terminal.as_deref_mut(),
        ctx.palette,
    );
    if ctx.dev_mode {
        prompt_answers::render_window(ui, state, terminal.as_deref());
    }

    action
}

pub fn section_gap(ui: &mut egui::Ui, size: f32) {
    ui.add_space(size);
}
