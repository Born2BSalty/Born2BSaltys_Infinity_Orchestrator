// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `workspace_progress_bar` — the 4-segment workspace step progress bar.
//
// Mirrors `wireframe-preview/screens.jsx::WorkspaceProgressBar`
// (line 3298-3356):
//   container: flex row, sketchyBorder, boxShadow "3px 3px 0 var(--shadow)",
//              overflow hidden, marginBottom 8.
//   each segment: flex 1, padding "5px 12px", borderRight 1.5px solid
//                 var(--border-strong) (except the last), minHeight 30,
//                 background = current ? var(--accent)
//                                      : upcoming ? var(--chrome-bg)
//                                      : var(--shell-bg),
//                 opacity = upcoming ? 0.55 : 1,
//                 gap 10.
//     kicker  ("STEP N"): Poppins 10px / 500, letterSpacing 1.4, uppercase,
//                 color = upcoming ? text-faint
//                       : completed ? text-muted
//                       : current ? #1a2638
//                       : text.
//     label:   fontSize current?14:13, fontWeight current?700:400,
//                 color = upcoming ? text-faint : current ? #1a2638 : text.
//     ✓ (completed only): marginLeft auto, var(--success), 15px.
//
// **Symbol-glyph rule.** The completed `✓` is U+2713 — a base-FiraCode glyph
// the bundled full FiraCode Nerd build covers (cmap-verified, HANDOFF
// "Symbol-glyph coverage"); it would tofu in the Latin-only Poppins subset.
// So it is rendered in `firacode_nerd`, prose in Poppins. (No
// Misc-Symbols/emoji glyph here, so no vector paint needed.)
//
// SPEC: §2.2 (workspace progress bar).

// rationale: f32→u8 channel/alpha roundings of small positive constants —
// correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_strong, redesign_chrome_bg, redesign_shadow,
    redesign_shell_bg, redesign_success, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};
use crate::ui::workspace::state_workspace::{WorkspaceStep, WorkspaceViewState};

/// Fixed `#1a2638` ink for text on the teal accent fill (theme-invariant —
/// same constant the wireframe + `redesign_btn` use for primary text).
const ON_ACCENT_INK: egui::Color32 = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

const SEG_PAD_X: f32 = 12.0;
const SEG_PAD_Y: f32 = 5.0;
const SEG_MIN_H: f32 = 30.0;
const SEG_GAP: f32 = 10.0;
const KICKER_SIZE: f32 = 10.0;
const CHECK_SIZE: f32 = 15.0;
const BOTTOM_MARGIN: f32 = 8.0;

/// Per-segment classification.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SegKind {
    Current,
    Completed,
    Upcoming,
}

/// Render the progress bar for the given workspace view state.
pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WorkspaceViewState) {
    let steps = WorkspaceStep::ALL;
    let n = steps.len();

    // Outer rect: full width, height = the tallest segment.
    let label_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_bold".into()),
        ))
    });
    let seg_h = (label_h + SEG_PAD_Y * 2.0).max(SEG_MIN_H);
    let full_w = ui.available_width();
    let (outer, _) = ui.allocate_exact_size(
        egui::vec2(full_w, seg_h + BOTTOM_MARGIN),
        egui::Sense::hover(),
    );
    let bar_rect = egui::Rect::from_min_size(outer.min, egui::vec2(full_w, seg_h));

    if !ui.is_rect_visible(bar_rect) {
        return;
    }

    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);

    // 3×3 hard drop shadow behind the whole bar (wireframe boxShadow
    // "3px 3px 0 var(--shadow)" — uses the same offset family as the
    // primary-button 2×2; 3px per the wireframe literal).
    let shadow_rect = bar_rect.translate(egui::vec2(
        REDESIGN_SHADOW_OFFSET_BTN_PX + 1.0,
        REDESIGN_SHADOW_OFFSET_BTN_PX + 1.0,
    ));
    painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));

    let seg_w = full_w / n as f32;
    for (i, step) in steps.iter().enumerate() {
        let kind = if *step == state.current_step {
            SegKind::Current
        } else if state.is_completed(*step) {
            SegKind::Completed
        } else {
            SegKind::Upcoming
        };

        let seg_min = egui::pos2(bar_rect.min.x + seg_w * i as f32, bar_rect.min.y);
        let seg_rect = egui::Rect::from_min_size(seg_min, egui::vec2(seg_w, seg_h));

        // Background fill (opacity baked for the upcoming case).
        let (bg, alpha) = match kind {
            SegKind::Current => (redesign_accent(palette), 1.0),
            SegKind::Upcoming => (redesign_chrome_bg(palette), 0.55),
            SegKind::Completed => (redesign_shell_bg(palette), 1.0),
        };
        painter.rect_filled(seg_rect, egui::CornerRadius::ZERO, with_alpha(bg, alpha));

        // Right divider (1.5px solid border-strong), except the last seg.
        if i < n - 1 {
            let x = seg_rect.max.x;
            painter.line_segment(
                [egui::pos2(x, seg_rect.min.y), egui::pos2(x, seg_rect.max.y)],
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            );
        }

        // Text colors per the wireframe.
        let kicker_color = match kind {
            SegKind::Upcoming => redesign_text_faint(palette),
            SegKind::Completed => redesign_text_muted(palette),
            SegKind::Current => ON_ACCENT_INK,
        };
        let label_color = match kind {
            SegKind::Upcoming => redesign_text_faint(palette),
            SegKind::Current => ON_ACCENT_INK,
            SegKind::Completed => redesign_text_primary(palette),
        };

        // Kicker ("STEP N", uppercase). The wireframe letterSpacing 1.4 is
        // approximated by uppercasing + the Poppins-medium 10px size (egui
        // has no per-glyph letter-spacing; the visual weight matches).
        let kicker_font =
            egui::FontId::new(KICKER_SIZE, egui::FontFamily::Name("poppins_medium".into()));
        let kicker_text = step.step_kicker().to_uppercase();
        let kicker_pos = egui::pos2(seg_rect.min.x + SEG_PAD_X, seg_rect.center().y);
        let kicker_galley = painter.layout_no_wrap(
            kicker_text.clone(),
            kicker_font.clone(),
            with_alpha(kicker_color, alpha),
        );
        painter.galley(
            egui::pos2(kicker_pos.x, kicker_pos.y - kicker_galley.size().y / 2.0),
            kicker_galley.clone(),
            with_alpha(kicker_color, alpha),
        );

        // Label.
        let (label_size, label_family) = if kind == SegKind::Current {
            (14.0, "poppins_bold")
        } else {
            (13.0, "poppins_medium")
        };
        let label_font = egui::FontId::new(label_size, egui::FontFamily::Name(label_family.into()));
        let label_x = kicker_pos.x + kicker_galley.size().x + SEG_GAP;
        let label_galley = painter.layout_no_wrap(
            step.label().to_string(),
            label_font,
            with_alpha(label_color, alpha),
        );
        painter.galley(
            egui::pos2(label_x, seg_rect.center().y - label_galley.size().y / 2.0),
            label_galley,
            with_alpha(label_color, alpha),
        );

        // ✓ for completed segments — flush-right (marginLeft auto), success
        // color, rendered in `firacode_nerd` (cmap-present base-FiraCode
        // glyph).
        if kind == SegKind::Completed {
            let check_font =
                egui::FontId::new(CHECK_SIZE, egui::FontFamily::Name("firacode_nerd".into()));
            let check_galley = painter.layout_no_wrap(
                "\u{2713}".to_string(),
                check_font,
                redesign_success(palette),
            );
            painter.galley(
                egui::pos2(
                    seg_rect.max.x - SEG_PAD_X - check_galley.size().x,
                    seg_rect.center().y - check_galley.size().y / 2.0,
                ),
                check_galley,
                redesign_success(palette),
            );
        }
    }

    // Outer 1.5px border + rounded clip on the whole bar.
    painter.rect_stroke(
        bar_rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

/// Apply an alpha multiplier (0.0..=1.0) on top of an existing `Color32`.
fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    if (alpha - 1.0).abs() < f32::EPSILON {
        return c;
    }
    let a = (f32::from(c.a()) * alpha).clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}
