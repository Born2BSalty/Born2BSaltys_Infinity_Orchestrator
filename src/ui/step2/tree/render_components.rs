// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ModState, Step2Selection};

use super::format::{colored_component_widget_text, format_component_row_label};
use super::render_helpers::{
    compat_colors, enforce_meta_mode_exclusive, enforce_subcomponent_single_select,
    set_component_checked_state,
};

pub(super) fn render_component_rows(
    ui: &mut egui::Ui,
    filter: &str,
    active_tab: &str,
    selected: &Option<Step2Selection>,
    next_selection_order: &mut usize,
    jump_to_selected_requested: &mut bool,
    mod_state: &mut Step2ModState,
) -> (Option<Step2Selection>, Option<(String, String, String)>) {
    let mod_name = mod_state.name.clone();
    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut enforce_single_select_for = Vec::<usize>::new();
    let mut enforce_meta_for = Vec::<usize>::new();

    for (component_idx, component) in mod_state.components.iter_mut().enumerate() {
        let label = component.label.as_str();
        let display_label = format_component_row_label(
            component.raw_line.as_str(),
            component.component_id.as_str(),
            label,
        );
        if filter.is_empty()
            || mod_name.to_lowercase().contains(filter)
            || label.to_lowercase().contains(filter)
        {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(25.0);
                let was_checked = component.checked;
                ui.push_id(
                    (
                        "mod_component_checkbox",
                        &mod_state.tp_file,
                        &mod_state.name,
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
                    set_component_checked_state(component, next_selection_order);
                    if component.checked {
                        enforce_single_select_for.push(component_idx);
                        enforce_meta_for.push(component_idx);
                    }
                }
                if component.disabled && component.checked {
                    component.checked = false;
                    component.selected_order = None;
                }
                let kind = component.compat_kind.as_deref();
                if let Some((dot_color, _, _)) = compat_colors(kind) {
                    ui.label(egui::RichText::new("•").color(dot_color).strong());
                }
                let is_selected = matches!(
                    selected,
                    Some(Step2Selection::Component {
                        game_tab,
                        tp_file,
                        component_id,
                        component_key,
                    }) if game_tab == active_tab
                        && tp_file == &mod_state.tp_file
                        && component_id == &component.component_id
                        && component_key == &component.raw_line
                );
                let widget_text = if component.disabled {
                    egui::WidgetText::RichText(
                        egui::RichText::new(display_label.as_str())
                            .strong()
                            .color(egui::Color32::from_gray(130)),
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
                        if *jump_to_selected_requested && is_selected {
                            ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                            *jump_to_selected_requested = false;
                        }
                        if component.disabled
                            && let Some(reason) = &component.disabled_reason
                        {
                            row = row.on_hover_text(reason);
                        }
                        if row.clicked() {
                            new_selection = Some(Step2Selection::Component {
                                game_tab: active_tab.to_string(),
                                tp_file: mod_state.tp_file.clone(),
                                component_id: component.component_id.clone(),
                                component_key: component.raw_line.clone(),
                            });
                        }
                        if let Some((pill_text_color, pill_bg, pill_label)) = compat {
                            ui.add_space(6.0);
                            let pill_text = egui::RichText::new(pill_label)
                                .color(pill_text_color)
                                .strong()
                                .size(11.0);
                            let mut pill_response = ui.add(
                                egui::Button::new(pill_text)
                                    .fill(pill_bg)
                                    .stroke(egui::Stroke::new(1.0, pill_bg))
                                    .corner_radius(egui::CornerRadius::same(7))
                                    .min_size(egui::vec2(0.0, 18.0)),
                            );
                            if let Some(reason) = &component.disabled_reason {
                                pill_response = pill_response.on_hover_text(reason);
                            }
                            if pill_response.clicked() {
                                new_selection = Some(Step2Selection::Component {
                                    game_tab: active_tab.to_string(),
                                    tp_file: mod_state.tp_file.clone(),
                                    component_id: component.component_id.clone(),
                                    component_key: component.raw_line.clone(),
                                });
                                open_compat_for_component = Some((
                                    mod_state.tp_file.clone(),
                                    component.component_id.clone(),
                                    component.raw_line.clone(),
                                ));
                            }
                        }
                    },
                );
            });
        }
    }
    for idx in enforce_single_select_for {
        enforce_subcomponent_single_select(mod_state, idx);
    }
    for idx in enforce_meta_for {
        enforce_meta_mode_exclusive(mod_state, idx);
    }
    (new_selection, open_compat_for_component)
}
