// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_popup_text::{build_mod_prompt_popup_text, mod_has_any_prompt};
use crate::app::state::{Step2ModState, Step2Selection};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_PILL_HEIGHT_PX, REDESIGN_BIO_PILL_RADIUS_PX, REDESIGN_BIO_ROW_GAP_PX,
    REDESIGN_BORDER_WIDTH_PX, redesign_prompt_fill, redesign_prompt_stroke, redesign_prompt_text,
};
use crate::ui::step2::tree_compat_display_step2::{parent_compat_summary, parent_compat_target};
use crate::ui::step2::tree_component_types_step2::ComponentRowsContext;
use crate::ui::step2::tree_selection_rules_step2::{
    enforce_collapsible_group_umbrella_after_bulk, enforce_meta_mode_after_bulk,
    enforce_subcomponent_single_select_keep_first, enforce_tp2_same_mod_exclusive_after_bulk,
    set_component_checked_state,
};

pub(crate) struct ParentRowResult {
    pub selection: Option<Step2Selection>,
    pub open_compat_for_component: Option<(String, String, String)>,
    pub open_prompt_popup: Option<(String, String)>,
}

pub(crate) fn render_parent_row(
    ui: &mut egui::Ui,
    mod_state: &mut Step2ModState,
    ctx: &mut ComponentRowsContext<'_>,
) -> ParentRowResult {
    let active_tab = ctx.active_tab;
    let selected = ctx.selected;
    let next_selection_order = &mut *ctx.next_selection_order;
    let prompt_eval = ctx.prompt_eval;
    let jump_to_selected_requested = &mut *ctx.jump_to_selected_requested;
    let palette = ctx.palette;
    let mod_name = mod_state.name.clone();
    let mod_visible_count = mod_state.components.len();
    let selected_visible_count = mod_state
        .components
        .iter()
        .filter(|component| component.checked)
        .count();
    let mod_header_label = format!("{mod_name} ({selected_visible_count}/{mod_visible_count})");
    let parent_summary = parent_compat_summary(mod_state, palette);
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
    let set_value = !any_selected;

    let mut new_selection: Option<Step2Selection> = None;
    let mut open_compat_for_component: Option<(String, String, String)> = None;
    let mut open_prompt_popup: Option<(String, String)> = None;
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
            if set_value {
                enforce_subcomponent_single_select_keep_first(mod_state);
                enforce_collapsible_group_umbrella_after_bulk(mod_state);
                enforce_tp2_same_mod_exclusive_after_bulk(mod_state);
            }
            enforce_meta_mode_after_bulk(mod_state);
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
                let row = ui.selectable_label(is_selected, mod_header_label.as_str());
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
                crate::ui::step2::tree_header_marker_step2::render(ui, mod_state, palette);
                if let Some((text_color, bg, label)) = &parent_summary {
                    ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
                    let resp = ui.add(
                        egui::Button::new(
                            crate::ui::shared::typography_global::strong(label)
                                .color(*text_color)
                                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
                        )
                        .fill(*bg)
                        .stroke(egui::Stroke::new(
                            crate::ui::shared::layout_tokens_global::BORDER_THIN,
                            *bg,
                        ))
                        .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_PX as u8))
                        .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
                    );
                    if resp.clicked()
                        && let Some(target_compat) = parent_compat_target(mod_state)
                    {
                        open_compat_for_component = Some((
                            mod_state.tp_file.clone(),
                            target_compat.component_id.clone(),
                            target_compat.raw_line.clone(),
                        ));
                    }
                }
                if mod_has_any_prompt(mod_state, prompt_eval) {
                    ui.add_space(REDESIGN_BIO_ROW_GAP_PX);
                    let prompt_resp = ui.add(
                        egui::Button::new(
                            crate::ui::shared::typography_global::strong("PROMPT")
                                .color(redesign_prompt_text(palette))
                                .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT),
                        )
                        .fill(redesign_prompt_fill(palette))
                        .stroke(egui::Stroke::new(
                            REDESIGN_BORDER_WIDTH_PX,
                            redesign_prompt_stroke(palette),
                        ))
                        .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_PX as u8))
                        .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX)),
                    );
                    let prompt_resp = prompt_resp
                        .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS);
                    if prompt_resp.clicked() {
                        new_selection = Some(Step2Selection::Mod {
                            game_tab: active_tab.to_string(),
                            tp_file: mod_state.tp_file.clone(),
                        });
                        if let Some(text) = build_mod_prompt_popup_text(mod_state, prompt_eval) {
                            open_prompt_popup = Some((mod_state.tp_file.clone(), text));
                        }
                    }
                }
            },
        );
    });
    ParentRowResult {
        selection: new_selection,
        open_compat_for_component,
        open_prompt_popup,
    }
}
