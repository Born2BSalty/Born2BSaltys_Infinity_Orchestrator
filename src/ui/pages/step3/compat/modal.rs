// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use super::model::CompatJumpAction;

mod filter_row;
mod footer;
mod filters;
mod header;
mod issues;
mod issue_text;
mod report;
mod target;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<CompatJumpAction> {
    let mut jump_request: Option<CompatJumpAction> = None;
    let mut open = state.step3.compat_modal_open;
    egui::Window::new("Compatibility Issues")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(760.0, 480.0))
        .min_width(260.0)
        .min_height(140.0)
        .show(ui.ctx(), |ui| {
            header::render_header(ui, state);
            ui.add_space(6.0);
            filter_row::render_filter_row(ui, state);
            ui.add_space(6.0);
            issues::render_issue_list(ui, state, &mut jump_request);
            ui.add_space(6.0);
            footer::render_footer(ui, state);
        });
    state.step3.compat_modal_open = open && state.step3.compat_modal_open;
    jump_request
}
