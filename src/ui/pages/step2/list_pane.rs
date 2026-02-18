// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::filter::{active_mods_mut, mod_matches_filter};
use crate::ui::step2::tree::{ModTreeRenderResult, render_mod_tree};
use crate::ui::state::WizardState;

use super::Step2Action;

pub(super) fn render(
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
                            let mods = active_mods_mut(&mut state.step2);
                            if mods.is_empty() {
                                ui.label("No mods scanned yet.");
                            } else {
                                let mut rendered_any = false;
                                for mod_state in mods {
                                    let matches = mod_matches_filter(mod_state, &filter);
                                    if !matches {
                                        continue;
                                    }
                                    rendered_any = true;
                                    let maybe_selected = render_mod_tree(
                                        ui,
                                        &filter,
                                        &active_tab,
                                        &selected,
                                        &mut next_selection_order,
                                        collapse_epoch,
                                        collapse_default_open,
                                        &mut jump_to_selected_requested,
                                        mod_state,
                                    );
                                    if let Some(ModTreeRenderResult {
                                        selected: new_selected,
                                        open_compat_for_component,
                                    }) = maybe_selected
                                    {
                                        selected = Some(new_selected);
                                        if let Some((tp_file, component_id, component_key)) = open_compat_for_component {
                                            *action = Some(Step2Action::OpenCompatForComponent {
                                                game_tab: active_tab.clone(),
                                                tp_file,
                                                component_id,
                                                component_key,
                                            });
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
                        });
                });
            });
    });
}
