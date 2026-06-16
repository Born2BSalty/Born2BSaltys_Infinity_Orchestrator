// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_error, redesign_hover_overlay, redesign_input_bg,
    redesign_rail_bg, redesign_selection_highlight, redesign_shell_bg, redesign_text_primary,
    redesign_warning,
};

#[must_use]
pub fn redesign_overlay_shadow(palette: ThemePalette) -> egui::epaint::Shadow {
    let color = match palette {
        ThemePalette::Dark => egui::Color32::from_rgba_unmultiplied(4, 12, 16, 150),
        ThemePalette::Light => egui::Color32::from_rgba_unmultiplied(26, 38, 56, 80),
    };
    egui::epaint::Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color,
    }
}

#[must_use]
pub fn build_for(palette: ThemePalette) -> egui::Visuals {
    let mut v = egui::Visuals::dark();

    let border_strong_stroke =
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
    let text_primary_stroke = egui::Stroke::new(1.0, redesign_text_primary(palette));
    let corner = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

    v.override_text_color = Some(redesign_text_primary(palette));
    v.panel_fill = redesign_shell_bg(palette);
    v.window_fill = redesign_shell_bg(palette);
    v.window_stroke = border_strong_stroke;
    v.window_corner_radius = corner;
    v.window_shadow = redesign_overlay_shadow(palette);
    v.window_highlight_topmost = false;
    v.popup_shadow = redesign_overlay_shadow(palette);
    v.faint_bg_color = redesign_rail_bg(palette);
    v.extreme_bg_color = redesign_input_bg(palette);
    v.code_bg_color = redesign_input_bg(palette);
    v.selection.bg_fill = redesign_selection_highlight(palette);
    v.selection.stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_accent(palette));
    v.hyperlink_color = redesign_accent(palette);
    v.error_fg_color = redesign_error(palette);
    v.warn_fg_color = redesign_warning(palette);

    v.widgets.noninteractive.bg_fill = redesign_shell_bg(palette);
    v.widgets.noninteractive.bg_stroke = border_strong_stroke;
    v.widgets.noninteractive.fg_stroke = text_primary_stroke;
    v.widgets.noninteractive.corner_radius = corner;
    v.widgets.noninteractive.expansion = 0.0;

    v.widgets.inactive.bg_fill = redesign_shell_bg(palette);
    v.widgets.inactive.weak_bg_fill = redesign_shell_bg(palette);
    v.widgets.inactive.bg_stroke = border_strong_stroke;
    v.widgets.inactive.fg_stroke = text_primary_stroke;
    v.widgets.inactive.corner_radius = corner;
    v.widgets.inactive.expansion = 0.0;

    v.widgets.hovered.bg_fill = redesign_shell_bg(palette);
    v.widgets.hovered.weak_bg_fill = redesign_hover_overlay(palette);
    v.widgets.hovered.bg_stroke = border_strong_stroke;
    v.widgets.hovered.fg_stroke = text_primary_stroke;
    v.widgets.hovered.corner_radius = corner;
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = redesign_accent(palette);
    v.widgets.active.bg_stroke = border_strong_stroke;
    v.widgets.active.fg_stroke = text_primary_stroke;
    v.widgets.active.corner_radius = corner;
    v.widgets.active.expansion = 0.0;

    v.widgets.open.bg_fill = redesign_hover_overlay(palette);
    v.widgets.open.bg_stroke = border_strong_stroke;
    v.widgets.open.fg_stroke = text_primary_stroke;
    v.widgets.open.corner_radius = corner;
    v.widgets.open.expansion = 0.0;

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shared::redesign_tokens::{
        ThemePalette, redesign_accent, redesign_border_strong, redesign_error,
        redesign_selection_highlight, redesign_shell_bg,
    };

    #[test]
    fn dark_visuals_window_fill_matches_dark_shell_bg() {
        let v = build_for(ThemePalette::Dark);
        assert_eq!(v.window_fill, redesign_shell_bg(ThemePalette::Dark));
    }

    #[test]
    fn light_visuals_window_fill_matches_light_shell_bg() {
        let v = build_for(ThemePalette::Light);
        assert_eq!(v.window_fill, redesign_shell_bg(ThemePalette::Light));
    }

    #[test]
    fn dark_visuals_widgets_inactive_bg_stroke_matches_dark_border_strong() {
        let v = build_for(ThemePalette::Dark);
        assert_eq!(
            v.widgets.inactive.bg_stroke.color,
            redesign_border_strong(ThemePalette::Dark)
        );
    }

    #[test]
    fn light_visuals_widgets_inactive_bg_stroke_matches_light_border_strong() {
        let v = build_for(ThemePalette::Light);
        assert_eq!(
            v.widgets.inactive.bg_stroke.color,
            redesign_border_strong(ThemePalette::Light)
        );
    }

    #[test]
    fn dark_visuals_widgets_active_bg_fill_matches_dark_accent() {
        let v = build_for(ThemePalette::Dark);
        assert_eq!(
            v.widgets.active.bg_fill,
            redesign_accent(ThemePalette::Dark)
        );
    }

    #[test]
    fn light_visuals_widgets_active_bg_fill_matches_light_accent() {
        let v = build_for(ThemePalette::Light);
        assert_eq!(
            v.widgets.active.bg_fill,
            redesign_accent(ThemePalette::Light)
        );
    }

    #[test]
    fn dark_visuals_selection_bg_fill_matches_dark_selection_highlight() {
        let v = build_for(ThemePalette::Dark);
        assert_eq!(
            v.selection.bg_fill,
            redesign_selection_highlight(ThemePalette::Dark)
        );
    }

    #[test]
    fn light_visuals_selection_bg_fill_matches_light_selection_highlight() {
        let v = build_for(ThemePalette::Light);
        assert_eq!(
            v.selection.bg_fill,
            redesign_selection_highlight(ThemePalette::Light)
        );
    }

    #[test]
    fn dark_visuals_error_fg_color_matches_dark_error() {
        let v = build_for(ThemePalette::Dark);
        assert_eq!(v.error_fg_color, redesign_error(ThemePalette::Dark));
    }

    #[test]
    fn light_visuals_error_fg_color_matches_light_error() {
        let v = build_for(ThemePalette::Light);
        assert_eq!(v.error_fg_color, redesign_error(ThemePalette::Light));
    }

    #[test]
    fn dark_visuals_window_highlight_topmost_disabled() {
        let v = build_for(ThemePalette::Dark);
        assert!(!v.window_highlight_topmost);
    }

    #[test]
    fn light_visuals_window_highlight_topmost_disabled() {
        let v = build_for(ThemePalette::Light);
        assert!(!v.window_highlight_topmost);
    }

    #[test]
    fn dark_visuals_overlay_shadows_enabled() {
        let v = build_for(ThemePalette::Dark);
        assert_ne!(v.window_shadow, egui::epaint::Shadow::NONE);
        assert_ne!(v.popup_shadow, egui::epaint::Shadow::NONE);
    }

    #[test]
    fn light_visuals_overlay_shadows_enabled() {
        let v = build_for(ThemePalette::Light);
        assert_ne!(v.window_shadow, egui::epaint::Shadow::NONE);
        assert_ne!(v.popup_shadow, egui::epaint::Shadow::NONE);
    }
}
