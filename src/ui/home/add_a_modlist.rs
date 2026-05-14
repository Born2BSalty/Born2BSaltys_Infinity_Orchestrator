// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::home::page_home::HomeAction;
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{REDESIGN_HOME_ACTION_COLUMN_GAP_PX, ThemePalette};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> Option<HomeAction> {
    let mut action = None;

    redesign_box(ui, palette, Some("add a modlist"), |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = REDESIGN_HOME_ACTION_COLUMN_GAP_PX;
            if redesign_btn(
                ui,
                palette,
                "paste import code",
                BtnOpts {
                    primary: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                action = Some(HomeAction::OpenInstall);
            }
            if redesign_btn(ui, palette, "create your own", BtnOpts::default()).clicked() {
                action = Some(HomeAction::OpenCreate);
            }
        });
    });

    action
}
