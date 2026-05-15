// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Redesign theme tokens (Infinity Orchestrator binary only).
//
// Per Phase 1 P1.T2 / P1.T3:
//   - `ThemePalette` is a plain `Copy + Eq` enum — **no global storage**, no
//     `AtomicU8`, no singleton. Per-frame the active palette is owned by
//     `OrchestratorApp` and passed explicitly to render code.
//   - Accessors are `pub fn` and take `palette: ThemePalette` as their first
//     argument; each is referentially transparent.
//   - Two `pub(crate) const` palette tables (`LIGHT`, `DARK`) hold the
//     `egui::Color32` values per SPEC §12.1 and wireframe `index.html:14-60`.
//
// Layout / spacing constants live in the same file per P1.T3 ("no need to
// fragment").
//
// SPEC: §1.2 (sketchy borders / shadow), §12.1 (palette), §12.2 (pill tones),
//       §12.3 (misc rules), §2.1 (left-rail width 200px labels mode).

use eframe::egui::Color32;

// ---------------------------------------------------------------------------
// Palette identity
// ---------------------------------------------------------------------------

/// Active palette identity. Plain `Copy + Eq`. No global state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemePalette {
    Light,
    Dark,
}

impl Default for ThemePalette {
    /// Dark is the default for the redesigned app (SPEC §1.2 / §12.1).
    fn default() -> Self {
        ThemePalette::Dark
    }
}

// ---------------------------------------------------------------------------
// Palette table
// ---------------------------------------------------------------------------

/// Densely packed table of every redesign color token per palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteValues {
    // Surfaces
    pub page_bg: Color32,
    pub shell_bg: Color32,
    pub chrome_bg: Color32,
    pub rail_bg: Color32,
    pub input_bg: Color32,
    pub shadow: Color32,

    // Borders
    pub border_strong: Color32,
    pub border_soft: Color32,

    // Text
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub text_fainter: Color32,

    // Status / brand
    pub success: Color32,
    pub status_dot: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_deep: Color32,

    // Pill tones (theme-invariant per SPEC §12.2 — same hex in both palettes).
    // These are pill BACKGROUNDS (bold; intended for dark text on tinted fill).
    pub pill_danger: Color32,
    pub pill_warn: Color32,
    pub pill_info: Color32,
    pub pill_neutral: Color32,
    pub pill_text: Color32,

    // Soft status accents — muted variants intended for subtle borders + hint
    // text, not for pill backgrounds. Used by the Settings → Paths per-row
    // status visuals where the pill_* / success colors read as too aggressive.
    pub warning_soft: Color32,
    pub success_soft: Color32,

    // Selection / hover (rgba per SPEC §12.3).
    pub selection_highlight: Color32,
    pub selection_highlight_hover: Color32,
    pub hover_overlay: Color32,
}

/// Light palette — SPEC §12.1 light table + wireframe `index.html:14-36` for
/// values the spec leaves silent (text_muted/faint/fainter, accent_hover).
pub(crate) const LIGHT: PaletteValues = PaletteValues {
    // Surfaces
    page_bg: Color32::from_rgb(0xe8, 0xee, 0xf5),
    shell_bg: Color32::from_rgb(0xf5, 0xf8, 0xfc),
    chrome_bg: Color32::from_rgb(0xcf, 0xdc, 0xe8),
    rail_bg: Color32::from_rgb(0xdd, 0xe6, 0xf0),
    input_bg: Color32::from_rgb(0xff, 0xff, 0xff),
    shadow: Color32::from_rgb(0x1a, 0x26, 0x38),

    // Borders
    border_strong: Color32::from_rgb(0x1a, 0x26, 0x38),
    border_soft: Color32::from_rgb(0xa5, 0xb4, 0xc7),

    // Text
    text_primary: Color32::from_rgb(0x1a, 0x26, 0x38),
    text_muted: Color32::from_rgb(0x5c, 0x6a, 0x7a),
    text_faint: Color32::from_rgb(0x88, 0x96, 0xa8),
    text_fainter: Color32::from_rgb(0xae, 0xbb, 0xcb),

    // Status / brand
    success: Color32::from_rgb(0x5f, 0xa8, 0x6a),
    status_dot: Color32::from_rgb(0x6f, 0xb8, 0x7a),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x14, 0xB8, 0xA6),
    // accent * 0.6 per wireframe app.jsx::darken — #0C6E64.
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    // Pill tones (SPEC §12.2 — same in both palettes; dark text on tinted bg).
    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    // Soft warning — desaturated amber readable on the light cream surface
    // without the pill-yellow shouting.
    warning_soft: Color32::from_rgb(0xb8, 0x82, 0x36),
    // Soft success — darker muted green for subtle row tinting.
    success_soft: Color32::from_rgb(0x4d, 0x80, 0x55),

    // Selection / hover overlays.
    selection_highlight: Color32::from_rgba_premultiplied(9, 84, 75, 46), // teal @ 18% alpha (premul)
    selection_highlight_hover: Color32::from_rgba_premultiplied(11, 103, 92, 56), // teal @ 22% alpha (premul)
    hover_overlay: Color32::from_rgba_premultiplied(2, 3, 5, 13), // ~5% dark on light
};

/// Dark palette — SPEC §12.1 dark table + wireframe `index.html:38-60`.
pub(crate) const DARK: PaletteValues = PaletteValues {
    // Surfaces
    page_bg: Color32::from_rgb(0x0B, 0x11, 0x16),
    shell_bg: Color32::from_rgb(0x11, 0x1A, 0x21),
    chrome_bg: Color32::from_rgb(0x15, 0x22, 0x2B),
    rail_bg: Color32::from_rgb(0x15, 0x22, 0x2B),
    input_bg: Color32::from_rgb(0x0B, 0x11, 0x16),
    shadow: Color32::from_rgb(0x24, 0x33, 0x3D),

    // Borders
    border_strong: Color32::from_rgb(0x24, 0x33, 0x3D),
    border_soft: Color32::from_rgb(0x24, 0x33, 0x3D),

    // Text
    text_primary: Color32::from_rgb(0xE6, 0xED, 0xF3),
    text_muted: Color32::from_rgb(0xA7, 0xB3, 0xBD),
    text_faint: Color32::from_rgb(0x6B, 0x77, 0x85),
    text_fainter: Color32::from_rgb(0x4d, 0x55, 0x60),

    // Status / brand
    success: Color32::from_rgb(0x4A, 0xDE, 0x80),
    status_dot: Color32::from_rgb(0x4A, 0xDE, 0x80),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x2D, 0xD4, 0xBF),
    // accent * 0.6 per wireframe app.jsx::darken — #0C6E64.
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    // Pill tones (SPEC §12.2 — same in both palettes; dark text on tinted bg).
    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    // Soft warning — muted gold readable against the dark slate input bg.
    warning_soft: Color32::from_rgb(0xa8, 0x8a, 0x4a),
    // Soft success — sage green with a slight teal lean to harmonize with
    // the accent, but desaturated enough to read as a subtle row signal.
    success_soft: Color32::from_rgb(0x5a, 0x90, 0x70),

    // Selection / hover overlays.
    selection_highlight: Color32::from_rgba_premultiplied(9, 84, 75, 46), // teal @ 18% alpha (premul)
    selection_highlight_hover: Color32::from_rgba_premultiplied(11, 103, 92, 56), // teal @ 22% alpha (premul)
    hover_overlay: Color32::from_rgba_premultiplied(9, 9, 10, 10), // ~4% light on dark
};

/// Return the table backing this palette.
#[inline]
pub(crate) const fn values(palette: ThemePalette) -> &'static PaletteValues {
    match palette {
        ThemePalette::Light => &LIGHT,
        ThemePalette::Dark => &DARK,
    }
}

// ---------------------------------------------------------------------------
// Pure-function color accessors
// ---------------------------------------------------------------------------

// Surfaces -------------------------------------------------------------------

pub fn redesign_page_bg(palette: ThemePalette) -> Color32 {
    values(palette).page_bg
}
pub fn redesign_shell_bg(palette: ThemePalette) -> Color32 {
    values(palette).shell_bg
}
pub fn redesign_chrome_bg(palette: ThemePalette) -> Color32 {
    values(palette).chrome_bg
}
pub fn redesign_rail_bg(palette: ThemePalette) -> Color32 {
    values(palette).rail_bg
}
pub fn redesign_input_bg(palette: ThemePalette) -> Color32 {
    values(palette).input_bg
}
pub fn redesign_shadow(palette: ThemePalette) -> Color32 {
    values(palette).shadow
}

// Borders --------------------------------------------------------------------

pub fn redesign_border_strong(palette: ThemePalette) -> Color32 {
    values(palette).border_strong
}
pub fn redesign_border_soft(palette: ThemePalette) -> Color32 {
    values(palette).border_soft
}

// Text -----------------------------------------------------------------------

pub fn redesign_text_primary(palette: ThemePalette) -> Color32 {
    values(palette).text_primary
}
pub fn redesign_text_muted(palette: ThemePalette) -> Color32 {
    values(palette).text_muted
}
pub fn redesign_text_faint(palette: ThemePalette) -> Color32 {
    values(palette).text_faint
}
pub fn redesign_text_fainter(palette: ThemePalette) -> Color32 {
    values(palette).text_fainter
}

// Status / brand -------------------------------------------------------------

pub fn redesign_success(palette: ThemePalette) -> Color32 {
    values(palette).success
}
pub fn redesign_status_dot(palette: ThemePalette) -> Color32 {
    values(palette).status_dot
}
pub fn redesign_accent(palette: ThemePalette) -> Color32 {
    values(palette).accent
}
pub fn redesign_accent_hover(palette: ThemePalette) -> Color32 {
    values(palette).accent_hover
}
pub fn redesign_accent_deep(palette: ThemePalette) -> Color32 {
    values(palette).accent_deep
}

// Pill tones (SPEC §12.2) ----------------------------------------------------

pub fn redesign_pill_danger(palette: ThemePalette) -> Color32 {
    values(palette).pill_danger
}
pub fn redesign_pill_warn(palette: ThemePalette) -> Color32 {
    values(palette).pill_warn
}
pub fn redesign_warning_soft(palette: ThemePalette) -> Color32 {
    values(palette).warning_soft
}
pub fn redesign_success_soft(palette: ThemePalette) -> Color32 {
    values(palette).success_soft
}
pub fn redesign_pill_info(palette: ThemePalette) -> Color32 {
    values(palette).pill_info
}
pub fn redesign_pill_neutral(palette: ThemePalette) -> Color32 {
    values(palette).pill_neutral
}
pub fn redesign_pill_text(palette: ThemePalette) -> Color32 {
    values(palette).pill_text
}

// Selection / hover overlays (SPEC §12.3) ------------------------------------

pub fn redesign_selection_highlight(palette: ThemePalette) -> Color32 {
    values(palette).selection_highlight
}
pub fn redesign_selection_highlight_hover(palette: ThemePalette) -> Color32 {
    values(palette).selection_highlight_hover
}
pub fn redesign_hover_overlay(palette: ThemePalette) -> Color32 {
    values(palette).hover_overlay
}

// ---------------------------------------------------------------------------
// Layout / spacing tokens (P1.T3)
// ---------------------------------------------------------------------------

/// Inner section border (Box, pill, button outline) — SPEC §1.2 (1.5px solid).
pub const REDESIGN_BORDER_WIDTH_PX: f32 = 1.5;
/// Outer shell frame border (titlebar / statusbar / shell) — wireframe
/// `index.html:81` (`border: 2px solid var(--border-strong)`).
pub const REDESIGN_SHELL_BORDER_WIDTH_PX: f32 = 2.0;
/// Default rounded-corner radius for boxes / pills — SPEC §1.2.
pub const REDESIGN_BORDER_RADIUS_PX: f32 = 3.0;
/// Main shell drop-shadow offset — SPEC §12.3 (6×6).
pub const REDESIGN_SHADOW_OFFSET_PX: f32 = 6.0;
/// Primary-button drop-shadow offset — SPEC §12.3 (2×2).
pub const REDESIGN_SHADOW_OFFSET_BTN_PX: f32 = 2.0;
/// Custom titlebar height — SPEC §1.2 + wireframe `index.html:89-90`.
pub const REDESIGN_TITLEBAR_HEIGHT_PX: f32 = 34.0;
/// Footer status bar height — SPEC §1.2 (26px always visible).
pub const REDESIGN_STATUSBAR_HEIGHT_PX: f32 = 26.0;
/// Left-rail width in labels mode — SPEC §2.1.
pub const REDESIGN_NAV_WIDTH_PX: f32 = 200.0;
/// Background dot spacing for the radial pattern — wireframe `index.html:67`.
pub const REDESIGN_DOT_BG_SPACING_PX: f32 = 20.0;
/// Horizontal page padding inside the body content area.
pub const REDESIGN_PAGE_PADDING_X_PX: f32 = 28.0;
/// Vertical page padding inside the body content area.
pub const REDESIGN_PAGE_PADDING_Y_PX: f32 = 24.0;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_shell_bg_matches_spec() {
        // SPEC §12.1 Light --shell-bg == #f5f8fc.
        let c = redesign_shell_bg(ThemePalette::Light);
        assert_eq!((c.r(), c.g(), c.b()), (0xf5, 0xf8, 0xfc));
    }

    #[test]
    fn dark_shell_bg_matches_spec() {
        // SPEC §12.1 Dark --shell-bg == #111A21.
        let c = redesign_shell_bg(ThemePalette::Dark);
        assert_eq!((c.r(), c.g(), c.b()), (0x11, 0x1A, 0x21));
    }

    #[test]
    fn dark_accent_hover_matches_spec() {
        let c = redesign_accent_hover(ThemePalette::Dark);
        assert_eq!((c.r(), c.g(), c.b()), (0x2D, 0xD4, 0xBF));
    }

    #[test]
    fn pill_tones_are_palette_invariant() {
        // SPEC §12.2 says pills are the same hex across palettes.
        assert_eq!(
            redesign_pill_danger(ThemePalette::Light),
            redesign_pill_danger(ThemePalette::Dark)
        );
        assert_eq!(
            redesign_pill_warn(ThemePalette::Light),
            redesign_pill_warn(ThemePalette::Dark)
        );
        assert_eq!(
            redesign_pill_info(ThemePalette::Light),
            redesign_pill_info(ThemePalette::Dark)
        );
        assert_eq!(
            redesign_pill_neutral(ThemePalette::Light),
            redesign_pill_neutral(ThemePalette::Dark)
        );
        assert_eq!(
            redesign_pill_text(ThemePalette::Light),
            redesign_pill_text(ThemePalette::Dark)
        );
    }

    #[test]
    fn layout_constants_present() {
        assert_eq!(REDESIGN_BORDER_WIDTH_PX, 1.5);
        assert_eq!(REDESIGN_SHELL_BORDER_WIDTH_PX, 2.0);
        assert_eq!(REDESIGN_TITLEBAR_HEIGHT_PX, 34.0);
        assert_eq!(REDESIGN_STATUSBAR_HEIGHT_PX, 26.0);
        assert_eq!(REDESIGN_NAV_WIDTH_PX, 200.0);
    }

    #[test]
    fn theme_palette_default_is_dark() {
        assert_eq!(ThemePalette::default(), ThemePalette::Dark);
    }
}
