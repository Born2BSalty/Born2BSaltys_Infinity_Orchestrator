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

    pub success: Color32,
    pub status_dot: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_deep: Color32,

    pub pill_danger: Color32,
    pub pill_warn: Color32,
    pub pill_info: Color32,
    pub pill_neutral: Color32,
    pub pill_text: Color32,

    pub warning_soft: Color32,
    pub success_soft: Color32,

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

    success: Color32::from_rgb(0x5f, 0xa8, 0x6a),
    status_dot: Color32::from_rgb(0x6f, 0xb8, 0x7a),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    warning_soft: Color32::from_rgb(0xb8, 0x82, 0x36),
    success_soft: Color32::from_rgb(0x4d, 0x80, 0x55),

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

    success: Color32::from_rgb(0x4A, 0xDE, 0x80),
    status_dot: Color32::from_rgb(0x4A, 0xDE, 0x80),
    accent: Color32::from_rgb(0x14, 0xB8, 0xA6),
    accent_hover: Color32::from_rgb(0x2D, 0xD4, 0xBF),
    accent_deep: Color32::from_rgb(0x0C, 0x6E, 0x64),

    pill_danger: Color32::from_rgb(0xe6, 0x9a, 0x96),
    pill_warn: Color32::from_rgb(0xe8, 0xc4, 0x41),
    pill_info: Color32::from_rgb(0xa8, 0xd2, 0xcc),
    pill_neutral: Color32::from_rgb(0xc4, 0xca, 0xd1),
    pill_text: Color32::from_rgb(0x1a, 0x26, 0x38),

    warning_soft: Color32::from_rgb(0xa8, 0x8a, 0x4a),
    success_soft: Color32::from_rgb(0x5a, 0x90, 0x70),

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
}
