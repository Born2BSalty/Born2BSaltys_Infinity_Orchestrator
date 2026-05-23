// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemePalette {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PaletteValues {
    pub page_bg: Color32,
    pub shell_bg: Color32,
    pub chrome_bg: Color32,
    pub rail_bg: Color32,
    pub input_bg: Color32,
    pub shadow: Color32,

    pub border_strong: Color32,
    pub border_soft: Color32,

    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_faint: Color32,
    pub text_fainter: Color32,
    pub text_disabled: Color32,

    pub success: Color32,
    pub success_bright: Color32,
    pub status_dot: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_deep: Color32,

    pub accent_path: Color32,
    pub accent_numbers: Color32,
    pub accent_comment: Color32,

    pub pill_danger: Color32,
    pub pill_warn: Color32,
    pub pill_info: Color32,
    pub pill_neutral: Color32,
    pub pill_text: Color32,

    pub warning: Color32,
    pub warning_soft: Color32,
    pub warning_emphasis: Color32,
    pub warning_fill: Color32,
    pub warning_parent: Color32,
    pub success_soft: Color32,

    pub error: Color32,
    pub error_emphasis: Color32,

    pub conflict: Color32,
    pub conflict_fill: Color32,
    pub conflict_parent: Color32,

    pub included: Color32,
    pub included_fill: Color32,

    pub info: Color32,
    pub info_fill: Color32,

    pub game_mismatch: Color32,
    pub game_mismatch_fill: Color32,

    pub conditional: Color32,
    pub conditional_fill: Color32,

    pub prompt_text: Color32,
    pub prompt_fill: Color32,
    pub prompt_stroke: Color32,

    pub status_running: Color32,
    pub status_preparing: Color32,
    pub status_idle: Color32,

    pub terminal_default: Color32,
    pub terminal_error: Color32,
    pub terminal_debug: Color32,
    pub terminal_sent: Color32,
    pub terminal_info: Color32,
    pub terminal_amber: Color32,
    pub terminal_sand: Color32,
    pub terminal_dim: Color32,

    pub selection_highlight: Color32,
    pub selection_highlight_hover: Color32,
    pub hover_overlay: Color32,

    pub dot_bg: Color32,
}

pub(crate) const LIGHT: PaletteValues = PaletteValues {
    page_bg: Color32::from_rgb(0xe8, 0xee, 0xf5),
    shell_bg: Color32::from_rgb(0xf5, 0xf8, 0xfc),
    chrome_bg: Color32::from_rgb(0xcf, 0xdc, 0xe8),
    rail_bg: Color32::from_rgb(0xdd, 0xe6, 0xf0),
    input_bg: Color32::from_rgb(0xff, 0xff, 0xff),
    shadow: Color32::from_rgb(0x1a, 0x26, 0x38),

    border_strong: Color32::from_rgb(0x1a, 0x26, 0x38),
    border_soft: Color32::from_rgb(0xa5, 0xb4, 0xc7),

    text_primary: Color32::from_rgb(0x1a, 0x26, 0x38),
    text_muted: Color32::from_rgb(0x5c, 0x6a, 0x7a),
    text_faint: Color32::from_rgb(0x88, 0x96, 0xa8),
    text_fainter: Color32::from_rgb(0xae, 0xbb, 0xcb),
    text_disabled: Color32::from_rgb(0x78, 0x78, 0x78),

    success: Color32::from_rgb(0x5f, 0xa8, 0x6a),
    success_bright: Color32::from_rgb(0x50, 0xa0, 0x58),
    status_dot: Color32::from_rgb(0x6f, 0xb8, 0x7a),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    accent_path: Color32::from_rgb(0x7a, 0x56, 0x28),
    accent_numbers: Color32::from_rgb(0x28, 0x4e, 0x9a),
    accent_comment: Color32::from_rgb(0x3a, 0x7a, 0x3a),

    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    warning: Color32::from_rgb(0x9a, 0x6a, 0x18),
    warning_soft: Color32::from_rgb(0xb8, 0x82, 0x36),
    warning_emphasis: Color32::from_rgb(0x9a, 0x68, 0x18),
    warning_fill: Color32::from_rgb(0xe0, 0xcc, 0xa0),
    warning_parent: Color32::from_rgb(0x9e, 0x70, 0x1c),
    success_soft: Color32::from_rgb(0x4d, 0x80, 0x55),

    error: Color32::from_rgb(0xc0, 0x30, 0x30),
    error_emphasis: Color32::from_rgb(0xc0, 0x28, 0x28),

    conflict: Color32::from_rgb(0xb0, 0x28, 0x28),
    conflict_fill: Color32::from_rgb(0xf0, 0xcc, 0xcc),
    conflict_parent: Color32::from_rgb(0xb8, 0x38, 0x38),

    included: Color32::from_rgb(0x60, 0x60, 0x60),
    included_fill: Color32::from_rgb(0xc8, 0xd4, 0xc8),

    info: Color32::from_rgb(0x28, 0x6a, 0x9e),
    info_fill: Color32::from_rgb(0xc0, 0xd8, 0xee),

    game_mismatch: Color32::from_rgb(0x7a, 0x28, 0x78),
    game_mismatch_fill: Color32::from_rgb(0xe0, 0xc0, 0xde),

    conditional: Color32::from_rgb(0x44, 0x5e, 0x78),
    conditional_fill: Color32::from_rgb(0xcc, 0xd8, 0xe4),

    prompt_text: Color32::from_rgb(0x20, 0x10, 0x00),
    prompt_fill: Color32::from_rgb(0xf8, 0xcc, 0x68),
    prompt_stroke: Color32::from_rgb(0xd8, 0xa8, 0x50),

    status_running: Color32::from_rgb(0x4a, 0x78, 0x20),
    status_preparing: Color32::from_rgb(0x50, 0x48, 0x88),
    status_idle: Color32::from_rgb(0x6a, 0x6a, 0x6a),

    terminal_default: Color32::from_rgb(0x28, 0x28, 0x28),
    terminal_error: Color32::from_rgb(0xb8, 0x28, 0x28),
    terminal_debug: Color32::from_rgb(0x28, 0x50, 0xaa),
    terminal_sent: Color32::from_rgb(0x28, 0x68, 0xaa),
    terminal_info: Color32::from_rgb(0x4a, 0x78, 0x20),
    terminal_amber: Color32::from_rgb(0x9a, 0x6a, 0x18),
    terminal_sand: Color32::from_rgb(0x7e, 0x60, 0x38),
    terminal_dim: Color32::from_rgb(0x68, 0x68, 0x68),

    selection_highlight: Color32::from_rgba_premultiplied(9, 84, 75, 46),
    selection_highlight_hover: Color32::from_rgba_premultiplied(11, 103, 92, 56),
    hover_overlay: Color32::from_rgba_premultiplied(2, 3, 5, 13),

    dot_bg: Color32::from_rgba_premultiplied(2, 3, 4, 20),
};

pub(crate) const DARK: PaletteValues = PaletteValues {
    page_bg: Color32::from_rgb(0x0B, 0x11, 0x16),
    shell_bg: Color32::from_rgb(0x11, 0x1A, 0x21),
    chrome_bg: Color32::from_rgb(0x15, 0x22, 0x2B),
    rail_bg: Color32::from_rgb(0x15, 0x22, 0x2B),
    input_bg: Color32::from_rgb(0x0B, 0x11, 0x16),
    shadow: Color32::from_rgb(0x24, 0x33, 0x3D),

    border_strong: Color32::from_rgb(0x24, 0x33, 0x3D),
    border_soft: Color32::from_rgb(0x24, 0x33, 0x3D),

    text_primary: Color32::from_rgb(0xE6, 0xED, 0xF3),
    text_muted: Color32::from_rgb(0xA7, 0xB3, 0xBD),
    text_faint: Color32::from_rgb(0x6B, 0x77, 0x85),
    text_fainter: Color32::from_rgb(0x4d, 0x55, 0x60),
    text_disabled: Color32::from_rgb(0x82, 0x82, 0x82),

    success: Color32::from_rgb(0x4A, 0xDE, 0x80),
    success_bright: Color32::from_rgb(0x78, 0xC8, 0x78),
    status_dot: Color32::from_rgb(0x4A, 0xDE, 0x80),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x2D, 0xD4, 0xBF),
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    accent_path: Color32::from_rgb(0xE0, 0xC4, 0x9C),
    accent_numbers: Color32::from_rgb(0x88, 0xB0, 0xFF),
    accent_comment: Color32::from_rgb(0x7C, 0xC4, 0x7C),

    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    warning: Color32::from_rgb(0xD6, 0xA8, 0x60),
    warning_soft: Color32::from_rgb(0xa8, 0x8a, 0x4a),
    warning_emphasis: Color32::from_rgb(0xD6, 0xAE, 0x54),
    warning_fill: Color32::from_rgb(0x4E, 0x3E, 0x22),
    warning_parent: Color32::from_rgb(0xDE, 0xB6, 0x5C),
    success_soft: Color32::from_rgb(0x5a, 0x90, 0x70),

    error: Color32::from_rgb(0xDC, 0x64, 0x64),
    error_emphasis: Color32::from_rgb(0xDC, 0x60, 0x60),

    conflict: Color32::from_rgb(0xD0, 0x60, 0x60),
    conflict_fill: Color32::from_rgb(0x58, 0x2C, 0x2C),
    conflict_parent: Color32::from_rgb(0xDC, 0x7A, 0x7A),

    included: Color32::from_rgb(0x96, 0x96, 0x96),
    included_fill: Color32::from_rgb(0x38, 0x48, 0x38),

    info: Color32::from_rgb(0x78, 0xBA, 0xE6),
    info_fill: Color32::from_rgb(0x2A, 0x42, 0x56),

    game_mismatch: Color32::from_rgb(0xCB, 0x6E, 0xBC),
    game_mismatch_fill: Color32::from_rgb(0x4E, 0x2C, 0x54),

    conditional: Color32::from_rgb(0xA4, 0xBE, 0xD0),
    conditional_fill: Color32::from_rgb(0x34, 0x42, 0x4E),

    prompt_text: Color32::from_rgb(0x28, 0x14, 0x00),
    prompt_fill: Color32::from_rgb(0xF5, 0xC3, 0x5F),
    prompt_stroke: Color32::from_rgb(0xD2, 0xA0, 0x46),

    status_running: Color32::from_rgb(0xA8, 0xCC, 0x62),
    status_preparing: Color32::from_rgb(0xB4, 0xAA, 0xDC),
    status_idle: Color32::from_rgb(0xAA, 0xAA, 0xAA),

    terminal_default: Color32::from_rgb(0xD2, 0xD2, 0xD2),
    terminal_error: Color32::from_rgb(0xE6, 0x60, 0x60),
    terminal_debug: Color32::from_rgb(0x46, 0x6E, 0xB4),
    terminal_sent: Color32::from_rgb(0x6E, 0xBE, 0xFF),
    terminal_info: Color32::from_rgb(0xA8, 0xCC, 0x62),
    terminal_amber: Color32::from_rgb(0xD6, 0xA8, 0x60),
    terminal_sand: Color32::from_rgb(0xD6, 0xB6, 0x92),
    terminal_dim: Color32::from_rgb(0x96, 0x96, 0x96),

    selection_highlight: Color32::from_rgba_premultiplied(9, 84, 75, 46),
    selection_highlight_hover: Color32::from_rgba_premultiplied(11, 103, 92, 56),
    hover_overlay: Color32::from_rgba_premultiplied(9, 9, 10, 10),

    dot_bg: Color32::from_rgba_premultiplied(12, 12, 12, 13),
};

#[inline]
pub(crate) const fn values(palette: ThemePalette) -> &'static PaletteValues {
    match palette {
        ThemePalette::Light => &LIGHT,
        ThemePalette::Dark => &DARK,
    }
}

#[must_use]
pub const fn redesign_page_bg(palette: ThemePalette) -> Color32 {
    values(palette).page_bg
}
#[must_use]
pub const fn redesign_shell_bg(palette: ThemePalette) -> Color32 {
    values(palette).shell_bg
}
#[must_use]
pub const fn redesign_chrome_bg(palette: ThemePalette) -> Color32 {
    values(palette).chrome_bg
}
#[must_use]
pub const fn redesign_rail_bg(palette: ThemePalette) -> Color32 {
    values(palette).rail_bg
}
#[must_use]
pub const fn redesign_input_bg(palette: ThemePalette) -> Color32 {
    values(palette).input_bg
}
#[must_use]
pub const fn redesign_shadow(palette: ThemePalette) -> Color32 {
    values(palette).shadow
}

#[must_use]
pub const fn redesign_border_strong(palette: ThemePalette) -> Color32 {
    values(palette).border_strong
}
#[must_use]
pub const fn redesign_border_soft(palette: ThemePalette) -> Color32 {
    values(palette).border_soft
}

#[must_use]
pub const fn redesign_text_primary(palette: ThemePalette) -> Color32 {
    values(palette).text_primary
}
#[must_use]
pub const fn redesign_text_muted(palette: ThemePalette) -> Color32 {
    values(palette).text_muted
}
#[must_use]
pub const fn redesign_text_faint(palette: ThemePalette) -> Color32 {
    values(palette).text_faint
}
#[must_use]
pub const fn redesign_text_fainter(palette: ThemePalette) -> Color32 {
    values(palette).text_fainter
}

#[must_use]
pub const fn redesign_success(palette: ThemePalette) -> Color32 {
    values(palette).success
}
#[must_use]
pub const fn redesign_status_dot(palette: ThemePalette) -> Color32 {
    values(palette).status_dot
}
#[must_use]
pub const fn redesign_accent(palette: ThemePalette) -> Color32 {
    values(palette).accent
}
#[must_use]
pub const fn redesign_accent_hover(palette: ThemePalette) -> Color32 {
    values(palette).accent_hover
}
#[must_use]
pub const fn redesign_accent_deep(palette: ThemePalette) -> Color32 {
    values(palette).accent_deep
}

#[must_use]
pub const fn redesign_pill_danger(palette: ThemePalette) -> Color32 {
    values(palette).pill_danger
}
#[must_use]
pub const fn redesign_pill_warn(palette: ThemePalette) -> Color32 {
    values(palette).pill_warn
}
#[must_use]
pub const fn redesign_warning_soft(palette: ThemePalette) -> Color32 {
    values(palette).warning_soft
}
#[must_use]
pub const fn redesign_success_soft(palette: ThemePalette) -> Color32 {
    values(palette).success_soft
}
#[must_use]
pub const fn redesign_pill_info(palette: ThemePalette) -> Color32 {
    values(palette).pill_info
}
#[must_use]
pub const fn redesign_pill_neutral(palette: ThemePalette) -> Color32 {
    values(palette).pill_neutral
}
#[must_use]
pub const fn redesign_pill_text(palette: ThemePalette) -> Color32 {
    values(palette).pill_text
}

#[must_use]
pub const fn redesign_selection_highlight(palette: ThemePalette) -> Color32 {
    values(palette).selection_highlight
}
#[must_use]
pub const fn redesign_selection_highlight_hover(palette: ThemePalette) -> Color32 {
    values(palette).selection_highlight_hover
}
#[must_use]
pub const fn redesign_hover_overlay(palette: ThemePalette) -> Color32 {
    values(palette).hover_overlay
}
#[must_use]
pub const fn redesign_dot(palette: ThemePalette) -> Color32 {
    values(palette).dot_bg
}

#[must_use]
pub const fn redesign_text_disabled(palette: ThemePalette) -> Color32 {
    values(palette).text_disabled
}

#[must_use]
pub const fn redesign_success_bright(palette: ThemePalette) -> Color32 {
    values(palette).success_bright
}

#[must_use]
pub const fn redesign_warning(palette: ThemePalette) -> Color32 {
    values(palette).warning
}

#[must_use]
pub const fn redesign_warning_emphasis(palette: ThemePalette) -> Color32 {
    values(palette).warning_emphasis
}

#[must_use]
pub const fn redesign_warning_fill(palette: ThemePalette) -> Color32 {
    values(palette).warning_fill
}

#[must_use]
pub const fn redesign_warning_parent(palette: ThemePalette) -> Color32 {
    values(palette).warning_parent
}

#[must_use]
pub const fn redesign_error(palette: ThemePalette) -> Color32 {
    values(palette).error
}

#[must_use]
pub const fn redesign_error_emphasis(palette: ThemePalette) -> Color32 {
    values(palette).error_emphasis
}

#[must_use]
pub const fn redesign_conflict(palette: ThemePalette) -> Color32 {
    values(palette).conflict
}

#[must_use]
pub const fn redesign_conflict_fill(palette: ThemePalette) -> Color32 {
    values(palette).conflict_fill
}

#[must_use]
pub const fn redesign_conflict_parent(palette: ThemePalette) -> Color32 {
    values(palette).conflict_parent
}

#[must_use]
pub const fn redesign_included(palette: ThemePalette) -> Color32 {
    values(palette).included
}

#[must_use]
pub const fn redesign_included_fill(palette: ThemePalette) -> Color32 {
    values(palette).included_fill
}

#[must_use]
pub const fn redesign_info(palette: ThemePalette) -> Color32 {
    values(palette).info
}

#[must_use]
pub const fn redesign_info_fill(palette: ThemePalette) -> Color32 {
    values(palette).info_fill
}

#[must_use]
pub const fn redesign_game_mismatch(palette: ThemePalette) -> Color32 {
    values(palette).game_mismatch
}

#[must_use]
pub const fn redesign_game_mismatch_fill(palette: ThemePalette) -> Color32 {
    values(palette).game_mismatch_fill
}

#[must_use]
pub const fn redesign_conditional(palette: ThemePalette) -> Color32 {
    values(palette).conditional
}

#[must_use]
pub const fn redesign_conditional_fill(palette: ThemePalette) -> Color32 {
    values(palette).conditional_fill
}

#[must_use]
pub const fn redesign_accent_path(palette: ThemePalette) -> Color32 {
    values(palette).accent_path
}

#[must_use]
pub const fn redesign_accent_numbers(palette: ThemePalette) -> Color32 {
    values(palette).accent_numbers
}

#[must_use]
pub const fn redesign_accent_comment(palette: ThemePalette) -> Color32 {
    values(palette).accent_comment
}

#[must_use]
pub const fn redesign_prompt_text(palette: ThemePalette) -> Color32 {
    values(palette).prompt_text
}

#[must_use]
pub const fn redesign_prompt_fill(palette: ThemePalette) -> Color32 {
    values(palette).prompt_fill
}

#[must_use]
pub const fn redesign_prompt_stroke(palette: ThemePalette) -> Color32 {
    values(palette).prompt_stroke
}

#[must_use]
pub const fn redesign_status_running(palette: ThemePalette) -> Color32 {
    values(palette).status_running
}

#[must_use]
pub const fn redesign_status_preparing(palette: ThemePalette) -> Color32 {
    values(palette).status_preparing
}

#[must_use]
pub const fn redesign_status_idle(palette: ThemePalette) -> Color32 {
    values(palette).status_idle
}

#[must_use]
pub const fn redesign_terminal_default(palette: ThemePalette) -> Color32 {
    values(palette).terminal_default
}

#[must_use]
pub const fn redesign_terminal_error(palette: ThemePalette) -> Color32 {
    values(palette).terminal_error
}

#[must_use]
pub const fn redesign_terminal_debug(palette: ThemePalette) -> Color32 {
    values(palette).terminal_debug
}

#[must_use]
pub const fn redesign_terminal_sent(palette: ThemePalette) -> Color32 {
    values(palette).terminal_sent
}

#[must_use]
pub const fn redesign_terminal_info(palette: ThemePalette) -> Color32 {
    values(palette).terminal_info
}

#[must_use]
pub const fn redesign_terminal_amber(palette: ThemePalette) -> Color32 {
    values(palette).terminal_amber
}

#[must_use]
pub const fn redesign_terminal_sand(palette: ThemePalette) -> Color32 {
    values(palette).terminal_sand
}

#[must_use]
pub const fn redesign_terminal_dim(palette: ThemePalette) -> Color32 {
    values(palette).terminal_dim
}

pub const REDESIGN_BORDER_WIDTH_PX: f32 = 1.5;
pub const REDESIGN_SHELL_BORDER_WIDTH_PX: f32 = 2.0;
pub const REDESIGN_BORDER_RADIUS_PX: f32 = 3.0;
pub const REDESIGN_BORDER_RADIUS_U8: u8 = 3;
pub const REDESIGN_PANEL_RADIUS_U8: u8 = 11;
pub const REDESIGN_SHADOW_OFFSET_PX: f32 = 6.0;
pub const REDESIGN_SHADOW_OFFSET_I8: i8 = 6;
pub const REDESIGN_SHADOW_OFFSET_BTN_PX: f32 = 2.0;
pub const REDESIGN_SHADOW_OFFSET_BTN_I8: i8 = 2;
pub const REDESIGN_TITLEBAR_HEIGHT_PX: f32 = 34.0;
pub const REDESIGN_STATUSBAR_HEIGHT_PX: f32 = 26.0;
pub const REDESIGN_NAV_WIDTH_PX: f32 = 200.0;
pub const REDESIGN_DOT_BG_SPACING_PX: f32 = 20.0;
pub const REDESIGN_PAGE_PADDING_X_PX: f32 = 28.0;
pub const REDESIGN_PAGE_PADDING_Y_PX: f32 = 24.0;

pub const WORKSPACE_CONTENT_TEXT_INSET: f32 = 0.0;

#[must_use]
pub fn redesign_with_alpha(c: Color32, numerator: u16, denominator: u16) -> Color32 {
    let denominator = denominator.max(1);
    let numerator = numerator.min(denominator);
    let alpha = u16::from(c.a()).saturating_mul(numerator) / denominator;
    let alpha = u8::try_from(alpha).unwrap_or(u8::MAX);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_shell_bg_matches_spec() {
        let c = redesign_shell_bg(ThemePalette::Light);
        assert_eq!((c.r(), c.g(), c.b()), (0xf5, 0xf8, 0xfc));
    }

    #[test]
    fn dark_shell_bg_matches_spec() {
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
        const {
            assert!(REDESIGN_BORDER_WIDTH_PX.to_bits() == 1.5_f32.to_bits());
            assert!(REDESIGN_SHELL_BORDER_WIDTH_PX.to_bits() == 2.0_f32.to_bits());
            assert!(REDESIGN_TITLEBAR_HEIGHT_PX.to_bits() == 34.0_f32.to_bits());
            assert!(REDESIGN_STATUSBAR_HEIGHT_PX.to_bits() == 26.0_f32.to_bits());
            assert!(REDESIGN_NAV_WIDTH_PX.to_bits() == 200.0_f32.to_bits());
        }
    }

    #[test]
    fn theme_palette_default_is_dark() {
        assert_eq!(ThemePalette::default(), ThemePalette::Dark);
    }

    #[test]
    fn dot_bg_differs_between_palettes() {
        assert_ne!(
            redesign_dot(ThemePalette::Light),
            redesign_dot(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_text_disabled_light_differs_from_dark() {
        assert_ne!(
            redesign_text_disabled(ThemePalette::Light),
            redesign_text_disabled(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_success_bright_light_differs_from_dark() {
        assert_ne!(
            redesign_success_bright(ThemePalette::Light),
            redesign_success_bright(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_warning_light_differs_from_dark() {
        assert_ne!(
            redesign_warning(ThemePalette::Light),
            redesign_warning(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_warning_emphasis_light_differs_from_dark() {
        assert_ne!(
            redesign_warning_emphasis(ThemePalette::Light),
            redesign_warning_emphasis(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_warning_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_warning_fill(ThemePalette::Light),
            redesign_warning_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_warning_parent_light_differs_from_dark() {
        assert_ne!(
            redesign_warning_parent(ThemePalette::Light),
            redesign_warning_parent(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_error_light_differs_from_dark() {
        assert_ne!(
            redesign_error(ThemePalette::Light),
            redesign_error(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_error_emphasis_light_differs_from_dark() {
        assert_ne!(
            redesign_error_emphasis(ThemePalette::Light),
            redesign_error_emphasis(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_conflict_light_differs_from_dark() {
        assert_ne!(
            redesign_conflict(ThemePalette::Light),
            redesign_conflict(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_conflict_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_conflict_fill(ThemePalette::Light),
            redesign_conflict_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_conflict_parent_light_differs_from_dark() {
        assert_ne!(
            redesign_conflict_parent(ThemePalette::Light),
            redesign_conflict_parent(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_included_light_differs_from_dark() {
        assert_ne!(
            redesign_included(ThemePalette::Light),
            redesign_included(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_included_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_included_fill(ThemePalette::Light),
            redesign_included_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_info_light_differs_from_dark() {
        assert_ne!(
            redesign_info(ThemePalette::Light),
            redesign_info(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_info_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_info_fill(ThemePalette::Light),
            redesign_info_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_game_mismatch_light_differs_from_dark() {
        assert_ne!(
            redesign_game_mismatch(ThemePalette::Light),
            redesign_game_mismatch(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_game_mismatch_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_game_mismatch_fill(ThemePalette::Light),
            redesign_game_mismatch_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_conditional_light_differs_from_dark() {
        assert_ne!(
            redesign_conditional(ThemePalette::Light),
            redesign_conditional(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_conditional_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_conditional_fill(ThemePalette::Light),
            redesign_conditional_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_accent_path_light_differs_from_dark() {
        assert_ne!(
            redesign_accent_path(ThemePalette::Light),
            redesign_accent_path(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_accent_numbers_light_differs_from_dark() {
        assert_ne!(
            redesign_accent_numbers(ThemePalette::Light),
            redesign_accent_numbers(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_accent_comment_light_differs_from_dark() {
        assert_ne!(
            redesign_accent_comment(ThemePalette::Light),
            redesign_accent_comment(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_prompt_text_light_differs_from_dark() {
        assert_ne!(
            redesign_prompt_text(ThemePalette::Light),
            redesign_prompt_text(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_prompt_fill_light_differs_from_dark() {
        assert_ne!(
            redesign_prompt_fill(ThemePalette::Light),
            redesign_prompt_fill(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_prompt_stroke_light_differs_from_dark() {
        assert_ne!(
            redesign_prompt_stroke(ThemePalette::Light),
            redesign_prompt_stroke(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_status_running_light_differs_from_dark() {
        assert_ne!(
            redesign_status_running(ThemePalette::Light),
            redesign_status_running(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_status_preparing_light_differs_from_dark() {
        assert_ne!(
            redesign_status_preparing(ThemePalette::Light),
            redesign_status_preparing(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_status_idle_light_differs_from_dark() {
        assert_ne!(
            redesign_status_idle(ThemePalette::Light),
            redesign_status_idle(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_default_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_default(ThemePalette::Light),
            redesign_terminal_default(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_error_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_error(ThemePalette::Light),
            redesign_terminal_error(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_debug_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_debug(ThemePalette::Light),
            redesign_terminal_debug(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_sent_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_sent(ThemePalette::Light),
            redesign_terminal_sent(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_info_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_info(ThemePalette::Light),
            redesign_terminal_info(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_amber_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_amber(ThemePalette::Light),
            redesign_terminal_amber(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_sand_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_sand(ThemePalette::Light),
            redesign_terminal_sand(ThemePalette::Dark)
        );
    }

    #[test]
    fn redesign_terminal_dim_light_differs_from_dark() {
        assert_ne!(
            redesign_terminal_dim(ThemePalette::Light),
            redesign_terminal_dim(ThemePalette::Dark)
        );
    }
}
