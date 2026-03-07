// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::state_step2::Step2Details;

pub(crate) fn render_selection_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.label(crate::ui::shared::typography_global::small_strong("Selection"));
    egui::Grid::new("step2_details_selection_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            render_value_row(
                ui,
                "Component",
                details.component_label.as_deref(),
                false,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            render_value_row(
                ui,
                "ID",
                details.component_id.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            render_value_row(
                ui,
                "Language",
                details.component_lang.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            render_value_row(
                ui,
                "Version",
                details.component_version.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );
            render_value_row(
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
            render_value_row(
                ui,
                "Order",
                selected_order.as_deref(),
                true,
                label_w,
                value_w,
                row_h,
                value_chars,
            );

            render_checked_row(ui, details, label_w, value_w, row_h);
            render_state_row(ui, details, label_w, value_w, row_h);

            if details.compat_kind.is_some() {
                render_compat_rows(ui, details, label_w, value_w, row_h, value_chars);
            }
        });
}

fn render_value_row(
    ui: &mut egui::Ui,
    label: &str,
    value: Option<&str>,
    monospace: bool,
    label_w: f32,
    value_w: f32,
    row_h: f32,
    value_chars: usize,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let raw = value.unwrap_or("No data");
    let display = ellipsize_end(raw, value_chars);
    let text = if monospace {
        crate::ui::shared::typography_global::monospace(display)
    } else {
        crate::ui::shared::typography_global::plain(display)
    };
    ui.add_sized([value_w, row_h], egui::Label::new(text))
        .on_hover_text(raw);
    if let Some(copy_value) = value {
        if ui
            .small_button("C")
            .on_hover_text(crate::ui::shared::tooltip_global::COPY)
            .clicked()
        {
            ui.ctx().copy_text(copy_value.to_string());
        }
    } else {
        ui.label("");
    }
    ui.end_row();
}

fn render_checked_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("Checked")),
    );
    let checked_pill = match details.is_checked {
        Some(true) => crate::ui::shared::typography_global::strong("Checked")
            .color(crate::ui::shared::theme_global::success()),
        Some(false) => crate::ui::shared::typography_global::strong("Unchecked")
            .color(crate::ui::shared::theme_global::text_muted()),
        None => crate::ui::shared::typography_global::strong("No data"),
    };
    ui.add_sized([value_w, row_h], egui::Label::new(checked_pill));
    ui.label("");
    ui.end_row();
}

fn render_state_row(
    ui: &mut egui::Ui,
    details: &Step2Details,
    label_w: f32,
    value_w: f32,
    row_h: f32,
) {
    ui.add_sized(
        [label_w, row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("State")),
    );
    let state_text = if details.is_disabled == Some(true) {
        crate::ui::shared::typography_global::strong("Disabled")
            .color(crate::ui::shared::theme_global::warning())
    } else {
        crate::ui::shared::typography_global::strong("Selectable")
            .color(crate::ui::shared::theme_global::success())
    };
    let state_resp = ui.add_sized([value_w, row_h], egui::Label::new(state_text));
    if let Some(reason) = details.disabled_reason.as_deref() {
        state_resp.on_hover_text(reason);
    }
    ui.label("");
    ui.end_row();
}

fn render_compat_rows(
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
        egui::Label::new(crate::ui::shared::typography_global::strong("Reason")),
    );
    let reason = details
        .disabled_reason
        .as_deref()
        .unwrap_or("Matched compatibility rule.");
    let display = ellipsize_end(reason, value_chars);
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(
            crate::ui::shared::typography_global::plain(display)
                .color(crate::ui::shared::theme_global::warning()),
        ),
    )
    .on_hover_text(reason);
    if ui
        .small_button("C")
        .on_hover_text(crate::ui::shared::tooltip_global::COPY)
        .clicked()
    {
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
        egui::Label::new(crate::ui::shared::typography_global::strong("Rule Origin")),
    );
    let origin = details
        .compat_source
        .as_deref()
        .unwrap_or("step2_compat_rules.toml");
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(crate::ui::shared::typography_global::monospace(ellipsize_end(
            origin, value_chars,
        ))),
    )
    .on_hover_text(origin);
    if ui
        .small_button("C")
        .on_hover_text(crate::ui::shared::tooltip_global::COPY)
        .clicked()
    {
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
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    ui.add_sized(
        [value_w, row_h],
        egui::Label::new(crate::ui::shared::typography_global::monospace(ellipsize_end(
            raw, value_chars,
        ))),
    )
    .on_hover_text(raw);
    if ui
        .small_button("C")
        .on_hover_text(crate::ui::shared::tooltip_global::COPY)
        .clicked()
    {
        ui.ctx().copy_text(raw.to_string());
    }
    ui.end_row();
}

fn ellipsize_end(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = value.chars().take(keep).collect();
    format!("{prefix}...")
}
