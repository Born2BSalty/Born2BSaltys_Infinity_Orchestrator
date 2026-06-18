// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, WORKSPACE_CONTENT_TEXT_INSET, redesign_text_faint,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, current: WorkspaceStep) {
    ui.horizontal(|ui| {
        ui.add_space(WORKSPACE_CONTENT_TEXT_INSET);
        ui.label(
            egui::RichText::new(current.hint())
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
    });
    ui.add_space(10.0);
}
