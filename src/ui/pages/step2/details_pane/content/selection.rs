// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

mod compat;
mod row;
mod status;

pub(super) fn render_selection_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.label(egui::RichText::new("Selection").small().strong());
    egui::Grid::new("step2_details_selection_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            row::render_value_row(
                ui,
                "Component",
                details.component_label.as_deref(),
                false,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            row::render_value_row(
                ui,
                "ID",
                details.component_id.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            row::render_value_row(
                ui,
                "Language",
                details.component_lang.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            row::render_value_row(
                ui,
                "Version",
                details.component_version.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            row::render_value_row(
                ui,
                "TP2 File",
                details.tp_file.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            let selected_order = details.selected_order.map(|n| n.to_string());
            row::render_value_row(
                ui,
                "Order",
                selected_order.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );

            status::render_checked_row(ui, details, label_w, value_w, row_h);
            status::render_state_row(ui, details, label_w, value_w, row_h);

            if details.compat_kind.is_some() {
                compat::render_compat_rows(ui, details, label_w, value_w, row_h, value_chars);
            }
        });
}
