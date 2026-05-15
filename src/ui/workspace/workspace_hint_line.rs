// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, ThemePalette, redesign_text_muted,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, step: WorkspaceStep) {
    ui.label(
        egui::RichText::new(step.hint())
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_text_muted(palette)),
    );
}
