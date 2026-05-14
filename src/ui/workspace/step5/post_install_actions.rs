// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{REDESIGN_HOME_ACTION_COLUMN_GAP_PX, ThemePalette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostInstallAction {
    ReturnToHome,
    OpenInstallFolder,
}

pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> Option<PostInstallAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_HOME_ACTION_COLUMN_GAP_PX;
        if redesign_btn(
            ui,
            palette,
            "Return to Home",
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            action = Some(PostInstallAction::ReturnToHome);
        }
        if redesign_btn(ui, palette, "Open install folder", BtnOpts::default()).clicked() {
            action = Some(PostInstallAction::OpenInstallFolder);
        }
    });

    action
}
