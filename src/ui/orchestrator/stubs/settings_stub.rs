// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Settings stub destination.
//
// Per Phase 2 P2.T5: title "Settings", sub "Coming in Phase 4".
//
// SPEC: §11.

use eframe::egui;

use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{redesign_text_faint, ThemePalette};

pub fn render_settings_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_screen_title(
        ui,
        palette,
        "Settings",
        Some("Coming in Phase 4 \u{2014} SPEC \u{00A7}11"),
    );

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Phase 4 wires the 5 file-folder tabs (General, Paths, Tools, Accounts, Advanced) plus the theme toggle.",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
