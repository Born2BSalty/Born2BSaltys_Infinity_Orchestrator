// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_summary::evaluate_component_prompt_summary;
use crate::app::prompt_popup_text::format_component_prompt_popup_text_with_body;
use crate::app::state::{Step2ComponentState, Step2Selection};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_PILL_HEIGHT_PX, REDESIGN_BIO_PILL_RADIUS_PX, REDESIGN_BIO_ROW_GAP_PX,
    REDESIGN_BORDER_WIDTH_PX, redesign_prompt_fill, redesign_prompt_stroke, redesign_prompt_text,
    redesign_text_disabled,
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
    let effectively_disabled = component.disabled
        || matches!(
            component.compat_kind.as_deref(),
            Some("mismatch") | Some("included")
        );
    let display_label = match display_override {
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
        ui.add_space(indent);
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
        if let Some((dot_color, _, _)) =
            compat_colors(component.compat_kind.as_deref(), ctx.palette)
        {
            ui.label(crate::ui::shared::typography_global::strong("•").color(dot_color));
        }
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
        let widget_text = if effectively_disabled {
            egui::WidgetText::RichText(
                crate::ui::shared::typography_global::strong(display_label.as_str())
                    .color(redesign_text_disabled(ctx.palette)),
            )
        } else {
            colored_component_widget_text(ui, display_label.as_str(), ctx.palette)
        };
        let row_w = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(row_w, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(row_w);
                let compat = compat_colors(component.compat_kind.as_deref(), ctx.palette);
                let mut row = ui.selectable_label(is_selected, widget_text);
                if *ctx.jump_to_selected_requested && is_selected {
                    ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                    *ctx.jump_to_selected_requested = false;
                }
                if effectively_disabled && let Some(reason) = &component.disabled_reason {
                    row = row.on_hover_text(reason);
                }
                if row.clicked() {
                    *ui_state.selection = Some(Step2Selection::Component {
                        game_tab: ctx.active_tab.to_string(),
                        tp_file: ctx.tp_file.to_string(),
                        component_id: component.component_id.clone(),
                        component_key: component.raw_line.clone(),
                    });
                }
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
                            .corner_radius(egui::CornerRadius::same(
                                REDESIGN_BIO_PILL_RADIUS_PX as u8,
                            ))
                            .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
                    );
                    if let Some(reason) = &component.disabled_reason {
                        pill_response = pill_response.on_hover_text(reason);
                    }
                    if pill_response.clicked() {
                        *ui_state.selection = Some(Step2Selection::Component {
                            game_tab: ctx.active_tab.to_string(),
                            tp_file: ctx.tp_file.to_string(),
                            component_id: component.component_id.clone(),
                            component_key: component.raw_line.clone(),
                        });
                        *ui_state.compat_popup = Some((
                            ctx.tp_file.to_string(),
                            component.component_id.clone(),
                            component.raw_line.clone(),
                        ));
                    }
                }
                let evaluated_prompt_summary =
                    evaluate_component_prompt_summary(component, ctx.prompt_eval);
                if !evaluated_prompt_summary.trim().is_empty() {
                    ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
                    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
                        .color(redesign_prompt_text(ctx.palette))
                        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
                    let prompt_response = ui
                        .add(
                            egui::Button::new(prompt_text)
                                .fill(redesign_prompt_fill(ctx.palette))
                                .stroke(egui::Stroke::new(
                                    REDESIGN_BORDER_WIDTH_PX,
                                    redesign_prompt_stroke(ctx.palette),
                                ))
                                .corner_radius(egui::CornerRadius::same(
                                    REDESIGN_BIO_PILL_RADIUS_PX as u8,
                                ))
                                .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
                        )
                        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
                    if prompt_response.clicked() {
                        *ui_state.selection = Some(Step2Selection::Component {
                            game_tab: ctx.active_tab.to_string(),
                            tp_file: ctx.tp_file.to_string(),
                            component_id: component.component_id.clone(),
                            component_key: component.raw_line.clone(),
                        });
                        *ui_state.prompt_popup = Some((
                            format!("{} #{}", ctx.tp_file, component.component_id),
                            format_component_prompt_popup_text_with_body(
                                component,
                                &evaluated_prompt_summary,
                            ),
                        ));
                    }
                }
            },
        );
    });
}
