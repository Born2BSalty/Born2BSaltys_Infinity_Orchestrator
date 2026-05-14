// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step1State;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HOME_GAME_LINE_GAP_PX, REDESIGN_HOME_GAME_LINE_TOP_MARGIN_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_accent_deep, redesign_text_faint,
    redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step1: &Step1State,
    path_check: Option<&(bool, String)>,
) {
    let validation_ok = path_check.map(|(ok, _)| *ok).unwrap_or(false);
    ui.label(
        egui::RichText::new("game installs detected")
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(redesign_accent_deep(palette)),
    );
    ui.add_space(REDESIGN_HOME_GAME_LINE_TOP_MARGIN_PX);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_HOME_GAME_LINE_GAP_PX;
        render_game_line(ui, palette, "BGEE", validation_ok && has_bgee_path(step1));
        render_game_line(ui, palette, "BG2EE", validation_ok && has_bg2ee_path(step1));
        render_game_line(ui, palette, "IWDEE", validation_ok && has_iwdee_path(step1));
    });
}

fn render_game_line(ui: &mut egui::Ui, palette: ThemePalette, label: &str, found: bool) {
    let text = if found {
        format!("✓ {label}")
    } else {
        format!("? {label} · not found")
    };
    let color = if found {
        redesign_text_primary(palette)
    } else {
        redesign_text_faint(palette)
    };
    ui.label(
        egui::RichText::new(text)
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(color),
    );
}

fn has_bgee_path(step1: &Step1State) -> bool {
    !step1.bgee_game_folder.trim().is_empty() || !step1.eet_bgee_game_folder.trim().is_empty()
}

fn has_bg2ee_path(step1: &Step1State) -> bool {
    !step1.bg2ee_game_folder.trim().is_empty() || !step1.eet_bg2ee_game_folder.trim().is_empty()
}

fn has_iwdee_path(step1: &Step1State) -> bool {
    step1.game_install == "IWDEE" && !step1.bgee_game_folder.trim().is_empty()
}
