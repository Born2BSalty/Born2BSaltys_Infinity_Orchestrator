// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Workspace stub destination.
//
// Per Phase 2 P2.T5: title "Workspace", sub "Coming in Phase 6", plus a short
// note about which modlist id is loaded. **It does NOT host `WizardApp`** —
// per C1/C4 revisions, the Workspace destination is wired in Phase 6 to call
// BIO's per-step page renderers directly (plus an orchestrator-side Step 4
// wrapper per C4).
//
// SPEC: §2.2.

use eframe::egui;

use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};

pub fn render_workspace_stub(ui: &mut egui::Ui, palette: ThemePalette, modlist_id: Option<&str>) {
    render_screen_title(
        ui,
        palette,
        "Workspace",
        Some("Coming in Phase 6 \u{2014} SPEC \u{00A7}2.2"),
    );

    ui.add_space(8.0);
    let note = modlist_id.map_or_else(
        || {
            String::from(
                "No modlist id loaded (entered via dev-mode `Open workspace stub (dev)` button)",
            )
        },
        |id| format!("Loaded modlist id: {id}"),
    );
    ui.label(
        egui::RichText::new(note)
            .size(13.0)
            .family(egui::FontFamily::Proportional)
            .color(redesign_text_faint(palette)),
    );

    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(
            "Phase 6 replaces this stub with the real workspace shell (progress bar + Steps 2\u{2013}5).",
        )
        .size(13.0)
        .family(egui::FontFamily::Proportional)
        .color(redesign_text_faint(palette)),
    );
}
