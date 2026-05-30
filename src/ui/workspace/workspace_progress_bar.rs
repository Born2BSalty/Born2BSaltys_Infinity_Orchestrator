// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_chrome_bg, redesign_shell_bg, redesign_success,
    redesign_text_faint, redesign_text_muted, redesign_text_primary, redesign_with_alpha,
};
use crate::ui::workspace::state_workspace::{WorkspaceStep, WorkspaceViewState};

const ON_ACCENT_INK: egui::Color32 = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

const SEG_PAD_X: f32 = 12.0;
const SEG_PAD_Y: f32 = 5.0;
const SEG_MIN_H: f32 = 30.0;
const SEG_GAP: f32 = 10.0;
const KICKER_SIZE: f32 = 10.0;
const CHECK_SIZE: f32 = 15.0;
const BOTTOM_MARGIN: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SegKind {
    Current,
    Completed,
    Upcoming,
}

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WorkspaceViewState) {
    let steps = WorkspaceStep::ALL;
    let n = steps.len();

    let label_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_bold".into()),
        ))
    });
    let seg_h = SEG_PAD_Y.mul_add(2.0, label_h).max(SEG_MIN_H);
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
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

    let n = u16::try_from(n).unwrap_or(1);
    let seg_w = full_w / f32::from(n);
    for (i, step) in steps.iter().enumerate() {
        let kind = if *step == state.current_step {
            SegKind::Current
        } else if state.is_completed(*step) {
            SegKind::Completed
        } else {
            SegKind::Upcoming
        };

        let i = u16::try_from(i).unwrap_or(0);
        paint_segment(
            painter,
            &SegmentPaint {
                palette,
                step: *step,
                kind,
                index: i,
                count: n,
                bar_rect,
                seg_h,
                seg_w,
            },
        );
    }

    painter.rect_stroke(
        bar_rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

struct SegmentPaint {
    palette: ThemePalette,
    step: WorkspaceStep,
    kind: SegKind,
    index: u16,
    count: u16,
    bar_rect: egui::Rect,
    seg_h: f32,
    seg_w: f32,
}

fn paint_segment(painter: &egui::Painter, paint: &SegmentPaint) {
    let palette = paint.palette;
    let seg_min = egui::pos2(
        paint
            .seg_w
            .mul_add(f32::from(paint.index), paint.bar_rect.min.x),
        paint.bar_rect.min.y,
    );
    let seg_rect = egui::Rect::from_min_size(seg_min, egui::vec2(paint.seg_w, paint.seg_h));

    let (bg, alpha) = match paint.kind {
        SegKind::Current => (redesign_accent(palette), 1.0),
        SegKind::Upcoming => (redesign_chrome_bg(palette), 0.55),
        SegKind::Completed => (redesign_shell_bg(palette), 1.0),
    };
    painter.rect_filled(seg_rect, egui::CornerRadius::ZERO, with_alpha(bg, alpha));

    if paint.index < paint.count - 1 {
        let x = seg_rect.max.x;
        painter.line_segment(
            [egui::pos2(x, seg_rect.min.y), egui::pos2(x, seg_rect.max.y)],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        );
    }

    let kicker_color = match paint.kind {
        SegKind::Upcoming => redesign_text_faint(palette),
        SegKind::Completed => redesign_text_muted(palette),
        SegKind::Current => ON_ACCENT_INK,
    };
    let label_color = match paint.kind {
        SegKind::Upcoming => redesign_text_faint(palette),
        SegKind::Current => ON_ACCENT_INK,
        SegKind::Completed => redesign_text_primary(palette),
    };
    paint_segment_text(
        painter,
        paint.step,
        paint.kind,
        seg_rect,
        kicker_color,
        label_color,
        alpha,
    );
    paint_completed_check(painter, palette, paint.kind, seg_rect);
}

fn paint_segment_text(
    painter: &egui::Painter,
    step: WorkspaceStep,
    kind: SegKind,
    seg_rect: egui::Rect,
    kicker_color: egui::Color32,
    label_color: egui::Color32,
    alpha: f32,
) {
    let row_cy = seg_rect.center().y;
    let kicker_font =
        egui::FontId::new(KICKER_SIZE, egui::FontFamily::Name("poppins_medium".into()));
    let kicker_text = step.step_kicker().to_uppercase();
    let kicker_rect = painter.text(
        egui::pos2(seg_rect.min.x + SEG_PAD_X, row_cy),
        egui::Align2::LEFT_CENTER,
        &kicker_text,
        kicker_font,
        with_alpha(kicker_color, alpha),
    );

    let (label_size, label_family) = if kind == SegKind::Current {
        (14.0, "poppins_bold")
    } else {
        (13.0, "poppins_medium")
    };
    let label_font = egui::FontId::new(label_size, egui::FontFamily::Name(label_family.into()));
    painter.text(
        egui::pos2(kicker_rect.right() + SEG_GAP, row_cy),
        egui::Align2::LEFT_CENTER,
        step.label(),
        label_font,
        with_alpha(label_color, alpha),
    );
}

fn paint_completed_check(
    painter: &egui::Painter,
    palette: ThemePalette,
    kind: SegKind,
    seg_rect: egui::Rect,
) {
    if kind == SegKind::Completed {
        let check_font =
            egui::FontId::new(CHECK_SIZE, egui::FontFamily::Name("firacode_nerd".into()));
        painter.text(
            egui::pos2(seg_rect.max.x - SEG_PAD_X, seg_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            "\u{2713}",
            check_font,
            redesign_success(palette),
        );
    }
}

fn with_alpha(c: egui::Color32, alpha: f32) -> egui::Color32 {
    if (alpha - 1.0).abs() < f32::EPSILON {
        return c;
    }
    redesign_with_alpha(c, 55, 100)
}
