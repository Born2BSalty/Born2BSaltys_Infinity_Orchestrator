// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `filter_chip` — Home filter-chip button (Installed / In progress / All).
//
// Mirrors `wireframe-preview/screens.jsx::Chip` (line 277-296):
//   sketchyBorder (1.5px solid border-strong) overridden to
//   `borderRadius: 14` (pill ends), `boxShadow: none`.
//   background: active ? var(--accent) : var(--shell-bg)
//   color:      active ? #1a2638       : var(--text)
//   fontSize:   13, fontWeight: active ? 500 : 400
//   trailing `(count)` span: color active ? #1a2638 : var(--text-faint),
//   fontWeight 400 (always).
//
// Padding: the wireframe is `4px 12px`. The vertical padding is
// **intentionally taller** here (7px) per a user visual call — the literal
// 4px read too cramped against the rest of the Home chrome. Horizontal
// padding stays at the wireframe's 12px. Documented as a deliberate
// deviation so it isn't re-flagged against the wireframe.
//
// Press feedback mirrors the wireframe's shared `.sk-btn:active
// { transform: translate(1px,1px) }`.
//
// SPEC: §3.1 ("Filter chips" — lighter visual treatment than the primary
// Btn: no drop shadow, 14px border radius; active chip filled in
// var(--accent) with dark text, inactive chips var(--shell-bg) + normal
// text).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_accent, redesign_border_strong, redesign_shell_bg, redesign_text_faint,
    redesign_text_primary, ThemePalette, REDESIGN_BORDER_WIDTH_PX,
};

/// 14px pill radius (wireframe `borderRadius: 14`).
const CHIP_RADIUS_PX: f32 = 14.0;
/// 12px horizontal padding (wireframe `padding: "4px 12px"`).
const CHIP_PAD_X_PX: f32 = 12.0;
/// Vertical padding — intentionally taller than the wireframe's 4px (see
/// the module header note); a deliberate user-directed visual deviation.
const CHIP_PAD_Y_PX: f32 = 7.0;
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
    let label_font = egui::FontId::new(
        13.0,
        if active {
            egui::FontFamily::Name("poppins_medium".into())
        } else {
            egui::FontFamily::Name("poppins_light".into())
        },
    );
    let count_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));

    // One combined galley (label run + count run, both centre-aligned on the
    // line) so the two weights share a single layout box. Laying them out as
    // two independent galleys made the active chip read top-biased — the
    // fix for the "selected pill text not vertically centered" issue.
    let mut job = egui::text::LayoutJob::default();
    job.append(
        label,
        0.0,
        egui::TextFormat {
            font_id: label_font,
            color: label_color,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    job.append(
        &format!(" ({count})"),
        0.0,
        egui::TextFormat {
            font_id: count_font,
            color: count_color,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    let galley = ui.fonts(|f| f.layout_job(job));

    let desired = egui::vec2(
        galley.size().x + CHIP_PAD_X_PX * 2.0,
        galley.size().y + CHIP_PAD_Y_PX * 2.0,
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    // Wireframe `.sk-btn:active { transform: translate(1px,1px) }`.
    let rect = if response.is_pointer_button_down_on() {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

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

        // Single galley centred on both axes.
        let pos = rect.center() - galley.size() * 0.5;
        painter.galley(pos, galley, label_color);
    }

    response
}
