// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ScreenTitle` primitive — 22px Poppins 500 title with optional 13px muted
// subtitle and a 20px bottom margin.
//
// Mirrors `wireframe-preview/screens.jsx::ScreenTitle` (line 191-218):
//   <h1 fontSize=22 fontWeight=500 color=var(--text) lineHeight=1>
//   <div fontSize=13 color=var(--text-muted) marginTop=4>
//   container marginBottom=20
//
// SPEC: §1.2 (typography), per Phase 2 P2.T6.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

/// Render a `ScreenTitle` at the current `ui` cursor.
///
/// - `title` — required, rendered in 22px Poppins 500 / primary text color.
/// - `sub`   — optional one-liner in 13px Poppins / muted text color.
///
/// Adds a 20px bottom margin (matches wireframe `marginBottom: 20px`).
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, title: &str, sub: Option<&str>) {
    ui.vertical(|ui| {
        ui.add_space(0.0);
        let title_rich = egui::RichText::new(title)
            .size(22.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette));
        ui.label(title_rich);

        if let Some(sub) = sub {
            ui.add_space(4.0);
            let sub_rich = egui::RichText::new(sub)
                .size(13.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_muted(palette));
            ui.label(sub_rich);
        }

        ui.add_space(20.0);
    });
}
