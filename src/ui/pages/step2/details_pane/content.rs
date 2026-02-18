// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod paths;
mod raw;
mod selection;

use eframe::egui;

use crate::ui::step2::details::Step2Details;

use super::Step2Action;

pub(super) fn render(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
) {
    egui::ScrollArea::vertical()
        .id_salt("step2_details_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if let Some(mod_name) = &details.mod_name {
                render_details_content(ui, mod_name, details, action);
            } else {
                ui.label("Select an item to view details.");
            }
        });
}

fn render_details_content(
    ui: &mut egui::Ui,
    mod_name: &str,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
) {
    let label_w = 86.0;
    let action_w = 48.0;
    let value_w = (ui.available_width() - label_w - action_w - 24.0).max(120.0);
    let row_h = 20.0;
    let value_chars = ((value_w / 7.2).floor() as usize).max(12);

    ui.label(egui::RichText::new(mod_name).strong());
    ui.add_space(4.0);

    selection::render_selection_grid(ui, details, label_w, value_w, row_h, value_chars);
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(4.0);
    paths::render_paths_grid(ui, details, action, label_w, value_w, row_h, value_chars);
    ui.add_space(6.0);
    raw::render_raw_line(ui, details);
}
