// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::ThemePalette;

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    redesign_btn(
        ui,
        palette,
        "...",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
}
