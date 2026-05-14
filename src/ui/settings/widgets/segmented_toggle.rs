// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{self, BtnOpts};
use crate::ui::shared::redesign_tokens::ThemePalette;

pub fn render_theme(ui: &mut egui::Ui, palette: ThemePalette, selected: &mut ThemePalette) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if btn::redesign_btn(
            ui,
            palette,
            "light",
            BtnOpts {
                primary: *selected == ThemePalette::Light,
                small: true,
                disabled: false,
            },
        )
        .clicked()
        {
            *selected = ThemePalette::Light;
        }
        if btn::redesign_btn(
            ui,
            palette,
            "dark",
            BtnOpts {
                primary: *selected == ThemePalette::Dark,
                small: true,
                disabled: false,
            },
        )
        .clicked()
        {
            *selected = ThemePalette::Dark;
        }
    });
}
