// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{ButtonIcon, render_icon_button};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::Step2Details;

#[derive(Clone, Copy)]
pub(crate) struct SelectionGridLayout {
    pub(crate) palette: ThemePalette,
    pub(crate) label_w: f32,
    pub(crate) value_w: f32,
    pub(crate) action_w: f32,
    pub(crate) row_h: f32,
    pub(crate) value_chars: usize,
}

pub(crate) fn render_selection_grid(
    ui: &mut egui::Ui,
    details: &Step2Details,
    action: &mut Option<Step2Action>,
    layout: SelectionGridLayout,
) {
    ui.label(crate::ui::shared::typography_global::small_strong(
        "Selection",
    ));
    egui::Grid::new("step2_details_selection_grid")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            render_value_row(
                ui,
                &layout,
                "Component",
                details.component_label.as_deref(),
                false,
                None,
                action,
            );
            render_value_row(
                ui,
                &layout,
                "ID",
                details.component_id.as_deref(),
                true,
                None,
                action,
            );
            render_checked_row(ui, details, &layout);
            render_state_row(ui, details, &layout);
            if details.compat_kind.is_some() {
                render_compat_rows(ui, details, layout);
            }
            render_value_row(
                ui,
                &layout,
                "Language",
                details.component_lang.as_deref(),
                true,
                None,
                action,
            );
            render_value_row(
                ui,
                &layout,
                "TP2 File",
                details.tp_file.as_deref(),
                true,
                details.tp2_path.clone().map(Step2Action::OpenSelectedTp2),
                action,
            );
            let shown_count = details.shown_component_count.map(|n| n.to_string());
            render_value_row(
                ui,
                &layout,
                "Shown",
                shown_count.as_deref(),
                true,
                None,
                action,
            );
            let hidden_count = details.hidden_component_count.map(|n| n.to_string());
            render_value_row(
                ui,
                &layout,
                "Hidden",
                hidden_count.as_deref(),
                true,
                None,
                action,
            );
            let raw_count = details.raw_component_count.map(|n| n.to_string());
            render_value_row(ui, &layout, "Raw", raw_count.as_deref(), true, None, action);
            let selected_order = details.selected_order.map(|n| n.to_string());
            render_value_row(
                ui,
                &layout,
                "Order",
                selected_order.as_deref(),
                true,
                None,
                action,
            );
        });
}

fn render_value_row(
    ui: &mut egui::Ui,
    layout: &SelectionGridLayout,
    label: &str,
    value: Option<&str>,
    monospace: bool,
    open_action: Option<Step2Action>,
    action: &mut Option<Step2Action>,
) {
    let Some(raw) = value else {
        return;
    };
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let display = ellipsize_end(raw, layout.value_chars);
    let text = if monospace {
        crate::ui::shared::typography_global::monospace(display)
    } else {
        crate::ui::shared::typography_global::plain(display)
    };
    let value_resp = ui
        .add_sized([layout.value_w, layout.row_h], egui::Label::new(text))
        .on_hover_text(raw);
    let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
    render_action_cell(ui, layout, Some(raw), open_action, action, visible);
    ui.end_row();
}

fn render_checked_row(ui: &mut egui::Ui, details: &Step2Details, layout: &SelectionGridLayout) {
    let Some(checked) = details.is_checked else {
        return;
    };
    ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("Checked")),
    );
    let checked_pill = if checked {
        crate::ui::shared::typography_global::strong("Checked")
            .color(crate::ui::shared::theme_global::success())
    } else {
        crate::ui::shared::typography_global::strong("Unchecked")
            .color(crate::ui::shared::theme_global::text_muted())
    };
    ui.add_sized(
        [layout.value_w, layout.row_h],
        egui::Label::new(checked_pill),
    );
    render_empty_action_cell(ui, layout);
    ui.end_row();
}

fn render_state_row(ui: &mut egui::Ui, details: &Step2Details, layout: &SelectionGridLayout) {
    let Some(is_disabled) = details.is_disabled else {
        return;
    };
    ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("State")),
    );
    let state_text = if is_disabled {
        crate::ui::shared::typography_global::strong("Disabled")
            .color(crate::ui::shared::theme_global::warning())
    } else {
        crate::ui::shared::typography_global::strong("Selectable")
            .color(crate::ui::shared::theme_global::success())
    };
    let state_resp = ui.add_sized([layout.value_w, layout.row_h], egui::Label::new(state_text));
    if let Some(reason) = details.disabled_reason.as_deref() {
        state_resp.on_hover_text(reason);
    }
    render_empty_action_cell(ui, layout);
    ui.end_row();
}

fn render_compat_rows(ui: &mut egui::Ui, details: &Step2Details, layout: SelectionGridLayout) {
    let mut ignored_action = None;
    render_value_row(
        ui,
        &layout,
        "Source Type",
        details.compat_role.as_deref(),
        false,
        None,
        &mut ignored_action,
    );
    render_value_row(
        ui,
        &layout,
        "Issue",
        details.compat_code.as_deref(),
        true,
        None,
        &mut ignored_action,
    );

    render_reason_row(ui, details, &layout);
    render_origin_row(ui, details, &layout);
    render_optional_monospace_row(
        ui,
        "Related",
        details.compat_related_target.as_deref(),
        &layout,
    );
    render_optional_monospace_row(
        ui,
        "Conflict Graph",
        details.compat_graph.as_deref(),
        &layout,
    );
    render_optional_monospace_row(
        ui,
        "Matched Rule",
        details.compat_evidence.as_deref(),
        &layout,
    );
}

fn render_reason_row(ui: &mut egui::Ui, details: &Step2Details, layout: &SelectionGridLayout) {
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("Reason")),
    );
    let reason = details
        .disabled_reason
        .as_deref()
        .unwrap_or("Matched compatibility rule.");
    let display = ellipsize_end(reason, layout.value_chars);
    let value_resp = ui
        .add_sized(
            [layout.value_w, layout.row_h],
            egui::Label::new(
                crate::ui::shared::typography_global::plain(display)
                    .color(crate::ui::shared::theme_global::warning()),
            ),
        )
        .on_hover_text(reason);
    let mut ignored_action = None;
    let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
    render_action_cell(ui, layout, Some(reason), None, &mut ignored_action, visible);
    ui.end_row();
}

fn render_origin_row(ui: &mut egui::Ui, details: &Step2Details, layout: &SelectionGridLayout) {
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong("Rule Origin")),
    );
    let origin = details
        .compat_source
        .as_deref()
        .unwrap_or("step2_compat_rules_user.toml");
    let value_resp = ui
        .add_sized(
            [layout.value_w, layout.row_h],
            egui::Label::new(crate::ui::shared::typography_global::monospace(
                ellipsize_end(origin, layout.value_chars),
            )),
        )
        .on_hover_text(origin);
    let mut ignored_action = None;
    let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
    render_action_cell(ui, layout, Some(origin), None, &mut ignored_action, visible);
    ui.end_row();
}

fn render_optional_monospace_row(
    ui: &mut egui::Ui,
    label: &str,
    value: Option<&str>,
    layout: &SelectionGridLayout,
) {
    let Some(raw) = value else {
        return;
    };
    let label_resp = ui.add_sized(
        [layout.label_w, layout.row_h],
        egui::Label::new(crate::ui::shared::typography_global::strong(label)),
    );
    let value_resp = ui
        .add_sized(
            [layout.value_w, layout.row_h],
            egui::Label::new(crate::ui::shared::typography_global::monospace(
                ellipsize_end(raw, layout.value_chars),
            )),
        )
        .on_hover_text(raw);
    let mut ignored_action = None;
    let visible = row_action_visible(ui, label_resp.rect, value_resp.rect, layout.action_w);
    render_action_cell(ui, layout, Some(raw), None, &mut ignored_action, visible);
    ui.end_row();
}

fn render_action_cell(
    ui: &mut egui::Ui,
    layout: &SelectionGridLayout,
    copy_value: Option<&str>,
    open_action: Option<Step2Action>,
    action: &mut Option<Step2Action>,
    visible: bool,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(layout.action_w, layout.row_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            if let Some(next_action) = open_action
                && render_icon_button(
                    ui,
                    layout.palette,
                    ButtonIcon::Open,
                    crate::ui::shared::tooltip_global::OPEN,
                    visible,
                )
                .clicked()
            {
                *action = Some(next_action);
            }
            if let Some(value) = copy_value
                && render_icon_button(
                    ui,
                    layout.palette,
                    ButtonIcon::Copy,
                    crate::ui::shared::tooltip_global::COPY,
                    visible,
                )
                .clicked()
            {
                ui.ctx().copy_text(value.to_string());
            }
        },
    );
}

fn render_empty_action_cell(ui: &mut egui::Ui, layout: &SelectionGridLayout) {
    ui.allocate_ui(egui::vec2(layout.action_w, layout.row_h), |_| {});
}

fn row_action_visible(
    ui: &egui::Ui,
    label_rect: egui::Rect,
    value_rect: egui::Rect,
    action_w: f32,
) -> bool {
    let hover_rect = egui::Rect::from_min_max(
        label_rect.min,
        egui::pos2(value_rect.right() + action_w + 8.0, value_rect.bottom()),
    )
    .expand(2.0);
    ui.rect_contains_pointer(hover_rect)
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
