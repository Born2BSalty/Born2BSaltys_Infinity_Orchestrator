// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `game_installs_detected` — the detected-games lines under the right-column
// CTAs.
//
// Mirrors `wireframe-preview/screens.jsx::HomeScreen` (line 357-362):
//   <Label hand>game installs detected</Label>          // rendered by caller
//   <Label style={fontSize:14}>✓ BGEE</Label>
//   <Label style={fontSize:14}>✓ BG2EE</Label>
//   <Label style={fontSize:14, color:var(--text-faint)}>? IWDEE · not found</Label>
//
// SPEC §3.3: "Detection comes from the same logic that today populates
// Step 1 path validation." Refresh semantics: "The block re-evaluates
// automatically whenever path validation runs" — the orchestrator's
// `settings_screen_state.path_validation_results` (a `ValidationReport`) is
// refreshed by Phase 4's `validate_now::run_now` (startup) and
// `validate_debounce::tick` (per-edit) and Settings → `Validate now`.
// Reading it per-frame here gives the auto-refresh with no manual control.
//
// Found  → `✓ <NAME>` in primary text.
// Missing → `? <NAME> · not found` in faint text.
//
// "Found" == the game's game-folder field validated to `PathStatus::Ok`.
// A `Warning` (path set but no chitin.key/lang), `Error`, `Empty`, or an
// absent field all read as not-found — the same threshold the rail status
// uses and what the user sees in Settings → Paths per-row hints.
//
// SPEC: §3.3 (game installs detected + Refresh semantics).

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::settings::state_settings::PathStatus;
use crate::ui::settings::validate_now::{
    FIELD_BG2EE_GAME_FOLDER, FIELD_BGEE_GAME_FOLDER, FIELD_IWDEE_GAME_FOLDER,
};
use crate::ui::shared::redesign_tokens::{
    redesign_text_faint, redesign_text_primary, ThemePalette,
};

/// The three games the wireframe lists, in order, with their game-folder
/// validation field key.
const GAME_ROWS: [(&str, &str); 3] = [
    ("BGEE", FIELD_BGEE_GAME_FOLDER),
    ("BG2EE", FIELD_BG2EE_GAME_FOLDER),
    ("IWDEE", FIELD_IWDEE_GAME_FOLDER),
];

/// Render the BGEE / BG2EE / IWDEE detection lines. (The `game installs
/// detected` header is painted by the caller — `add_a_modlist` — so the
/// 20px gap + header live in the same Box.)
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &OrchestratorApp) {
    let report = &orchestrator.settings_screen_state.path_validation_results;

    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        for (name, field) in &GAME_ROWS {
            let found = matches!(report.fields.get(*field), Some(PathStatus::Ok { .. }));
            let (marker, text, color) = if found {
                ("\u{2713}", name.to_string(), redesign_text_primary(palette)) // ✓
            } else {
                (
                    "?",
                    format!("{name} \u{00B7} not found"),
                    redesign_text_faint(palette),
                )
            };
            // The marker glyph is rendered in FiraCode Nerd, not Poppins:
            // our shipped Poppins TTFs are a Latin-only subset that lacks
            // U+2713 (✓), so a Poppins ✓ tofus to `?` — making a *found*
            // game look identical to a missing one. Nerd Font has full
            // symbol coverage (same reason the brand mark uses it). The
            // name text stays Poppins for consistency with the rest of the
            // UI; a tight row keeps "✓ BGEE" reading as one unit.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(
                    egui::RichText::new(marker)
                        .size(14.0)
                        .family(egui::FontFamily::Name("firacode_nerd".into()))
                        .color(color),
                );
                ui.label(
                    egui::RichText::new(text)
                        .size(14.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(color),
                );
            });
        }
    });
}
