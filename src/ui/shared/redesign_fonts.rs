// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Redesign-only font installation for the Infinity Orchestrator binary.
//!
//! Per Phase 1 P1.T1 / H7: this function builds a complete `FontDefinitions`
//! from scratch (`FontDefinitions::default()` + the redesign families) and
//! installs it via `ctx.set_fonts(...)`. **No `ctx.fonts(|f| ...)` read; no
//! additive composition.** The companion `configure_typography` only mutates
//! `Style`, so the two compose cleanly when this function is called first.
//!
//! Font sources (Phase 1 trial):
//! - Poppins 300 / 500 / 700: derived from the wireframe's `.woff2` subsets
//!   (Latin-only) and converted to `.ttf` for egui consumption. A future pass
//!   should replace these with full Latin-Extended `.ttf` builds from the
//!   upstream Google Fonts release for better glyph coverage.
//! - `FiraCode` Nerd 300: derived from the wireframe's `firacode-nerd-300.woff2`
//!   and converted to `.ttf`.

use eframe::egui;

const POPPINS_LIGHT: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Light.ttf");
const POPPINS_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Medium.ttf");
const POPPINS_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Poppins-Bold.ttf");
const FIRACODE_NERD_LIGHT: &[u8] =
    include_bytes!("../../../assets/fonts/FiraCodeNerdFont-Light.ttf");

/// Install the redesign fonts on the given egui context.
///
/// Builds a complete `FontDefinitions` and replaces the egui font configuration
/// in one call. The Proportional family is prefixed with `poppins_medium` (the
/// default UI weight); the Monospace family is prefixed with `firacode_nerd`.
/// Additional weights (`poppins_light`, `poppins_bold`) are registered so render
/// code can select them per-`RichText` via `FontId::new(size, FontFamily::Name(..))`
/// or by inserting them into custom families in later phases.
pub fn install_redesign_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    register_font(&mut fonts, "poppins_light", POPPINS_LIGHT);
    register_font(&mut fonts, "poppins_medium", POPPINS_MEDIUM);
    register_font(&mut fonts, "poppins_bold", POPPINS_BOLD);
    register_font(&mut fonts, "firacode_nerd", FIRACODE_NERD_LIGHT);

    // Default proportional UI text → Poppins 500 (medium).
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "poppins_medium".to_owned());

    // Default monospace → FiraCode Nerd.
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "firacode_nerd".to_owned());

    // Named families for weight-specific selection in later phases.
    fonts.families.insert(
        egui::FontFamily::Name("poppins_light".into()),
        vec!["poppins_light".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("poppins_medium".into()),
        vec!["poppins_medium".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("poppins_bold".into()),
        vec!["poppins_bold".to_owned()],
    );
    fonts.families.insert(
        egui::FontFamily::Name("firacode_nerd".into()),
        vec!["firacode_nerd".to_owned()],
    );

    ctx.set_fonts(fonts);
}

fn register_font(fonts: &mut egui::FontDefinitions, name: &'static str, bytes: &'static [u8]) {
    if bytes.is_empty() {
        // Defensive guard: if a font asset is missing/empty at build time the
        // include_bytes! above would have failed to compile, but we keep this
        // check so future placeholder/zero-length fallbacks degrade gracefully.
        return;
    }
    fonts
        .font_data
        .insert(name.to_owned(), egui::FontData::from_static(bytes).into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redesign_fonts_register() {
        let ctx = egui::Context::default();
        install_redesign_fonts(&ctx);
        // Drive one frame so set_fonts is realized before we inspect.
        let _ = ctx.run(egui::RawInput::default(), |_| {});

        ctx.fonts(|f| {
            // The names we registered must resolve to actual font ids.
            let _light = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_light".into())),
                'A',
            );
            let _medium = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into())),
                'A',
            );
            let _bold = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("poppins_bold".into())),
                'A',
            );
            let _mono = f.glyph_width(
                &egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
                'A',
            );
        });
    }
}
