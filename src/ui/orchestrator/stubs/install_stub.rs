// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install stub destination.
//
// Per Phase 2 P2.T5: title "Install shared modlist", sub "Coming in Phase 5".
//
// SPEC: §4.

use eframe::egui;

use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

pub fn render_install_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_screen_title(
        ui,
        palette,
        "Install shared modlist",
        Some("Coming in Phase 5 \u{2014} SPEC \u{00A7}4"),
    );

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "Phase 5 wires the paste-import-code flow (stages 1\u{2013}3: paste, preview, downloading).",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
