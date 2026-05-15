// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `Label` primitive — two variants.
//
// Mirrors `wireframe-preview/screens.jsx::Label` (line 177-189):
//   regular  → Poppins 13px, color: var(--text)
//   hand     → Poppins 14px, color: var(--accent-deep)
//
// SPEC: §1.2.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_accent_deep, redesign_text_primary, ThemePalette,
};

/// Regular label — 13px Poppins in primary text color.
pub fn redesign_label(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    let rich = egui::RichText::new(text)
        .size(13.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_text_primary(palette));
    ui.label(rich)
}

/// "Hand-style" label — 14px Poppins in `accent-deep`, used for hint copy
/// throughout the wireframe (`<Label hand>` calls).
pub fn redesign_label_hand(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    let rich = egui::RichText::new(text)
        .size(14.0)
        .family(egui::FontFamily::Name("poppins_medium".into()))
        .color(redesign_accent_deep(palette));
    ui.label(rich)
}
