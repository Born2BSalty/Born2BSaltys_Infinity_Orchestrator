// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::Game;
use crate::ui::orchestrator::widgets::brand_mark;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
};

pub const ART_ASPECT_W: f32 = 460.0;
pub const ART_ASPECT_H: f32 = 215.0;

const GRADIENT_STRIPS: usize = 32;
const STRIP_COUNT_F: f32 = 32.0;
const GRADIENT_FALLOFF: f32 = 0.55;
const WATERMARK_ALPHA: u8 = 54;

#[must_use]
pub fn height_for_width(width: f32) -> f32 {
    (width * ART_ASPECT_H / ART_ASPECT_W).round()
}

const fn tint(game: Game) -> egui::Color32 {
    match game {
        Game::BGEE => egui::Color32::from_rgb(0x24, 0x44, 0x6e),
        Game::BG2EE => egui::Color32::from_rgb(0x63, 0x35, 0x27),
        Game::IWDEE => egui::Color32::from_rgb(0x2a, 0x55, 0x68),
        Game::EET => egui::Color32::from_rgb(0x3a, 0x31, 0x6a),
    }
}

pub fn paint(ui: &egui::Ui, palette: ThemePalette, game: Game, rect: egui::Rect) {
    if !ui.is_rect_visible(rect) {
        return;
    }

    paint_gradient(ui, game, rect);
    paint_watermark(ui, rect);
    paint_game_label(ui, game, rect);

    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

fn paint_gradient(ui: &egui::Ui, game: Game, rect: egui::Rect) {
    let base = tint(game);
    let painter = ui.painter();
    let strip_h = rect.height() / STRIP_COUNT_F;
    let step = 1.0 / (STRIP_COUNT_F - 1.0);
    let mut top = rect.top();
    let mut fraction = 0.0_f32;

    for index in 0..GRADIENT_STRIPS {
        let is_first = index == 0;
        let is_last = index + 1 == GRADIENT_STRIPS;
        let bottom = if is_last {
            rect.bottom()
        } else {
            top + strip_h
        };
        let strip = egui::Rect::from_min_max(
            egui::pos2(rect.left(), top),
            egui::pos2(rect.right(), bottom),
        );
        let corner = egui::CornerRadius {
            nw: if is_first {
                REDESIGN_BORDER_RADIUS_U8
            } else {
                0
            },
            ne: if is_first {
                REDESIGN_BORDER_RADIUS_U8
            } else {
                0
            },
            sw: if is_last {
                REDESIGN_BORDER_RADIUS_U8
            } else {
                0
            },
            se: if is_last {
                REDESIGN_BORDER_RADIUS_U8
            } else {
                0
            },
        };
        painter.rect_filled(
            strip,
            corner,
            base.gamma_multiply(GRADIENT_FALLOFF.mul_add(-fraction, 1.0)),
        );
        top = bottom;
        fraction += step;
    }
}

fn paint_watermark(ui: &egui::Ui, rect: egui::Rect) {
    let by_height = rect.height() * 0.46;
    let by_width = rect.width() * 0.28;
    let mark_size = by_height.min(by_width).max(24.0);
    let mark_rect = egui::Rect::from_center_size(
        egui::pos2(
            rect.center().x,
            rect.height().mul_add(-0.06, rect.center().y),
        ),
        egui::vec2(mark_size, mark_size),
    );
    let target_px = mark_size * ui.ctx().pixels_per_point();
    let tex = brand_mark::texture(ui.ctx(), brand_mark::pixels_for(target_px));
    egui::Image::new(egui::load::SizedTexture::new(tex.id(), tex.size_vec2()))
        .tint(egui::Color32::from_white_alpha(WATERMARK_ALPHA))
        .paint_at(ui, mark_rect);
}

fn paint_game_label(ui: &egui::Ui, game: Game, rect: egui::Rect) {
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.bottom() - 10.0),
        egui::Align2::LEFT_BOTTOM,
        game.to_legacy_string(),
        egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
        egui::Color32::from_white_alpha(210),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_height_follows_the_460_by_215_aspect() {
        assert!((height_for_width(460.0) - 215.0).abs() < f32::EPSILON);
        assert!((height_for_width(920.0) - 430.0).abs() < f32::EPSILON);
        assert!(height_for_width(300.0) > 0.0);
    }

    #[test]
    fn every_game_has_its_own_tint() {
        let mut seen = vec![
            tint(Game::BGEE),
            tint(Game::BG2EE),
            tint(Game::IWDEE),
            tint(Game::EET),
        ];
        let total = seen.len();
        seen.sort_unstable_by_key(egui::Color32::to_array);
        seen.dedup();
        assert_eq!(seen.len(), total, "each game must read as a distinct tint");
    }

    #[test]
    fn strip_count_constant_matches_the_loop_bound() {
        assert!((STRIP_COUNT_F - 32.0).abs() < f32::EPSILON);
        assert_eq!(GRADIENT_STRIPS, 32);
    }
}
