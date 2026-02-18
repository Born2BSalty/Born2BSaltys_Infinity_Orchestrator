// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::{Step2ModState, Step2Selection};

use super::render_helpers::{parent_compat_summary, set_component_checked_state};

pub(super) struct ParentRowResult {
    pub selection: Option<Step2Selection>,
    pub open_compat_for_component: Option<(String, String, String)>,
}

pub(super) fn render_parent_row(
    ui: &mut egui::Ui,
    mod_state: &mut Step2ModState,
    active_tab: &str,
    selected: &Option<Step2Selection>,
    next_selection_order: &mut usize,
    jump_to_selected_requested: &mut bool,
) -> ParentRowResult {
    let mod_name = mod_state.name.clone();
    let parent_summary = parent_compat_summary(mod_state);
    let enabled_count = mod_state.components.iter().filter(|c| !c.disabled).count();
    let all_selected = enabled_count > 0
        && mod_state
            .components
            .iter()
            .filter(|component| !component.disabled)
            .all(|component| component.checked);
    let any_selected = mod_state
        .components
        .iter()
        .filter(|component| !component.disabled)
        .any(|component| component.checked);
    let set_value = !all_selected;

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut parent_checked = all_selected;
    let mut checkbox = egui::Checkbox::new(&mut parent_checked, "");
    if any_selected && !all_selected {
        checkbox = checkbox.indeterminate(true);
    }

    ui.horizontal(|ui| {
        let parent_clicked = ui
            .push_id(
                (
                    "mod_parent_checkbox",
                    &mod_state.tp_file,
                    &mod_state.name,
                    &mod_state.tp2_path,
                ),
                |ui| {
                    ui.add_enabled_ui(enabled_count > 0, |ui| ui.add(checkbox).clicked())
                        .inner
                },
            )
            .inner;
        if parent_clicked {
            for component in &mut mod_state.components {
                if component.disabled {
                    continue;
                }
                component.checked = set_value;
                set_component_checked_state(component, next_selection_order);
            }
            mod_state.checked = enabled_count > 0
                && mod_state
                    .components
                    .iter()
                    .filter(|component| !component.disabled)
                    .all(|component| component.checked);
        }
        let is_selected = matches!(
            selected,
            Some(Step2Selection::Mod { game_tab, tp_file })
                if game_tab == active_tab && tp_file == &mod_state.tp_file
        );
        let row_w = ui.available_width().max(0.0);
        ui.allocate_ui_with_layout(
            egui::vec2(row_w, ui.spacing().interact_size.y),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_max_width(row_w);
                let row = ui.selectable_label(is_selected, mod_name.as_str());
                if *jump_to_selected_requested && is_selected {
                    ui.scroll_to_rect(row.rect, Some(egui::Align::Center));
                    *jump_to_selected_requested = false;
                }
                if row.clicked() {
                    new_selection = Some(Step2Selection::Mod {
                        game_tab: active_tab.to_string(),
                        tp_file: mod_state.tp_file.clone(),
                    });
                }
                if let Some((text_color, bg, label)) = &parent_summary {
                    ui.add_space(6.0);
                    let resp = ui.add(
                        egui::Button::new(
                            egui::RichText::new(label)
                                .color(*text_color)
                                .strong()
                                .size(11.0),
                        )
                        .fill(*bg)
                        .stroke(egui::Stroke::new(1.0, *bg))
                        .corner_radius(egui::CornerRadius::same(7))
                        .min_size(egui::vec2(0.0, 18.0)),
                    );
                    if resp.clicked()
                        && let Some(first_compat) = mod_state
                            .components
                            .iter()
                            .find(|c| c.compat_kind.is_some())
                    {
                        open_compat_for_component =
                            Some((
                                mod_state.tp_file.clone(),
                                first_compat.component_id.clone(),
                                first_compat.raw_line.clone(),
                            ));
                    }
                }
            },
        );
    });
    ParentRowResult {
        selection: new_selection,
        open_compat_for_component,
    }
}
