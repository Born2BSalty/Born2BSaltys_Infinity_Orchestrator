// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step2::actions::{clear_all, select_visible};
use crate::ui::step2::filter::active_mods_mut;

use super::Step2Action;
use super::util::active_mods_ref;

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    controls_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(controls_rect), |ui| {
        let bgee_scanned = !state.step2.bgee_mods.is_empty();
        let bg2_scanned = !state.step2.bg2ee_mods.is_empty();
        let has_any_checked = active_mods_ref(state)
            .iter()
            .any(|m| m.checked || m.components.iter().any(|c| c.checked));
        ui.horizontal(|ui| {
            if ui
                .add_sized([148.0, 28.0], egui::Button::new("Scan Mods Folder"))
                .on_hover_text("Scan the configured Mods Folder and build the mod/component tree.")
                .clicked()
            {
                *action = Some(Step2Action::StartScan);
            }
            if state.step2.is_scanning
                && ui
                    .add_sized([124.0, 28.0], egui::Button::new("Cancel Scan"))
                    .on_hover_text("Cancel the active scan.")
                    .clicked()
            {
                *action = Some(Step2Action::CancelScan);
            }
            let has_scanned = bgee_scanned || bg2_scanned;
            if has_any_checked
                && ui
                    .add_enabled(
                        has_scanned,
                        egui::Button::new("Clear All").min_size(egui::vec2(84.0, 28.0)),
                    )
                    .on_hover_text("Uncheck all components in the current tab.")
                    .clicked()
            {
                let mut next_order = state.step2.next_selection_order;
                let mods = active_mods_mut(&mut state.step2);
                clear_all(mods, &mut next_order);
                state.step2.next_selection_order = next_order;
                state.step2.selected = None;
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Select Visible").min_size(egui::vec2(108.0, 28.0)),
                )
                .on_hover_text("Check all filter-matching components in the current tab.")
                .clicked()
            {
                let filter = state.step2.search_query.trim().to_lowercase();
                let mut next_order = state.step2.next_selection_order;
                let mods = active_mods_mut(&mut state.step2);
                select_visible(mods, &filter, &mut next_order);
                state.step2.next_selection_order = next_order;
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Collapse All").min_size(egui::vec2(104.0, 28.0)),
                )
                .on_hover_text("Collapse all parent mods in the tree.")
                .clicked()
            {
                state.step2.collapse_default_open = false;
                state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Expand All").min_size(egui::vec2(94.0, 28.0)),
                )
                .on_hover_text("Expand all parent mods in the tree.")
                .clicked()
            {
                state.step2.collapse_default_open = true;
                state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
            }
            if ui
                .add_enabled(
                    state.step2.selected.is_some(),
                    egui::Button::new("Jump to Selected").min_size(egui::vec2(132.0, 28.0)),
                )
                .on_hover_text("Scroll to the currently selected row in the tree.")
                .clicked()
            {
                state.step2.jump_to_selected_requested = true;
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Revalidate").min_size(egui::vec2(94.0, 28.0)),
                )
                .on_hover_text("Re-run compatibility checks against current selections/order.")
                .clicked()
            {
                *action = Some(Step2Action::RevalidateCompat);
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Export Compat").min_size(egui::vec2(114.0, 28.0)),
                )
                .on_hover_text("Export Step 2 compatibility report to diagnostics (TXT).")
                .clicked()
            {
                *action = Some(Step2Action::ExportCompatReport);
            }
        });
    });
}
