// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `Btn` primitive — sketchy-bordered button with primary / secondary fill,
// optional small variant, optional disabled.
//
// Mirrors `wireframe-preview/screens.jsx::Btn` (line 22-43):
//   sketchyBorder + 1.5px solid border
//   background: primary ? var(--accent) : var(--shell-bg)
//   color:      primary ? #1a2638       : var(--text)
//   padding:    small ? 4px 10px : 8px 16px
//   fontSize:   small ? 12 : 14
//   opacity:    disabled ? 0.5 : 1
//   boxShadow:  primary ? "2px 2px 0 var(--shadow)" : none
//
// SPEC: §1.2 (sketchy aesthetic, 1.5px borders, 2×2 drop shadow on primary).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette,
    redesign_accent, redesign_border_strong, redesign_shadow, redesign_shell_bg,
    redesign_text_primary,
};

/// Optional rendering options for `redesign_btn`. All default to `false`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BtnOpts {
    /// Filled accent background + 2×2 drop shadow.
    pub primary: bool,
    /// Smaller padding (4×10) and 12px font.
    pub small: bool,
    /// 50% opacity, click suppressed.
    pub disabled: bool,
}

/// Paint a redesign button at the current `ui` cursor and return the
/// `egui::Response` so callers can react to `.clicked()`.
///
/// `palette` carries the active theme; `opts` toggles primary / small /
/// disabled. The button uses the redesign Poppins font; the small variant
/// drops to 12px.
pub fn redesign_btn(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    opts: BtnOpts,
) -> egui::Response {
    let (pad_x, pad_y, font_size) = if opts.small { (10.0, 4.0, 12.0) } else { (16.0, 8.0, 14.0) };
    let fill = if opts.primary {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let text_color = if opts.primary {
        // SPEC § wireframe::screens.jsx — fixed `#1a2638` for primary text
        // (high-contrast against the teal accent, theme-invariant).
        egui::Color32::from_rgb(0x1a, 0x26, 0x38)
    } else {
        redesign_text_primary(palette)
    };

    // Measure the label so we can size the rect.
    let font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));
    let text_galley = ui.painter().layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired_size = egui::vec2(
        text_galley.size().x + pad_x * 2.0,
        text_galley.size().y + pad_y * 2.0,
    );

    let sense = if opts.disabled { egui::Sense::hover() } else { egui::Sense::click() };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let alpha = if opts.disabled { 0.5 } else { 1.0 };

        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);

        // Primary drops a 2×2 shadow behind the rect.
        if opts.primary {
            let shadow_rect = rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            ));
            painter.rect_filled(shadow_rect, radius, with_alpha(redesign_shadow(palette), alpha));
        }

        // Body fill.
        painter.rect_filled(rect, radius, with_alpha(fill, alpha));

        // 1.5px solid border.
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                with_alpha(redesign_border_strong(palette), alpha),
            ),
            egui::StrokeKind::Inside,
        );

        // Centered label.
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            with_alpha(text_color, alpha),
        );
    }

    response
}

/// Apply an alpha multiplier (0.0..=1.0) on top of an existing `Color32`.
fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    let a = (c.a() as f32 * alpha).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
