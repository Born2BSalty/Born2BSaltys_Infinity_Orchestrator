// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step1State;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HOME_GAME_LINE_GAP_PX, REDESIGN_HOME_GAME_LINE_TOP_MARGIN_PX,
    REDESIGN_HOME_GAME_STATUS_ICON_WIDTH_PX, REDESIGN_HOME_GAME_STATUS_STROKE_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_accent_deep, redesign_font_light,
    redesign_text_faint, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step1: &Step1State,
    path_check: Option<&(bool, String)>,
) {
    let validation_ok = path_check.is_some_and(|(ok, _)| *ok);
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
    let color = if found {
        redesign_text_primary(palette)
    } else {
        redesign_text_faint(palette)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_HOME_GAME_LINE_GAP_PX;
        let (icon_rect, _) = ui.allocate_exact_size(
            egui::Vec2::splat(REDESIGN_HOME_GAME_STATUS_ICON_WIDTH_PX),
            egui::Sense::hover(),
        );
        if found {
            paint_check(ui.painter(), icon_rect, color);
        } else {
            ui.painter().text(
                icon_rect.center(),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::monospace(REDESIGN_LABEL_FONT_SIZE_PX),
                color,
            );
        }

        let text = if found {
            label.to_owned()
        } else {
            format!("{label} · not found")
        };
        ui.label(
            egui::RichText::new(text)
                .font(egui::FontId::new(
                    REDESIGN_LABEL_FONT_SIZE_PX,
                    redesign_font_light(),
                ))
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(color),
        );
    });
}

fn paint_check(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(REDESIGN_HOME_GAME_STATUS_STROKE_PX, color);
    painter.line_segment(
        [
            egui::pos2(rect.left(), rect.center().y),
            egui::pos2(rect.center().x, rect.bottom()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(rect.center().x, rect.bottom()),
            egui::pos2(rect.right(), rect.top()),
        ],
        stroke,
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
