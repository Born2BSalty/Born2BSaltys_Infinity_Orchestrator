// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `Pill` primitive — generic tone-aware status pill (small rounded label with
// a tinted fill + dark text).
//
// Mirrors `wireframe-preview/screens.jsx::Pill` semantics: a compact rounded
// chip used for short status words. Tones come from SPEC §12.2's
// theme-invariant pill palette (`pill_danger` / `pill_warn` / `pill_info` /
// `pill_neutral`) with the fixed dark `pill_text` foreground.
//
// NOTE (Phase 5 / Run 1): the Home modlist cards intentionally do **not**
// render a status pill — per SPEC §3.1 ("without needing an extra status
// pill") + the wireframe `HomeScreen` cards, the meta line and the
// action-button label (`resume` vs `open`) disambiguate state. This generic
// widget is provided per the Phase 5 file inventory for the surfaces that do
// use pills (Workspace fork badge, preview overview, etc.).
//
// SPEC: §12.2 (pill tones).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); `Self` vs the explicit enum
// name is a stylistic preference (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::use_self
)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, ThemePalette, redesign_pill_danger, redesign_pill_info,
    redesign_pill_neutral, redesign_pill_text, redesign_pill_warn,
};

/// Pill tone → fill color (SPEC §12.2, theme-invariant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillTone {
    Danger,
    Warn,
    Info,
    Neutral,
}

impl PillTone {
    fn fill(self, palette: ThemePalette) -> egui::Color32 {
        match self {
            PillTone::Danger => redesign_pill_danger(palette),
            PillTone::Warn => redesign_pill_warn(palette),
            PillTone::Info => redesign_pill_info(palette),
            PillTone::Neutral => redesign_pill_neutral(palette),
        }
    }
}

/// Render a pill at the current cursor and return its `Response`.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    tone: PillTone,
) -> egui::Response {
    let pad_x = 8.0;
    let pad_y = 2.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(11.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
            tone.fill(palette),
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    response
}
