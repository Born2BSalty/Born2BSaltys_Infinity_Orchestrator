// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_summary::evaluate_component_prompt_summary;
use crate::app::prompt_popup_text::format_component_prompt_popup_text_with_body;
use crate::app::state::{Step2ComponentState, Step2Selection};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_PILL_HEIGHT_PX, REDESIGN_BIO_PILL_RADIUS_U8, REDESIGN_BIO_ROW_GAP_PX,
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_BOX_LABEL_FONT_SIZE_PX, redesign_font_mono,
    redesign_prompt_fill, redesign_prompt_stroke, redesign_prompt_text, redesign_text_disabled,
    redesign_text_faint,
};
use crate::ui::step2::format_step2::{
    colored_component_widget_text, format_component_row_label,
    format_component_row_label_with_display,
};
use crate::ui::step2::tree_compat_display_step2::compat_colors;
use crate::ui::step2::tree_component_types_step2::{ComponentRenderState, ComponentRowsContext};
use crate::ui::step2::tree_selection_rules_step2::set_component_checked_state;

pub(crate) fn render_component_row(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    component: &mut Step2ComponentState,
    display_override: Option<&str>,
    indent: f32,
) {
    let effectively_disabled = component_effectively_disabled(component);
    let display_label = component_display_label(component, display_override);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(indent);
        render_component_checkbox(
            ui,
            ctx,
            ui_state,
            component_idx,
            component,
            effectively_disabled,
        );
        render_selected_order_label(ui, ctx, component);
        render_component_compat_marker(ui, ctx, component);
        let is_selected = component_is_selected(ctx, component);
        let widget_text = component_widget_text(ctx, &display_label, effectively_disabled);
        render_component_row_body(
            ui,
            ComponentRowBody {
                ctx,
                ui_state,
                component,
                widget_text: Some(widget_text),
                effectively_disabled,
                is_selected,
            },
        );
    });
}

fn component_effectively_disabled(component: &Step2ComponentState) -> bool {
    component.disabled
        || matches!(
            component.compat_kind.as_deref(),
            Some("mismatch" | "included")
        )
}

fn component_display_label(
    component: &Step2ComponentState,
    display_override: Option<&str>,
) -> String {
    display_override.map_or_else(
        || {
            format_component_row_label(
                component.raw_line.as_str(),
                component.component_id.as_str(),
                component.label.as_str(),
            )
        },
        |display| {
            format_component_row_label_with_display(
                component.raw_line.as_str(),
                component.component_id.as_str(),
                display,
            )
        },
    )
}

fn render_component_checkbox(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    component: &mut Step2ComponentState,
    effectively_disabled: bool,
) {
    let was_checked = component.checked;
    ui.push_id(
        (
            "mod_component_checkbox",
            ctx.tp_file,
            ctx.mod_name,
            &component.component_id,
            component_idx,
        ),
        |ui| {
            ui.add_enabled_ui(!effectively_disabled, |ui| {
                ui.checkbox(&mut component.checked, "");
            });
        },
    );
    if component.checked != was_checked {
        set_component_checked_state(component, ctx.next_selection_order);
        if component.checked {
            ui_state.enforce_single_select_for.push(component_idx);
            ui_state.enforce_collapsible_group_for.push(component_idx);
            ui_state.enforce_meta_for.push(component_idx);
        }
    }
    if effectively_disabled && component.checked {
        component.checked = false;
        component.selected_order = None;
    }
}

fn render_selected_order_label(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    component: &Step2ComponentState,
) {
    if component.checked
        && let Some(order) = component.selected_order
    {
        ui.label(
            egui::RichText::new(format!("#{order:02}"))
                .size(REDESIGN_BOX_LABEL_FONT_SIZE_PX)
                .family(redesign_font_mono())
                .color(redesign_text_faint(ctx.palette)),
        );
    }
}

fn render_component_compat_marker(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    component: &Step2ComponentState,
) {
    if let Some((dot_color, _, _)) = compat_colors(component.compat_kind.as_deref(), ctx.palette) {
        ui.label(crate::ui::shared::typography_global::strong("•").color(dot_color));
    }
}

fn component_is_selected(ctx: &ComponentRowsContext<'_>, component: &Step2ComponentState) -> bool {
    matches!(
        ctx.selected,
        Some(Step2Selection::Component {
            game_tab,
            tp_file: selected_tp,
            component_id,
            component_key,
        }) if game_tab == ctx.active_tab
            && selected_tp == ctx.tp_file
            && component_id == &component.component_id
            && component_key == &component.raw_line
    )
}

fn component_widget_text(
    ctx: &ComponentRowsContext<'_>,
    display_label: &str,
    effectively_disabled: bool,
) -> egui::WidgetText {
    if effectively_disabled {
        egui::WidgetText::RichText(
            crate::ui::shared::typography_global::strong(display_label)
                .color(redesign_text_disabled(ctx.palette)),
        )
    } else {
        colored_component_widget_text(display_label, ctx.filter, ctx.palette)
    }
}

struct ComponentRowBody<'ctx, 'ctx_data, 'state, 'state_data, 'component> {
    ctx: &'ctx mut ComponentRowsContext<'ctx_data>,
    ui_state: &'state mut ComponentRenderState<'state_data>,
    component: &'component mut Step2ComponentState,
    widget_text: Option<egui::WidgetText>,
    effectively_disabled: bool,
    is_selected: bool,
}

fn render_component_row_body(ui: &mut egui::Ui, mut body: ComponentRowBody<'_, '_, '_, '_, '_>) {
    let row_w = ui.available_width().max(0.0);
    ui.allocate_ui_with_layout(
        egui::vec2(row_w, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_max_width(row_w);
            let Some(widget_text) = body.widget_text.take() else {
                return;
            };
            let compat = compat_colors(body.component.compat_kind.as_deref(), body.ctx.palette);
            let mut row = ui.selectable_label(body.is_selected, widget_text);
            if *body.ctx.jump_to_selected_requested && body.is_selected {
                ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                *body.ctx.jump_to_selected_requested = false;
            }
            if body.effectively_disabled
                && let Some(reason) = &body.component.disabled_reason
            {
                row = row.on_hover_text(reason);
            }
            if row.clicked() {
                select_component(body.ctx, body.ui_state, body.component);
            }
            render_component_compat_pill(ui, &mut body, compat);
            render_component_prompt_pill(ui, &mut body);
        },
    );
}

fn select_component(
    ctx: &ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component: &Step2ComponentState,
) {
    *ui_state.selection = Some(Step2Selection::Component {
        game_tab: ctx.active_tab.to_string(),
        tp_file: ctx.tp_file.to_string(),
        component_id: component.component_id.clone(),
        component_key: component.raw_line.clone(),
    });
}

fn render_component_compat_pill(
    ui: &mut egui::Ui,
    body: &mut ComponentRowBody<'_, '_, '_, '_, '_>,
    compat: Option<(egui::Color32, egui::Color32, &'static str)>,
) {
    if let Some((pill_text_color, pill_bg, pill_label)) = compat {
        ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
        let pill_text = crate::ui::shared::typography_global::strong(pill_label)
            .color(pill_text_color)
            .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
        let mut pill_response = ui.add(
            egui::Button::new(pill_text)
                .fill(pill_bg)
                .stroke(egui::Stroke::new(
                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                    pill_bg,
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_U8))
                .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
        );
        if let Some(reason) = &body.component.disabled_reason {
            pill_response = pill_response.on_hover_text(reason);
        }
        if pill_response.clicked() {
            select_component(body.ctx, body.ui_state, body.component);
            *body.ui_state.compat_popup = Some((
                body.ctx.tp_file.to_string(),
                body.component.component_id.clone(),
                body.component.raw_line.clone(),
            ));
        }
    }
}

fn render_component_prompt_pill(
    ui: &mut egui::Ui,
    body: &mut ComponentRowBody<'_, '_, '_, '_, '_>,
) {
    let evaluated_prompt_summary =
        evaluate_component_prompt_summary(body.component, body.ctx.prompt_eval);
    if evaluated_prompt_summary.trim().is_empty() {
        return;
    }
    ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
        .color(redesign_prompt_text(body.ctx.palette))
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let prompt_response = ui
        .add(
            egui::Button::new(prompt_text)
                .fill(redesign_prompt_fill(body.ctx.palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_prompt_stroke(body.ctx.palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_U8))
                .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
        )
        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
    if prompt_response.clicked() {
        select_component(body.ctx, body.ui_state, body.component);
        *body.ui_state.prompt_popup = Some((
            format!("{} #{}", body.ctx.tp_file, body.component.component_id),
            format_component_prompt_popup_text_with_body(body.component, &evaluated_prompt_summary),
        ));
    }
}
