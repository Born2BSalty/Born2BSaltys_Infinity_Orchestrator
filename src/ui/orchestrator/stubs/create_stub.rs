// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create stub destination.
//
// Per Phase 2 P2.T5: title "Create / edit modlist", sub "Coming in Phase 6".
//
// SPEC: §5.

use eframe::egui;

use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

pub fn render_create_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_screen_title(
        ui,
        palette,
        "Create / edit modlist",
        Some("Coming in Phase 6 \u{2014} SPEC \u{00A7}5"),
    );

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Phase 6 wires the Create choose mode, setup box, starting-point cards, and the workspace shell.",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
