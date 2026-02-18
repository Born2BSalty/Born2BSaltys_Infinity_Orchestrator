// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::Step2Details;

use super::row::render_value_row;
use super::super::super::format::ellipsize_end;

pub(super) fn render_compat_rows(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    render_value_row(
        ui,
        "Role",
        details.compat_role.as_deref(),
        false,
        label_w,
        value_w,
        row_h,
        value_chars,
    );
    render_value_row(
        ui,
        "Issue Code",
        details.compat_code.as_deref(),
        true,
        label_w,
        value_w,
        row_h,
        value_chars,
    );

    render_reason_row(ui, details, label_w, value_w, row_h, value_chars);
    render_origin_row(ui, details, label_w, value_w, row_h, value_chars);
    render_optional_monospace_row(
        ui,
        "Related",
        details.compat_related_target.as_deref(),
        label_w,
        value_w,
        row_h,
        value_chars,
    );
    render_optional_monospace_row(
        ui,
        "Conflict Graph",
        details.compat_graph.as_deref(),
        label_w,
        value_w,
        row_h,
        value_chars,
    );
    render_optional_monospace_row(
        ui,
        "Rule Detail",
        details.compat_evidence.as_deref(),
        label_w,
        value_w,
        row_h,
        value_chars,
    );
}

fn render_reason_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new("Reason").strong()),
    );
    let reason = details
        .disabled_reason
        .as_deref()
        .unwrap_or("Matched compatibility rule.");
    let display = ellipsize_end(reason, value_chars);
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(egui::RichText::new(display).color(egui::Color32::from_rgb(214, 168, 96))),
    )
    .on_hover_text(reason);
    if ui.small_button("C").on_hover_text("Copy").clicked() {
        ui.ctx().copy_text(reason.to_string());
    }
    ui.end_row();
}

fn render_origin_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new("Rule Origin").strong()),
    );
    let origin = details
        .compat_source
        .as_deref()
        .unwrap_or("step2_compat_rules.toml");
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(egui::RichText::new(ellipsize_end(origin, value_chars)).monospace()),
    )
    .on_hover_text(origin);
    if ui.small_button("C").on_hover_text("Copy").clicked() {
        ui.ctx().copy_text(origin.to_string());
    }
    ui.end_row();
}

fn render_optional_monospace_row(
    ui: &mut egui::Ui,
    label: &str,
    value: Option<&str>,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    let Some(raw) = value else {
        return;
    };
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(egui::RichText::new(label).strong()),
    );
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(egui::RichText::new(ellipsize_end(raw, value_chars)).monospace()),
    )
    .on_hover_text(raw);
    if ui.small_button("C").on_hover_text("Copy").clicked() {
        ui.ctx().copy_text(raw.to_string());
    }
    ui.end_row();
}
