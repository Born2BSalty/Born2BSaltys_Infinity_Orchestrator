// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod action_bar;
mod content;
mod format;

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step2::details::selected_details;

use super::Step2Action;

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    right_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
        let details = selected_details(state);
        ui.group(|ui| {
            ui.set_min_size(right_rect.size() - egui::vec2(12.0, 12.0));
            ui.label(egui::RichText::new("Details").strong().size(14.0));
            ui.add_space(4.0);
            action_bar::render(ui, &details, action);
            ui.separator();
            content::render(ui, &details, action);
        });
    });
}
