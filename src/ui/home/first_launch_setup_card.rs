// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `first_launch_setup_card` — the setup CTA shown inside the left Box when
// the modlist registry is empty (zero installed AND zero in-progress).
//
// Per SPEC §3.4: when the registry is empty the left column's main Box
// replaces its contents (filter chips + list) with this card:
//   - A short heading: `Welcome to Infinity Orchestrator`.
//   - One faint sub-line: `Get set up first — point BIO at your games and
//     tools.`
//   - A single primary button: `Open Settings` that navigates to
//     Settings → Paths.
//
// The wireframe `HomeScreen` mocks always have content, so there is no
// dedicated first-launch wireframe component — SPEC §3.4 is the canonical
// source for this card's copy + structure (HANDOFF source-of-truth ordering:
// the wireframe is silent here, so the spec governs).
//
// This module renders the *inner* content only; `page_home` wraps it in the
// shared `redesign_box` chassis so the empty state occupies the same Box the
// filter chips + list would.
//
// SPEC: §3.4 (first-launch / empty-registry state), §3.1.

use eframe::egui;

use crate::ui::orchestrator::widgets::{redesign_btn, BtnOpts};
use crate::ui::shared::redesign_tokens::{
    redesign_text_faint, redesign_text_primary, ThemePalette,
};

/// Render the first-launch CTA content. Returns `true` when `Open Settings`
/// was clicked (the caller routes to Settings → Paths).
pub fn render(ui: &mut egui::Ui, palette: ThemePalette) -> bool {
    let mut open_settings = false;

    ui.vertical(|ui| {
        ui.add_space(4.0);
        // Heading — SPEC §3.4 verbatim.
        ui.label(
            egui::RichText::new("Welcome to Infinity Orchestrator")
                .size(16.0)
                .family(egui::FontFamily::Name("poppins_bold".into()))
                .color(redesign_text_primary(palette)),
        );

        ui.add_space(6.0);
        // Faint sub-line — SPEC §3.4 verbatim (note the em dash).
        ui.label(
            egui::RichText::new("Get set up first \u{2014} point BIO at your games and tools.")
                .size(13.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_faint(palette)),
        );

        ui.add_space(14.0);
        if redesign_btn(
            ui,
            palette,
            "Open Settings",
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .clicked()
        {
            open_settings = true;
        }
        ui.add_space(4.0);
    });

    open_settings
}
