// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::home::game_installs_detected;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_box, redesign_btn};
use crate::ui::shared::redesign_tokens::redesign_accent_deep;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddAModlistAction {
    #[default]
    None,
    PasteImportCode,
    CreateYourOwn,
}

pub fn render(ui: &mut egui::Ui, orchestrator: &OrchestratorApp) -> AddAModlistAction {
    let palette = orchestrator.theme_palette;
    let mut action = AddAModlistAction::None;

    redesign_box(ui, palette, Some("add a modlist"), |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;

            if redesign_btn(
                ui,
                palette,
                "paste import code",
                BtnOpts {
                    primary: true,
                    block: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = AddAModlistAction::PasteImportCode;
            }

            if redesign_btn(
                ui,
                palette,
                "create your own",
                BtnOpts {
                    block: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = AddAModlistAction::CreateYourOwn;
            }
        });

        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("game installs detected")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_accent_deep(palette)),
        );
        ui.add_space(6.0);
        game_installs_detected::render(ui, palette, orchestrator);
    });

    action
}
