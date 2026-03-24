// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ComponentState, Step2ModState, Step2Selection};
use crate::ui::step2::format_step2::{
    colored_component_widget_text, format_component_row_label,
    format_component_row_label_with_display,
};
use crate::ui::step2::prompt_eval_step2::evaluate_component_prompt_summary;
use crate::ui::step2::state_step2::PromptEvalContext;
use crate::ui::step2::tree_parent_step2::format_component_prompt_popup_text_with_body;
use crate::ui::step2::tree_step2::step2_tree::render_helpers::{
    compat_colors, enforce_meta_mode_exclusive, enforce_subcomponent_single_select,
    set_component_checked_state, split_subcomponent_label,
};

type CompatPopupTarget = Option<(String, String, String)>;
type PromptPopupTarget = Option<(String, String)>;

pub(crate) struct ComponentRowsResult {
    pub selection: Option<Step2Selection>,
    pub compat_popup: CompatPopupTarget,
    pub prompt_popup: PromptPopupTarget,
}

pub(crate) struct ComponentRowsContext<'a> {
    pub filter: &'a str,
    pub active_tab: &'a str,
    pub selected: &'a Option<Step2Selection>,
    pub next_selection_order: &'a mut usize,
    pub prompt_eval: &'a PromptEvalContext,
    pub jump_to_selected_requested: &'a mut bool,
    pub tp_file: &'a str,
    pub mod_name: &'a str,
}

struct ComponentRowUiState<'a> {
    selection: &'a mut Option<Step2Selection>,
    compat_popup: &'a mut CompatPopupTarget,
    prompt_popup: &'a mut PromptPopupTarget,
    enforce_single_select_for: &'a mut Vec<usize>,
    enforce_meta_for: &'a mut Vec<usize>,
}

pub(crate) fn render_component_rows(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    mod_state: &mut Step2ModState,
) -> ComponentRowsResult {
    let mod_name_match = ctx.filter.is_empty() || ctx.mod_name.to_lowercase().contains(ctx.filter);
    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: CompatPopupTarget = None;
    let mut open_prompt_popup: PromptPopupTarget = None;
    let mut enforce_single_select_for = Vec::<usize>::new();
    let mut enforce_meta_for = Vec::<usize>::new();

    let mut component_idx = 0usize;
    while component_idx < mod_state.components.len() {
        let current_label = mod_state.components[component_idx].label.clone();
        let subgroup = split_subcomponent_label(&current_label);
        let mut group_end = component_idx + 1;
        if let Some((header, _)) = subgroup.as_ref() {
            while group_end < mod_state.components.len() {
                let next_label = mod_state.components[group_end].label.clone();
                let Some((next_header, _)) = split_subcomponent_label(&next_label) else {
                    break;
                };
                if !next_header.eq_ignore_ascii_case(header) {
                    break;
                }
                group_end += 1;
            }
        }

        let is_subgroup = subgroup.is_some() && group_end - component_idx > 1;
        if is_subgroup {
            let (header, _) = subgroup.unwrap_or_default();
            let subgroup_matches =
                !ctx.filter.is_empty() && header.to_lowercase().contains(ctx.filter);
            let any_child_visible = (component_idx..group_end).any(|idx| {
                mod_name_match
                    || subgroup_matches
                    || mod_state.components[idx].label.to_lowercase().contains(ctx.filter)
            });
            if any_child_visible {
                egui::CollapsingHeader::new(header.as_str())
                    .id_salt(("step2_subgroup", ctx.tp_file, component_idx, header.as_str()))
                    .default_open(true)
                    .show(ui, |ui| {
                        for idx in component_idx..group_end {
                            let child_choice =
                                split_subcomponent_label(&mod_state.components[idx].label)
                                    .map(|(_, choice)| choice)
                                    .unwrap_or_else(|| mod_state.components[idx].label.clone());
                            let row_visible = mod_name_match
                                || subgroup_matches
                                || mod_state.components[idx]
                                    .label
                                    .to_lowercase()
                                    .contains(ctx.filter);
                            if row_visible {
                                let mut ui_state = ComponentRowUiState {
                                    selection: &mut new_selection,
                                    compat_popup: &mut open_compat_for_component,
                                    prompt_popup: &mut open_prompt_popup,
                                    enforce_single_select_for: &mut enforce_single_select_for,
                                    enforce_meta_for: &mut enforce_meta_for,
                                };
                                render_component_row(
                                    ui,
                                    ctx,
                                    &mut ui_state,
                                    idx,
                                    &mut mod_state.components[idx],
                                    Some(child_choice.as_str()),
                                    45.0,
                                );
                            }
                        }
                    });
            }
            component_idx = group_end;
            continue;
        }

        let label = mod_state.components[component_idx].label.clone();
        let row_visible = mod_name_match || label.to_lowercase().contains(ctx.filter);
        if row_visible {
            let mut ui_state = ComponentRowUiState {
                selection: &mut new_selection,
                compat_popup: &mut open_compat_for_component,
                prompt_popup: &mut open_prompt_popup,
                enforce_single_select_for: &mut enforce_single_select_for,
                enforce_meta_for: &mut enforce_meta_for,
            };
            render_component_row(
                ui,
                ctx,
                &mut ui_state,
                component_idx,
                &mut mod_state.components[component_idx],
                None,
                25.0,
            );
        }
        component_idx += 1;
    }

    for idx in enforce_single_select_for {
        enforce_subcomponent_single_select(mod_state, idx);
    }
    for idx in enforce_meta_for {
        enforce_meta_mode_exclusive(mod_state, idx);
    }

    ComponentRowsResult {
        selection: new_selection,
        compat_popup: open_compat_for_component,
        prompt_popup: open_prompt_popup,
    }
}

fn render_component_row(
    ui: &mut egui::Ui,
    ctx: &mut ComponentRowsContext<'_>,
    ui_state: &mut ComponentRowUiState<'_>,
    component_idx: usize,
    component: &mut Step2ComponentState,
    display_override: Option<&str>,
    indent: f32,
) {
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
                ui.add_enabled_ui(!component.disabled, |ui| {
                    ui.checkbox(&mut component.checked, "");
                });
            },
        );
        if component.checked != was_checked {
            set_component_checked_state(component, ctx.next_selection_order);
            if component.checked {
                ui_state.enforce_single_select_for.push(component_idx);
                ui_state.enforce_meta_for.push(component_idx);
            }
        }
        if component.disabled && component.checked {
            component.checked = false;
            component.selected_order = None;
        }
        if let Some((dot_color, _, _)) = compat_colors(component.compat_kind.as_deref()) {
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
        let widget_text = if component.disabled {
            egui::WidgetText::RichText(
                crate::ui::shared::typography_global::strong(display_label.as_str())
                    .color(crate::ui::shared::theme_global::text_disabled()),
            )
        } else {
            colored_component_widget_text(ui, display_label.as_str())
        };
        let row_w = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(row_w, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(row_w);
                let compat = compat_colors(component.compat_kind.as_deref());
                let mut row = ui.selectable_label(is_selected, widget_text);
                if *ctx.jump_to_selected_requested && is_selected {
                    ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                    *ctx.jump_to_selected_requested = false;
                }
                if component.disabled && let Some(reason) = &component.disabled_reason {
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
                    ui.add_space(6.0);
                    let prompt_text = crate::ui::shared::typography_global::strong("PROMPT")
                        .color(crate::ui::shared::theme_global::prompt_text())
                        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
                    let prompt_response = ui
                        .add(
                            egui::Button::new(prompt_text)
                                .fill(crate::ui::shared::theme_global::prompt_fill())
                                .stroke(egui::Stroke::new(
                                    crate::ui::shared::layout_tokens_global::BORDER_THIN,
                                    crate::ui::shared::theme_global::prompt_stroke(),
                                ))
                                .corner_radius(egui::CornerRadius::same(7))
                                .min_size(egui::vec2(0.0, 18.0)),
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
