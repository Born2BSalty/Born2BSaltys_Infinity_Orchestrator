// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::{active_mods_mut, build_prompt_eval_context};
use crate::ui::step2::tree_step2::step2_tree::{render_mod_tree, ModTreeRenderResult};

pub(crate) fn render_list_pane(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    left_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
        ui.set_clip_rect(left_rect);
        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin {
                left: 0,
                right: 6,
                top: 0,
                bottom: 0,
            })
            .show(ui, |ui| {
                ui.set_min_size(left_rect.size());
                ui.scope(|ui| {
                    let mut scroll = egui::style::ScrollStyle::solid();
                    scroll.bar_width = 12.0;
                    scroll.bar_inner_margin = 0.0;
                    scroll.bar_outer_margin = 2.0;
                    ui.style_mut().spacing.scroll = scroll;

                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let filter = state.step2.search_query.trim().to_lowercase();
                            let active_tab = state.step2.active_game_tab.clone();
                            let collapse_epoch = state.step2.collapse_epoch;
                            let collapse_default_open = state.step2.collapse_default_open;
                            let mut selected = state.step2.selected.clone();
                            let mut next_selection_order = state.step2.next_selection_order;
                            let mut jump_to_selected_requested =
                                state.step2.jump_to_selected_requested;
                            let mut prompt_popup: Option<(String, String)> = None;
                            let prompt_eval = build_prompt_eval_context(state);
                            let mods = active_mods_mut(&mut state.step2);
                            if mods.is_empty() {
                                ui.label("No mods scanned yet.");
                            } else {
                                let mut rendered_any = false;
                                for mod_state in mods.iter_mut() {
                                    let matches =
                                        crate::ui::step2::service_step2::mod_matches_filter(
                                            mod_state, &filter,
                                        );
                                    if !matches {
                                        continue;
                                    }
                                    rendered_any = true;
                                    let mut render_ctx = crate::ui::step2::tree_step2::step2_tree::render::ModTreeRenderContext {
                                        filter: &filter,
                                        active_tab: &active_tab,
                                        selected: &selected,
                                        next_selection_order: &mut next_selection_order,
                                        prompt_eval: &prompt_eval,
                                        collapse_epoch,
                                        collapse_default_open,
                                        jump_to_selected_requested: &mut jump_to_selected_requested,
                                    };
                                    let maybe_selected = render_mod_tree(ui, &mut render_ctx, mod_state);
                                    if let Some(ModTreeRenderResult {
                                        selected: new_selected,
                                        open_compat_for_component,
                                        open_prompt_popup,
                                    }) = maybe_selected
                                    {
                                        selected = Some(new_selected);
                                        if let Some((tp_file, component_id, component_key)) =
                                            open_compat_for_component
                                        {
                                            *action = Some(Step2Action::OpenCompatForComponent {
                                                game_tab: active_tab.clone(),
                                                tp_file,
                                                component_id,
                                                component_key,
                                            });
                                        }
                                        if let Some((title, text)) = open_prompt_popup {
                                            prompt_popup = Some((title, text));
                                        }
                                    }
                                    ui.add_space(6.0);
                                }
                                if !rendered_any {
                                    ui.label("No mods/components match your search.");
                                }
                            }
                            state.step2.selected = selected;
                            state.step2.next_selection_order = next_selection_order;
                            state.step2.jump_to_selected_requested = jump_to_selected_requested;
                            if let Some((title, text)) = prompt_popup {
                                state.step2.prompt_popup_title = title;
                                state.step2.prompt_popup_text = text;
                                state.step2.prompt_popup_open = true;
                            }
                        });
                });
            });
    });
}
