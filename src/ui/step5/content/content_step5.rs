// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::layout_tokens_global::STEP5_SECTION_GAP;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::content_dev_header_step5::render_dev_header;
use crate::ui::step5::content_install_row_step5::render_install_row;
use crate::ui::step5::page_step5::{Step5RenderOptions, Step5RenderRuntime};
use crate::ui::step5::prompt_answers_step5 as prompt_answers;
use crate::ui::step5::service_diagnostics_support_step5::apply_dev_defaults;
use crate::ui::step5::state_step5::install_in_progress;
use crate::ui::step5::status_bar_step5 as status_bar;
use crate::ui::step5::top_panels_step5 as top_panels;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut runtime: Step5RenderRuntime<'_>,
    options: Step5RenderOptions<'_>,
) -> Option<Step5Action> {
    apply_dev_defaults(state, options.dev_mode);

    let _running = install_in_progress(state);

    render_dev_header(
        ui,
        state,
        runtime.terminal.as_deref(),
        options.dev_mode,
        options.palette,
    );

    top_panels::render(ui, state);

    let action = render_install_row(ui, state, &mut runtime, options);

    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_console(
        ui,
        state,
        runtime.console_view,
        runtime.terminal.as_deref_mut(),
        runtime.terminal_error,
        options.palette,
    );
    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_status_and_input(
        ui,
        state,
        runtime.console_view,
        runtime.terminal.as_deref_mut(),
        options.palette,
    );
    prompt_answers::render_window(ui, state, runtime.terminal.as_deref(), options.palette);

    action
}

pub fn section_gap(ui: &mut egui::Ui, size: f32) {
    ui.add_space(size);
}
