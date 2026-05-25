// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

/// Renders a section title row with an optional item count.
///
/// Paints the title in primary text with strong weight, and appends
/// ` (N)` in muted text when `count` is `Some`. Does not wrap content — the
/// caller places a framed content block below.
pub fn redesign_section_header(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    title: &str,
    count: Option<usize>,
) {
    ui.horizontal(|ui| {
        let primary_color = redesign_text_primary(palette);
        let muted_color = redesign_text_muted(palette);

        ui.label(egui::RichText::new(title).color(primary_color).strong());

        if let Some(n) = count {
            ui.label(
                egui::RichText::new(format!("({n})"))
                    .color(muted_color)
                    .size(12.0),
            );
        }
    });
}
