// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `filter_chip` — Home filter-chip button (Installed / In progress / All).
//
// Mirrors `wireframe-preview/screens.jsx::Chip` (line 277-296):
//   sketchyBorder (1.5px solid border-strong, 3px radius base) overridden to
//   `borderRadius: 14` (pill ends), `boxShadow: none`.
//   background: active ? var(--accent) : var(--shell-bg)
//   color:      active ? #1a2638       : var(--text)
//   padding:    4px 12px
//   fontSize:   13, fontWeight: active ? 500 : 400
//   trailing `(count)` span:
//     color:      active ? #1a2638 : var(--text-faint)
//     fontWeight: 400 (always)
//
// SPEC: §3.1 ("Filter chips" — lighter visual treatment than the primary
// Btn: no drop shadow, 14px border radius; active chip filled in
// var(--accent) with dark text, inactive chips var(--shell-bg) + normal
// text).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

/// 14px pill radius (wireframe `borderRadius: 14`).
const CHIP_RADIUS_PX: f32 = 14.0;
/// 4px × 12px padding (wireframe `padding: "4px 12px"`).
const CHIP_PAD_X_PX: f32 = 12.0;
const CHIP_PAD_Y_PX: f32 = 4.0;
/// Fixed dark text on the teal accent fill — theme-invariant, matches the
/// wireframe's `#1a2638` and the redesign Btn's primary text.
const ON_ACCENT_TEXT: egui::Color32 = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

/// Render a filter chip and return the click `Response`.
///
/// `label` is the chip text (e.g. "Installed"); `count` is shown after it as
/// a faint `(N)`; `active` fills the chip with the accent.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    count: usize,
    active: bool,
) -> egui::Response {
    let label_color = if active {
        ON_ACCENT_TEXT
    } else {
        redesign_text_primary(palette)
    };
    let count_color = if active {
        ON_ACCENT_TEXT
    } else {
        redesign_text_faint(palette)
    };

    // Active chip uses Poppins medium (≈ weight 500); inactive uses the
    // lighter Poppins. The trailing count is always the lighter weight.
    let label_family = if active {
        egui::FontFamily::Name("poppins_medium".into())
    } else {
        egui::FontFamily::Name("poppins_light".into())
    };
    let count_family = egui::FontFamily::Name("poppins_light".into());

    let label_font = egui::FontId::new(13.0, label_family);
    let count_font = egui::FontId::new(13.0, count_family);
    let count_text = format!(" ({count})");

    let label_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), label_font.clone(), label_color);
    let count_galley =
        ui.painter()
            .layout_no_wrap(count_text.clone(), count_font.clone(), count_color);

    let content_w = label_galley.size().x + count_galley.size().x;
    let content_h = label_galley.size().y.max(count_galley.size().y);
    let desired = egui::vec2(
        content_w + CHIP_PAD_X_PX * 2.0,
        content_h + CHIP_PAD_Y_PX * 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(CHIP_RADIUS_PX as u8);
        let fill = if active {
            redesign_accent(palette)
        } else {
            redesign_shell_bg(palette)
        };
        // No drop shadow (wireframe `boxShadow: "none"`).
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );

        // Lay label + count out left-to-right, vertically centered.
        let total_w = label_galley.size().x + count_galley.size().x;
        let mut x = rect.center().x - total_w / 2.0;
        let cy = rect.center().y;
        painter.galley(
            egui::pos2(x, cy - label_galley.size().y / 2.0),
            label_galley.clone(),
            label_color,
        );
        x += label_galley.size().x;
        painter.galley(
            egui::pos2(x, cy - count_galley.size().y / 2.0),
            count_galley.clone(),
            count_color,
        );
    }

    response
}
