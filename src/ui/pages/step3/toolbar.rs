// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step3::{access, tabs};
use crate::ui::state::WizardState;

use super::Step3Action;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState, action: &mut Option<Step3Action>) {
    ui.horizontal(|ui| {
        let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
        let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
        if show_bgee && show_bg2ee {
            tabs::draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
            tabs::draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
        } else if show_bgee {
            ui.label(egui::RichText::new("BGEE").monospace());
        } else if show_bg2ee {
            ui.label(egui::RichText::new("BG2EE").monospace());
        }

        if state.compat.error_count > 0
            && ui
                .button(
                    egui::RichText::new(format!("{} errors", state.compat.error_count))
                        .color(egui::Color32::from_rgb(220, 100, 100))
                        .strong(),
                )
                .on_hover_text("Open compatibility issues")
                .clicked()
        {
            state.step3.compat_modal_open = true;
        }
        if state.compat.warning_count > 0
            && ui
                .button(
                    egui::RichText::new(format!("{} warnings", state.compat.warning_count))
                        .color(egui::Color32::from_rgb(220, 180, 100))
                        .strong(),
                )
                .on_hover_text("Open compatibility issues")
                .clicked()
        {
            state.step3.compat_modal_open = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (
                items,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                _,
                collapsed_blocks,
                _,
                _,
                undo_stack,
                redo_stack,
            ) = access::active_list_mut(state);

            if ui
                .button("Revalidate")
                .on_hover_text("Re-run compatibility check for current selection/order.")
                .clicked()
            {
                *action = Some(Step3Action::Revalidate);
            }
            if ui
                .button("Expand All")
                .on_hover_text("Expand all parent blocks.")
                .clicked()
            {
                collapsed_blocks.clear();
            }
            if ui
                .button("Collapse All")
                .on_hover_text("Collapse all parent blocks.")
                .clicked()
            {
                collapsed_blocks.clear();
                for item in items.iter().filter(|i| i.is_parent) {
                    if !collapsed_blocks.contains(&item.block_id) {
                        collapsed_blocks.push(item.block_id.clone());
                    }
                }
            }
            if ui
                .add_enabled(!redo_stack.is_empty(), egui::Button::new("Redo"))
                .on_hover_text("Redo the most recent undone reorder.")
                .clicked()
                && let Some(next) = redo_stack.pop()
            {
                undo_stack.push(items.clone());
                *items = next;
            }
            if ui
                .add_enabled(!undo_stack.is_empty(), egui::Button::new("Undo"))
                .on_hover_text("Undo the most recent reorder.")
                .clicked()
                && let Some(previous) = undo_stack.pop()
            {
                redo_stack.push(items.clone());
                *items = previous;
            }
        });
    });
}
