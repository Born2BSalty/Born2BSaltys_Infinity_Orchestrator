// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_summary::evaluate_component_prompt_summary;
use crate::app::prompt_popup_text::format_component_prompt_popup_text_with_body;
use crate::app::state::{Step2ComponentState, Step2Selection};
use crate::ui::orchestrator::widgets::{ButtonIcon, render_icon_button};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, redesign_border_strong, redesign_text_disabled, redesign_text_primary,
};
use crate::ui::step2::format_step2::{
    colored_component_widget_text, format_component_row_label,
    format_component_row_label_with_display,
};
use crate::ui::step2::tree_compat_display_step2::compat_colors_redesign as compat_colors;
use crate::ui::step2::tree_component_types_step2::{ComponentRenderState, ComponentRowsContext};
use crate::ui::step2::tree_selection_rules_step2::set_component_checked_state;

#[derive(Clone, Copy)]
pub(crate) struct ComponentRowOptions<'a> {
    pub(crate) display_override: Option<&'a str>,
    pub(crate) indent: f32,
    pub(crate) is_radio_select: bool,
}

#[derive(Clone, Copy)]
struct ComponentRowView<'a> {
    display_label: &'a str,
    effectively_disabled: bool,
    is_selected: bool,
}

pub(crate) fn render_component_row(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    component: &mut Step2ComponentState,
    opts: ComponentRowOptions<'_>,
) {
    let effectively_disabled = component.disabled
        || matches!(
            component.compat_kind.as_deref(),
            Some("mismatch" | "included")
        );
    let display_label = match opts.display_override {
        Some(display) => format_component_row_label_with_display(
            component.raw_line.as_str(),
            component.component_id.as_str(),
            display,
        ),
        None => format_component_row_label(
            component.raw_line.as_str(),
            component.component_id.as_str(),
            component.label.as_str(),
        ),
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(opts.indent);
        render_component_checkbox(
            ui,
            ctx,
            ui_state,
            component_idx,
            component,
            effectively_disabled,
            opts.is_radio_select,
        );
        if effectively_disabled && component.checked {
            component.checked = false;
            component.selected_order = None;
        }
        render_compat_dot(ui, ctx, component);
        let is_selected = matches!(
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
        );
        render_component_label_area(
            ui,
            ctx,
            ui_state,
            component,
            ComponentRowView {
                display_label: display_label.as_str(),
                effectively_disabled,
                is_selected,
            },
        );
    });
}

fn render_component_checkbox(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component_idx: usize,
    component: &mut Step2ComponentState,
    effectively_disabled: bool,
    is_radio_select: bool,
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
            if is_radio_select {
                let clicked =
                    paint_radio_glyph(ui, component.checked, effectively_disabled, ctx.palette);
                if clicked && !effectively_disabled {
                    component.checked = !component.checked;
                }
            } else {
                ui.add_enabled_ui(!effectively_disabled, |ui| {
                    ui.checkbox(&mut component.checked, "");
                });
            }
        },
    );
    if component.checked == was_checked {
        return;
    }
    set_component_checked_state(component, ctx.next_selection_order);
    if component.checked {
        ui_state.enforce_single_select_for.push(component_idx);
        ui_state.enforce_collapsible_group_for.push(component_idx);
        ui_state.enforce_meta_for.push(component_idx);
    }
}

fn paint_radio_glyph(
    ui: &mut egui::Ui,
    checked: bool,
    effectively_disabled: bool,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
) -> bool {
    let icon_size = ui.spacing().icon_width;
    let size = egui::vec2(icon_size, icon_size);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let (ring_color, fill_color) = if effectively_disabled {
        let muted = redesign_text_disabled(palette);
        (muted, muted)
    } else {
        (
            redesign_border_strong(palette),
            redesign_text_primary(palette),
        )
    };
    let center = rect.center();
    let ring_radius = (icon_size * 0.5).min(6.0);
    let painter = ui.painter();
    painter.circle_stroke(
        center,
        ring_radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, ring_color),
    );
    if checked {
        let fill_radius = (ring_radius - 3.0).max(1.5);
        painter.circle_filled(center, fill_radius, fill_color);
    }
    response.clicked()
}

fn render_compat_dot(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    component: &Step2ComponentState,
) {
    if let Some((dot_color, _, _)) = compat_colors(component.compat_kind.as_deref(), ctx.palette) {
        ui.label(crate::ui::shared::typography_global::strong("•").color(dot_color));
    }
}

fn render_component_label_area(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component: &Step2ComponentState,
    view: ComponentRowView<'_>,
) {
    let widget_text = component_widget_text(
        ui,
        view.display_label,
        view.effectively_disabled,
        ctx.palette,
    );
    let row_w = ui.available_width().max(0.0);
    let row_h = ui.spacing().interact_size.y;
    let (row_rect, row_response) =
        ui.allocate_exact_size(egui::vec2(row_w, row_h), egui::Sense::hover());
    let row_hovered = row_response.hovered() || ui.rect_contains_pointer(row_rect);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(row_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_max_width(row_w);
            let mut row = ui.selectable_label(view.is_selected, widget_text);
            if *ctx.jump_to_selected_requested && view.is_selected {
                ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                *ctx.jump_to_selected_requested = false;
            }
            if view.effectively_disabled
                && let Some(reason) = &component.disabled_reason
            {
                row = row.on_hover_text(reason);
            }
            if row.clicked() {
                select_component(ctx, ui_state, component);
            }
            render_compat_pill(ui, ctx, ui_state, component);
            render_prompt_pill(ui, ctx, ui_state, component);
            render_component_details_action(
                ui,
                ctx,
                ui_state,
                component,
                row_hovered || view.is_selected,
            );
        },
    );
}

fn component_widget_text(
    ui: &egui::Ui,
    display_label: &str,
    effectively_disabled: bool,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
) -> egui::WidgetText {
    if effectively_disabled {
        egui::WidgetText::RichText(
            crate::ui::shared::typography_global::strong(display_label).color(
                crate::ui::shared::redesign_tokens::redesign_text_disabled(palette),
            ),
        )
    } else {
        colored_component_widget_text(ui, display_label)
    }
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

fn render_component_details_action(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component: &Step2ComponentState,
    visible: bool,
) {
    let spare = (ui.available_width() - 24.0).max(0.0);
    ui.add_space(spare);
    if render_icon_button(
        ui,
        ctx.palette,
        ButtonIcon::Details,
        "Show details",
        visible,
    )
    .clicked()
    {
        select_component(ctx, ui_state, component);
        *ui_state.open_details = true;
    }
}

fn render_compat_pill(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component: &Step2ComponentState,
) {
    let Some((pill_text_color, pill_bg, pill_label)) =
        compat_colors(component.compat_kind.as_deref(), ctx.palette)
    else {
        return;
    };
    ui.add_space(6.0);
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
            .corner_radius(egui::CornerRadius::same(7))
            .min_size(egui::vec2(0.0, 18.0)),
    );
    if let Some(reason) = &component.disabled_reason {
        pill_response = pill_response.on_hover_text(reason);
    }
    if pill_response.clicked() {
        select_component(ctx, ui_state, component);
        *ui_state.compat_popup = Some((
            ctx.tp_file.to_string(),
            component.component_id.clone(),
            component.raw_line.clone(),
        ));
    }
}

fn render_prompt_pill(
    ui: &mut egui::Ui,
    ctx: &ComponentRowsContext<'_>,
    ui_state: &mut ComponentRenderState<'_>,
    component: &Step2ComponentState,
) {
    let evaluated_prompt_summary = evaluate_component_prompt_summary(component, ctx.prompt_eval);
    if evaluated_prompt_summary.trim().is_empty() {
        return;
    }
    ui.add_space(6.0);
    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
        .color(crate::ui::shared::redesign_tokens::redesign_prompt_text(
            ctx.palette,
        ))
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let prompt_response = ui
        .add(
            egui::Button::new(prompt_text)
                .fill(crate::ui::shared::redesign_tokens::redesign_prompt_fill(
                    ctx.palette,
                ))
                .stroke(egui::Stroke::new(
                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                    crate::ui::shared::redesign_tokens::redesign_prompt_stroke(ctx.palette),
                ))
                .corner_radius(egui::CornerRadius::same(7))
                .min_size(egui::vec2(0.0, 18.0)),
        )
        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
    if prompt_response.clicked() {
        select_component(ctx, ui_state, component);
        *ui_state.prompt_popup = Some((
            format!("{} #{}", ctx.tp_file, component.component_id),
            format_component_prompt_popup_text_with_body(component, &evaluated_prompt_summary),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::step2::tree_selection_rules_step2::set_component_checked_state;

    fn blank_component(id: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    #[test]
    fn radio_click_and_checkbox_click_produce_identical_checked_state() {
        let mut order_radio = 1usize;
        let mut comp_radio = blank_component("C1");
        comp_radio.checked = !comp_radio.checked;
        set_component_checked_state(&mut comp_radio, &mut order_radio);

        let mut order_checkbox = 1usize;
        let mut comp_checkbox = blank_component("C1");
        comp_checkbox.checked = !comp_checkbox.checked;
        set_component_checked_state(&mut comp_checkbox, &mut order_checkbox);

        assert_eq!(
            comp_radio.checked, comp_checkbox.checked,
            "radio and checkbox branches must produce identical checked state after toggle",
        );
        assert_eq!(
            comp_radio.selected_order, comp_checkbox.selected_order,
            "radio and checkbox branches must produce identical selected_order after toggle",
        );
        assert_eq!(
            order_radio, order_checkbox,
            "radio and checkbox branches must advance next_selection_order identically",
        );
    }
}
