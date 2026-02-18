// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step2::tabs::draw_tab;

use super::Step2Action;

pub(super) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    tabs_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(tabs_rect), |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Mods / Components").strong().size(14.0))
                .on_hover_text("Active game tab controls which component list and log-apply action are used.");
            let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
            let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
            let active_is_bgee = state.step2.active_game_tab == "BGEE";
            let active_is_bg2 = state.step2.active_game_tab == "BG2EE";
            let bgee_scanned = !state.step2.bgee_mods.is_empty();
            let bg2_scanned = !state.step2.bg2ee_mods.is_empty();

            if show_bgee && show_bg2ee {
                draw_tab(ui, &mut state.step2.active_game_tab, "BGEE");
                draw_tab(ui, &mut state.step2.active_game_tab, "BG2EE");
            } else if show_bgee {
                ui.label(egui::RichText::new("BGEE").monospace());
            } else if show_bg2ee {
                ui.label(egui::RichText::new("BG2EE").monospace());
            }

            ui.add_space(10.0);
            if active_is_bgee
                && ui
                    .add_enabled(
                        bgee_scanned,
                        egui::Button::new("Select BGEE via WeiDU Log")
                            .min_size(egui::vec2(230.0, 24.0)),
                    )
                    .on_hover_text("Read BGEE WeiDU log and tick matching components.")
                    .clicked()
            {
                *action = Some(Step2Action::SelectBgeeViaLog);
            }
            if active_is_bg2
                && ui
                    .add_enabled(
                        bg2_scanned,
                        egui::Button::new("Select BG2EE via WeiDU Log")
                            .min_size(egui::vec2(236.0, 24.0)),
                    )
                    .on_hover_text("Read BG2EE WeiDU log and tick matching components.")
                    .clicked()
            {
                *action = Some(Step2Action::SelectBg2eeViaLog);
            }
        });
    });
}
