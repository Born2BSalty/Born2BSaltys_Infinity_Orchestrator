// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePalette {
    Light,
    Dark,
}

pub const REDESIGN_BORDER_WIDTH_PX: f32 = 1.5;
pub const REDESIGN_DASHED_BORDER_WIDTH_PX: f32 = 1.0;
pub const REDESIGN_SHELL_BORDER_WIDTH_PX: f32 = 2.0;
pub const REDESIGN_BORDER_RADIUS_PX: f32 = 3.0;
pub const REDESIGN_SHELL_RADIUS_PX: f32 = 6.0;
pub const REDESIGN_APP_PADDING_PX: f32 = 24.0;
pub const REDESIGN_APP_MAX_WIDTH_PX: f32 = 1280.0;
pub const REDESIGN_SHELL_MIN_HEIGHT_PX: f32 = 700.0;
pub const REDESIGN_SHADOW_OFFSET_PX: f32 = 6.0;
pub const REDESIGN_SHADOW_OFFSET_BTN_PX: f32 = 2.0;
pub const REDESIGN_TITLEBAR_HEIGHT_PX: f32 = 34.0;
pub const REDESIGN_TITLEBAR_PADDING_X_PX: f32 = 12.0;
pub const REDESIGN_TITLEBAR_DOT_RADIUS_PX: f32 = 6.0;
pub const REDESIGN_TITLEBAR_DOT_GAP_PX: f32 = 6.0;
pub const REDESIGN_TITLEBAR_DOT_STROKE_PX: f32 = 1.2;
pub const REDESIGN_TITLEBAR_CONTROL_GAP_PX: f32 = 14.0;
pub const REDESIGN_TITLEBAR_CONTROL_WIDTH_PX: f32 = 18.0;
pub const REDESIGN_TITLEBAR_CONTROL_GLYPH_SIZE_PX: f32 = 8.0;
pub const REDESIGN_TITLEBAR_FONT_SIZE_PX: f32 = 10.0;
pub const REDESIGN_TITLEBAR_CONTROL_FONT_SIZE_PX: f32 = 12.0;
pub const REDESIGN_STATUSBAR_HEIGHT_PX: f32 = 26.0;
pub const REDESIGN_STATUSBAR_PADDING_X_PX: f32 = 12.0;
pub const REDESIGN_STATUSBAR_FONT_SIZE_PX: f32 = 10.0;
pub const REDESIGN_NAV_WIDTH_PX: f32 = 200.0;
pub const REDESIGN_NAV_ITEM_RADIUS_PX: f32 = 4.0;
pub const REDESIGN_NAV_ITEM_PADDING_X_PX: f32 = 10.0;
pub const REDESIGN_NAV_ITEM_PADDING_Y_PX: f32 = 8.0;
pub const REDESIGN_NAV_ITEM_GAP_PX: f32 = 10.0;
pub const REDESIGN_NAV_ITEM_ICON_WIDTH_PX: f32 = 22.0;
pub const REDESIGN_NAV_ITEM_ICON_FONT_SIZE_PX: f32 = 18.0;
pub const REDESIGN_NAV_ITEM_LABEL_FONT_SIZE_PX: f32 = 12.0;
pub const REDESIGN_NAV_TOP_PADDING_PX: f32 = 14.0;
pub const REDESIGN_NAV_BRAND_BOTTOM_GAP_PX: f32 = 12.0;
pub const REDESIGN_NAV_BRAND_PADDING_X_MARGIN: i8 = 6;
pub const REDESIGN_NAV_BRAND_PADDING_Y_MARGIN: i8 = 4;
pub const REDESIGN_NAV_BRAND_MARK_SIZE_PX: f32 = 36.0;
pub const REDESIGN_NAV_BRAND_MARK_RADIUS_PX: f32 = 6.0;
pub const REDESIGN_NAV_BRAND_MARK_FONT_SIZE_PX: f32 = 24.0;
pub const REDESIGN_NAV_BRAND_TEXT_GAP_PX: f32 = 10.0;
pub const REDESIGN_NAV_BRAND_NAME_FONT_SIZE_PX: f32 = 10.0;
pub const REDESIGN_NAV_BRAND_SUB_FONT_SIZE_PX: f32 = 9.0;
pub const REDESIGN_NAV_SEPARATOR_INSET_PX: f32 = 6.0;
pub const REDESIGN_NAV_SEPARATOR_GAP_PX: f32 = 12.0;
pub const REDESIGN_NAV_FOOTER_TOP_PADDING_PX: f32 = 10.0;
pub const REDESIGN_NAV_FOOTER_DOT_SIZE_PX: f32 = 8.0;
pub const REDESIGN_NAV_FOOTER_DOT_GAP_PX: f32 = 8.0;
pub const REDESIGN_NAV_FOOTER_FONT_SIZE_PX: f32 = 11.0;
pub const REDESIGN_DOT_BG_SPACING_PX: f32 = 20.0;
pub const REDESIGN_PAGE_PADDING_X_PX: f32 = 28.0;
pub const REDESIGN_PAGE_PADDING_Y_PX: f32 = 24.0;
pub const REDESIGN_SETTINGS_PAGE_PADDING_TOP_PX: f32 = 20.0;
pub const REDESIGN_SETTINGS_BOX_PADDING_X_PX: f32 = 22.0;
pub const REDESIGN_SETTINGS_BOX_PADDING_Y_PX: f32 = 18.0;
pub const REDESIGN_BOX_LABEL_FONT_SIZE_PX: f32 = 10.0;
pub const REDESIGN_BOX_LABEL_GAP_PX: f32 = 8.0;
pub const REDESIGN_SETTINGS_TAB_GAP_PX: f32 = 6.0;
pub const REDESIGN_SETTINGS_TAB_PADDING_X_PX: f32 = 18.0;
pub const REDESIGN_SETTINGS_TAB_PADDING_Y_PX: f32 = 8.0;
pub const REDESIGN_SETTINGS_TAB_RADIUS_PX: f32 = 4.0;
pub const REDESIGN_SETTINGS_ROW_PADDING_Y_PX: f32 = 10.0;
pub const REDESIGN_SETTINGS_ROW_GAP_PX: f32 = 16.0;
pub const REDESIGN_SETTINGS_ROW_COLUMN_GAP_PX: f32 = 28.0;
pub const REDESIGN_SETTINGS_ROW_GRID_GAP_Y_PX: f32 = 4.0;
pub const REDESIGN_BUTTON_PADDING_X_PX: f32 = 16.0;
pub const REDESIGN_BUTTON_PADDING_Y_PX: f32 = 8.0;
pub const REDESIGN_BUTTON_SMALL_PADDING_X_PX: f32 = 10.0;
pub const REDESIGN_BUTTON_SMALL_PADDING_Y_PX: f32 = 4.0;
pub const REDESIGN_BUTTON_FONT_SIZE_PX: f32 = 14.0;
pub const REDESIGN_BUTTON_SMALL_FONT_SIZE_PX: f32 = 12.0;
pub const REDESIGN_LABEL_FONT_SIZE_PX: f32 = 14.0;
pub const REDESIGN_HINT_FONT_SIZE_PX: f32 = 13.0;
pub const REDESIGN_TAB_FONT_SIZE_PX: f32 = 16.0;
pub const REDESIGN_AVATAR_FONT_SIZE_PX: f32 = 16.0;
pub const REDESIGN_PILL_FONT_SIZE_PX: f32 = 12.0;
pub const REDESIGN_PATH_ROW_HEIGHT_PX: f32 = 34.0;
pub const REDESIGN_PATH_ROW_LABEL_WIDTH_PX: f32 = 150.0;
pub const REDESIGN_PATH_ROW_HINT_WIDTH_PX: f32 = 90.0;
pub const REDESIGN_PATH_ROW_GAP_PX: f32 = 10.0;
pub const REDESIGN_PATH_INPUT_PADDING_X_PX: f32 = 8.0;
pub const REDESIGN_PATH_INPUT_PADDING_Y_PX: f32 = 3.0;
pub const REDESIGN_PATH_INPUT_FONT_SIZE_PX: f32 = 11.0;
pub const REDESIGN_INPUT_MIN_HEIGHT_PX: f32 = 200.0;
pub const REDESIGN_PATH_BUTTON_WIDTH_PX: f32 = 82.0;
pub const REDESIGN_TOGGLE_WIDTH_PX: f32 = 38.0;
pub const REDESIGN_TOGGLE_HEIGHT_PX: f32 = 20.0;
pub const REDESIGN_TOGGLE_RADIUS_PX: f32 = 12.0;
pub const REDESIGN_TOGGLE_KNOB_RADIUS_PX: f32 = 8.0;
pub const REDESIGN_TOGGLE_KNOB_INSET_PX: f32 = 2.0;
pub const REDESIGN_ACCOUNT_CARD_PADDING_X_PX: f32 = 16.0;
pub const REDESIGN_ACCOUNT_CARD_PADDING_Y_PX: f32 = 10.0;
pub const REDESIGN_ACCOUNT_CARD_GAP_PX: f32 = 12.0;
pub const REDESIGN_ACCOUNT_AVATAR_SIZE_PX: f32 = 36.0;
pub const REDESIGN_ACCOUNT_PILL_RADIUS_PX: f32 = 6.0;
pub const REDESIGN_ACCOUNT_PILL_PADDING_X_PX: f32 = 8.0;
pub const REDESIGN_ACCOUNT_PILL_PADDING_Y_PX: f32 = 3.0;
pub const REDESIGN_FILTER_CHIP_RADIUS_PX: f32 = 14.0;
pub const REDESIGN_FILTER_CHIP_PADDING_X_PX: f32 = 12.0;
pub const REDESIGN_FILTER_CHIP_PADDING_Y_PX: f32 = 4.0;
pub const REDESIGN_FILTER_CHIP_LABEL_GAP_PX: f32 = 4.0;
pub const REDESIGN_FILTER_CHIP_FONT_SIZE_PX: f32 = 13.0;
pub const REDESIGN_HOME_PANEL_PADDING_MARGIN: i8 = 16;
pub const REDESIGN_MODLIST_CARD_PADDING_X_PX: f32 = 12.0;
pub const REDESIGN_MODLIST_CARD_PADDING_Y_PX: f32 = 10.0;
pub const REDESIGN_MODLIST_CARD_TEXT_GAP_PX: f32 = 2.0;
pub const REDESIGN_MODLIST_CARD_ACTION_GAP_PX: f32 = 6.0;
pub const REDESIGN_MODLIST_CARD_ACTION_WIDTH_PX: f32 = 116.0;
pub const REDESIGN_MODLIST_CARD_NAME_FONT_SIZE_PX: f32 = 13.0;
pub const REDESIGN_MODLIST_CARD_META_FONT_SIZE_PX: f32 = 14.0;
pub const REDESIGN_HOME_GRID_GAP_PX: f32 = 20.0;
pub const REDESIGN_HOME_GRID_BOTTOM_MARGIN_PX: f32 = 20.0;
pub const REDESIGN_HOME_LEFT_COLUMN_WEIGHT: f32 = 2.0;
pub const REDESIGN_HOME_RIGHT_COLUMN_WEIGHT: f32 = 1.0;
pub const REDESIGN_HOME_CHIP_ROW_GAP_PX: f32 = 8.0;
pub const REDESIGN_HOME_CHIP_ROW_BOTTOM_MARGIN_PX: f32 = 12.0;
pub const REDESIGN_HOME_CARD_LIST_GAP_PX: f32 = 10.0;
pub const REDESIGN_HOME_ACTION_COLUMN_GAP_PX: f32 = 10.0;
pub const REDESIGN_HOME_GAME_BLOCK_TOP_MARGIN_PX: f32 = 20.0;
pub const REDESIGN_HOME_GAME_LINE_GAP_PX: f32 = 4.0;
pub const REDESIGN_HOME_GAME_LINE_TOP_MARGIN_PX: f32 = 6.0;
pub const REDESIGN_HOME_GAME_STATUS_ICON_WIDTH_PX: f32 = 14.0;
pub const REDESIGN_HOME_GAME_STATUS_STROKE_PX: f32 = 1.5;
pub const REDESIGN_HOME_SETUP_CARD_GAP_PX: f32 = 10.0;
pub const REDESIGN_HOME_CONFIRM_WIDTH_PX: f32 = 460.0;
pub const REDESIGN_SCREEN_TITLE_FONT_SIZE_PX: f32 = 22.0;
pub const REDESIGN_SCREEN_SUBTITLE_FONT_SIZE_PX: f32 = 13.0;
pub const REDESIGN_SCREEN_TITLE_SUBTITLE_GAP_PX: f32 = 4.0;
pub const REDESIGN_SCREEN_TITLE_BOTTOM_GAP_PX: f32 = 20.0;
pub const REDESIGN_KEBAB_MENU_WIDTH_PX: f32 = 180.0;
pub const REDESIGN_KEBAB_MENU_PADDING_PX: f32 = 4.0;
pub const REDESIGN_KEBAB_MENU_OFFSET_PX: f32 = 4.0;
pub const REDESIGN_KEBAB_MENU_SHADOW_OFFSET_PX: f32 = 3.0;
pub const REDESIGN_KEBAB_MENU_ITEM_PADDING_X_PX: f32 = 10.0;
pub const REDESIGN_KEBAB_MENU_ITEM_PADDING_Y_PX: f32 = 6.0;
pub const REDESIGN_KEBAB_MENU_ITEM_FONT_SIZE_PX: f32 = 13.0;
pub const REDESIGN_WORKSPACE_HEADER_GAP_PX: f32 = 20.0;
pub const REDESIGN_WORKSPACE_SECTION_GAP_PX: f32 = 10.0;
pub const REDESIGN_WORKSPACE_NAV_GAP_PX: f32 = 10.0;
pub const REDESIGN_SUBFLOW_SECTION_GAP_PX: f32 = 14.0;
pub const REDESIGN_SUBFLOW_FOOTER_GAP_PX: f32 = 10.0;
pub const REDESIGN_MODAL_LIST_INDENT_PX: f32 = 18.0;
pub const REDESIGN_MODAL_LIST_GAP_PX: f32 = 8.0;
pub const REDESIGN_BIO_SMALL_BUTTON_HEIGHT_PX: f32 = 22.0;
pub const REDESIGN_BIO_PILL_HEIGHT_PX: f32 = 18.0;
pub const REDESIGN_BIO_PILL_RADIUS_PX: f32 = 7.0;
pub const REDESIGN_BIO_PILL_RADIUS_U8: u8 = 7;
pub const REDESIGN_BIO_ROW_GAP_PX: f32 = 6.0;
pub const REDESIGN_BIO_SCROLL_BAR_WIDTH_PX: f32 = 12.0;
pub const REDESIGN_BIO_SCROLL_INNER_MARGIN_PX: f32 = 0.0;
pub const REDESIGN_BIO_SCROLL_OUTER_MARGIN_PX: f32 = 2.0;

#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "egui margin APIs use i8; redesign pixel tokens are bounded UI constants"
)]
pub const fn redesign_i8_px(value: f32) -> i8 {
    value as i8
}

#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "egui radius APIs use u8; redesign pixel tokens are bounded UI constants"
)]
pub const fn redesign_u8_px(value: f32) -> u8 {
    value as u8
}

#[derive(Debug, Clone, Copy)]
struct PaletteValues {
    page_bg: egui::Color32,
    shell_bg: egui::Color32,
    chrome_bg: egui::Color32,
    rail_bg: egui::Color32,
    border_strong: egui::Color32,
    border_soft: egui::Color32,
    border_dashed_light: egui::Color32,
    text_primary: egui::Color32,
    text_muted: egui::Color32,
    text_faint: egui::Color32,
    text_fainter: egui::Color32,
    input_bg: egui::Color32,
    shadow: egui::Color32,
    dot_bg: egui::Color32,
    hover_overlay: egui::Color32,
    success: egui::Color32,
    status_dot: egui::Color32,
    accent: egui::Color32,
    accent_hover: egui::Color32,
    accent_deep: egui::Color32,
    selection_highlight: egui::Color32,
    selection_highlight_hover: egui::Color32,
}

const LIGHT: PaletteValues = PaletteValues {
    page_bg: egui::Color32::from_rgb(0xe8, 0xee, 0xf5),
    shell_bg: egui::Color32::from_rgb(0xf5, 0xf8, 0xfc),
    chrome_bg: egui::Color32::from_rgb(0xcf, 0xdc, 0xe8),
    rail_bg: egui::Color32::from_rgb(0xdd, 0xe6, 0xf0),
    border_strong: egui::Color32::from_rgb(0x1a, 0x26, 0x38),
    border_soft: egui::Color32::from_rgb(0xa5, 0xb4, 0xc7),
    border_dashed_light: egui::Color32::from_rgb(0xcf, 0xd9, 0xe5),
    text_primary: egui::Color32::from_rgb(0x1a, 0x26, 0x38),
    text_muted: egui::Color32::from_rgb(0x5c, 0x6a, 0x7a),
    text_faint: egui::Color32::from_rgb(0x88, 0x96, 0xa8),
    text_fainter: egui::Color32::from_rgb(0xae, 0xbb, 0xcb),
    input_bg: egui::Color32::from_rgb(0xff, 0xff, 0xff),
    shadow: egui::Color32::from_rgb(0x1a, 0x26, 0x38),
    dot_bg: egui::Color32::from_rgba_premultiplied(0x1a, 0x26, 0x38, 20),
    hover_overlay: egui::Color32::from_rgba_premultiplied(0x1a, 0x26, 0x38, 13),
    success: egui::Color32::from_rgb(0x5f, 0xa8, 0x6a),
    status_dot: egui::Color32::from_rgb(0x6f, 0xb8, 0x7a),
    accent: egui::Color32::from_rgb(0x14, 0xb8, 0xa6),
    accent_hover: egui::Color32::from_rgb(0x14, 0xb8, 0xa6),
    accent_deep: egui::Color32::from_rgb(0x0c, 0x6e, 0x64),
    selection_highlight: egui::Color32::from_rgba_premultiplied(0x14, 0xb8, 0xa6, 46),
    selection_highlight_hover: egui::Color32::from_rgba_premultiplied(0x14, 0xb8, 0xa6, 56),
};

const DARK: PaletteValues = PaletteValues {
    page_bg: egui::Color32::from_rgb(0x0b, 0x11, 0x16),
    shell_bg: egui::Color32::from_rgb(0x11, 0x1a, 0x21),
    chrome_bg: egui::Color32::from_rgb(0x15, 0x22, 0x2b),
    rail_bg: egui::Color32::from_rgb(0x15, 0x22, 0x2b),
    border_strong: egui::Color32::from_rgb(0x24, 0x33, 0x3d),
    border_soft: egui::Color32::from_rgb(0x24, 0x33, 0x3d),
    border_dashed_light: egui::Color32::from_rgb(0x1b, 0x27, 0x30),
    text_primary: egui::Color32::from_rgb(0xe6, 0xed, 0xf3),
    text_muted: egui::Color32::from_rgb(0xa7, 0xb3, 0xbd),
    text_faint: egui::Color32::from_rgb(0x6b, 0x77, 0x85),
    text_fainter: egui::Color32::from_rgb(0x4d, 0x55, 0x60),
    input_bg: egui::Color32::from_rgb(0x0b, 0x11, 0x16),
    shadow: egui::Color32::from_rgb(0x24, 0x33, 0x3d),
    dot_bg: egui::Color32::from_rgba_premultiplied(0xe6, 0xed, 0xf3, 13),
    hover_overlay: egui::Color32::from_rgba_premultiplied(0xe6, 0xed, 0xf3, 10),
    success: egui::Color32::from_rgb(0x4a, 0xde, 0x80),
    status_dot: egui::Color32::from_rgb(0x4a, 0xde, 0x80),
    accent: egui::Color32::from_rgb(0x14, 0xb8, 0xa6),
    accent_hover: egui::Color32::from_rgb(0x2d, 0xd4, 0xbf),
    accent_deep: egui::Color32::from_rgb(0x0c, 0x6e, 0x64),
    selection_highlight: egui::Color32::from_rgba_premultiplied(0x14, 0xb8, 0xa6, 46),
    selection_highlight_hover: egui::Color32::from_rgba_premultiplied(0x14, 0xb8, 0xa6, 56),
};

const fn values(palette: ThemePalette) -> PaletteValues {
    match palette {
        ThemePalette::Light => LIGHT,
        ThemePalette::Dark => DARK,
    }
}

#[must_use]
pub const fn redesign_page_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).page_bg
}

#[must_use]
pub const fn redesign_shell_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).shell_bg
}

#[must_use]
pub const fn redesign_chrome_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).chrome_bg
}

#[must_use]
pub const fn redesign_rail_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).rail_bg
}

#[must_use]
pub const fn redesign_border_strong(palette: ThemePalette) -> egui::Color32 {
    values(palette).border_strong
}

#[must_use]
pub const fn redesign_border_soft(palette: ThemePalette) -> egui::Color32 {
    values(palette).border_soft
}

#[must_use]
pub const fn redesign_border_dashed_light(palette: ThemePalette) -> egui::Color32 {
    values(palette).border_dashed_light
}

#[must_use]
pub const fn redesign_text_primary(palette: ThemePalette) -> egui::Color32 {
    values(palette).text_primary
}

#[must_use]
pub const fn redesign_text_muted(palette: ThemePalette) -> egui::Color32 {
    values(palette).text_muted
}

#[must_use]
pub const fn redesign_text_faint(palette: ThemePalette) -> egui::Color32 {
    values(palette).text_faint
}

#[must_use]
pub const fn redesign_text_fainter(palette: ThemePalette) -> egui::Color32 {
    values(palette).text_fainter
}

#[must_use]
pub const fn redesign_text_on_accent(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0x1a, 0x1a, 0x1a)
}

#[must_use]
pub const fn redesign_input_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).input_bg
}

#[must_use]
pub const fn redesign_shadow(palette: ThemePalette) -> egui::Color32 {
    values(palette).shadow
}

#[must_use]
pub const fn redesign_dot_bg(palette: ThemePalette) -> egui::Color32 {
    values(palette).dot_bg
}

#[must_use]
pub const fn redesign_success(palette: ThemePalette) -> egui::Color32 {
    values(palette).success
}

#[must_use]
pub const fn redesign_status_dot(palette: ThemePalette) -> egui::Color32 {
    values(palette).status_dot
}

#[must_use]
pub const fn redesign_accent(palette: ThemePalette) -> egui::Color32 {
    values(palette).accent
}

#[must_use]
pub const fn redesign_accent_hover(palette: ThemePalette) -> egui::Color32 {
    values(palette).accent_hover
}

#[must_use]
pub const fn redesign_accent_deep(palette: ThemePalette) -> egui::Color32 {
    values(palette).accent_deep
}

#[must_use]
pub const fn redesign_pill_danger(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xe6, 0x9a, 0x96)
}

#[must_use]
pub const fn redesign_pill_warn(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xe8, 0xc4, 0x41)
}

#[must_use]
pub const fn redesign_pill_info(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xa8, 0xd2, 0xcc)
}

#[must_use]
pub const fn redesign_pill_neutral(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xc4, 0xca, 0xd1)
}

#[must_use]
pub const fn redesign_selection_highlight(palette: ThemePalette) -> egui::Color32 {
    values(palette).selection_highlight
}

#[must_use]
pub const fn redesign_selection_highlight_hover(palette: ThemePalette) -> egui::Color32 {
    values(palette).selection_highlight_hover
}

#[must_use]
pub const fn redesign_hover_overlay(palette: ThemePalette) -> egui::Color32 {
    values(palette).hover_overlay
}

#[must_use]
pub const fn redesign_text_disabled(palette: ThemePalette) -> egui::Color32 {
    values(palette).text_fainter
}

#[must_use]
pub const fn redesign_accent_path(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x8a, 0x60, 0x32),
        ThemePalette::Dark => egui::Color32::from_rgb(0xe8, 0xc4, 0x8f),
    }
}

#[must_use]
pub const fn redesign_accent_numbers(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x1f, 0x5f, 0xbb),
        ThemePalette::Dark => egui::Color32::from_rgb(0x8a, 0xb4, 0xff),
    }
}

#[must_use]
pub const fn redesign_accent_comment(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x36, 0x7d, 0x45),
        ThemePalette::Dark => egui::Color32::from_rgb(0x7c, 0xc4, 0x7c),
    }
}

#[must_use]
pub const fn redesign_warning(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x9f, 0x67, 0x1b),
        ThemePalette::Dark => egui::Color32::from_rgb(0xd6, 0xa8, 0x60),
    }
}

#[must_use]
pub const fn redesign_warning_soft(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xa8, 0x73, 0x20),
        ThemePalette::Dark => egui::Color32::from_rgb(0xdc, 0xb4, 0x64),
    }
}

#[must_use]
pub const fn redesign_error(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xb7, 0x3c, 0x3c),
        ThemePalette::Dark => egui::Color32::from_rgb(0xdc, 0x64, 0x64),
    }
}

#[must_use]
pub const fn redesign_prompt_text(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0x28, 0x14, 0x00)
}

#[must_use]
pub const fn redesign_prompt_fill(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xf5, 0xc3, 0x5f)
}

#[must_use]
pub const fn redesign_prompt_stroke(_palette: ThemePalette) -> egui::Color32 {
    egui::Color32::from_rgb(0xd2, 0xa0, 0x46)
}

#[must_use]
pub const fn redesign_compat_included(palette: ThemePalette) -> egui::Color32 {
    redesign_text_faint(palette)
}

#[must_use]
pub const fn redesign_compat_included_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xdc, 0xe8, 0xdd),
        ThemePalette::Dark => egui::Color32::from_rgb(0x38, 0x48, 0x38),
    }
}

#[must_use]
pub const fn redesign_compat_conflict(palette: ThemePalette) -> egui::Color32 {
    redesign_error(palette)
}

#[must_use]
pub const fn redesign_compat_conflict_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xf2, 0xdc, 0xdc),
        ThemePalette::Dark => egui::Color32::from_rgb(0x58, 0x2c, 0x2c),
    }
}

#[must_use]
pub const fn redesign_compat_warning(palette: ThemePalette) -> egui::Color32 {
    redesign_warning(palette)
}

#[must_use]
pub const fn redesign_compat_warning_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xf2, 0xe7, 0xc8),
        ThemePalette::Dark => egui::Color32::from_rgb(0x4e, 0x3e, 0x22),
    }
}

#[must_use]
pub const fn redesign_compat_info(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x2e, 0x78, 0xa6),
        ThemePalette::Dark => egui::Color32::from_rgb(0x78, 0xba, 0xe6),
    }
}

#[must_use]
pub const fn redesign_compat_info_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xd9, 0xeb, 0xf6),
        ThemePalette::Dark => egui::Color32::from_rgb(0x2a, 0x42, 0x56),
    }
}

#[must_use]
pub const fn redesign_compat_mismatch(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x9b, 0x4b, 0x8d),
        ThemePalette::Dark => egui::Color32::from_rgb(0xcb, 0x6e, 0xbc),
    }
}

#[must_use]
pub const fn redesign_compat_mismatch_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xef, 0xdc, 0xef),
        ThemePalette::Dark => egui::Color32::from_rgb(0x4e, 0x2c, 0x54),
    }
}

#[must_use]
pub const fn redesign_compat_conditional(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x59, 0x6f, 0x7c),
        ThemePalette::Dark => egui::Color32::from_rgb(0xa4, 0xbe, 0xd0),
    }
}

#[must_use]
pub const fn redesign_compat_conditional_fill(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0xde, 0xe7, 0xed),
        ThemePalette::Dark => egui::Color32::from_rgb(0x34, 0x42, 0x4e),
    }
}

#[must_use]
pub const fn redesign_status_running(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x4d, 0x86, 0x30),
        ThemePalette::Dark => egui::Color32::from_rgb(0xa8, 0xcc, 0x62),
    }
}

#[must_use]
pub const fn redesign_status_preparing(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x67, 0x5b, 0xae),
        ThemePalette::Dark => egui::Color32::from_rgb(0xb4, 0xaa, 0xdc),
    }
}

#[must_use]
pub const fn redesign_status_idle(palette: ThemePalette) -> egui::Color32 {
    redesign_text_muted(palette)
}

#[must_use]
pub const fn redesign_terminal_default(palette: ThemePalette) -> egui::Color32 {
    redesign_text_primary(palette)
}

#[must_use]
pub const fn redesign_terminal_error(palette: ThemePalette) -> egui::Color32 {
    redesign_error(palette)
}

#[must_use]
pub const fn redesign_terminal_debug(palette: ThemePalette) -> egui::Color32 {
    match palette {
        ThemePalette::Light => egui::Color32::from_rgb(0x2c, 0x62, 0xab),
        ThemePalette::Dark => egui::Color32::from_rgb(0x46, 0x6e, 0xb4),
    }
}

#[must_use]
pub const fn redesign_terminal_sent(palette: ThemePalette) -> egui::Color32 {
    redesign_accent_numbers(palette)
}

#[must_use]
pub const fn redesign_terminal_info(palette: ThemePalette) -> egui::Color32 {
    redesign_status_running(palette)
}

#[must_use]
pub const fn redesign_terminal_amber(palette: ThemePalette) -> egui::Color32 {
    redesign_warning(palette)
}

#[must_use]
pub const fn redesign_terminal_sand(palette: ThemePalette) -> egui::Color32 {
    redesign_accent_path(palette)
}

#[must_use]
pub const fn redesign_terminal_dim(palette: ThemePalette) -> egui::Color32 {
    redesign_text_faint(palette)
}

pub fn apply_redesign_bio_visuals(ui: &mut egui::Ui, palette: ThemePalette) {
    let visuals = &mut ui.style_mut().visuals;
    visuals.override_text_color = Some(redesign_text_primary(palette));
    visuals.panel_fill = redesign_shell_bg(palette);
    visuals.window_fill = redesign_shell_bg(palette);
    visuals.extreme_bg_color = redesign_input_bg(palette);
    visuals.faint_bg_color = redesign_chrome_bg(palette);
    visuals.selection.bg_fill = redesign_selection_highlight(palette);
    visuals.selection.stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_accent(palette));

    visuals.widgets.noninteractive.bg_fill = redesign_shell_bg(palette);
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette));
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_primary(palette));

    visuals.widgets.inactive.bg_fill = redesign_shell_bg(palette);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette));
    visuals.widgets.inactive.fg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_primary(palette));

    visuals.widgets.hovered.bg_fill = redesign_selection_highlight_hover(palette);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_accent(palette));
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_primary(palette));

    visuals.widgets.active.bg_fill = redesign_accent(palette);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_accent_deep(palette));
    visuals.widgets.active.fg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_on_accent(palette));

    visuals.widgets.open.bg_fill = redesign_chrome_bg(palette);
    visuals.widgets.open.bg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
    visuals.widgets.open.fg_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_primary(palette));
}

#[must_use]
pub fn redesign_font_light() -> egui::FontFamily {
    egui::FontFamily::Name("poppins_light".into())
}

#[must_use]
pub fn redesign_font_medium() -> egui::FontFamily {
    egui::FontFamily::Name("poppins_medium".into())
}

#[must_use]
pub fn redesign_font_bold() -> egui::FontFamily {
    egui::FontFamily::Name("poppins_bold".into())
}

#[must_use]
pub fn redesign_font_mono() -> egui::FontFamily {
    egui::FontFamily::Name("firacode_nerd".into())
}
