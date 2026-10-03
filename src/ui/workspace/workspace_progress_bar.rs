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
const STACK_GAP: f32 = 2.0;
const KICKER_SIZE: f32 = 10.0;
const CHECK_SIZE: f32 = 15.0;
const BOTTOM_MARGIN: f32 = 8.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SegKind {
    Current,
    Completed,
    Upcoming,
}

#[must_use]
const fn stacked_needed(inline_widths: &[f32], seg_w: f32) -> bool {
    let mut i = 0;
    while i < inline_widths.len() {
        if inline_widths[i] > seg_w {
            return true;
        }
        i += 1;
    }
    false
}

#[must_use]
const fn label_max_width(seg_w: f32, completed: bool, check_w: f32) -> f32 {
    let reserved = if completed { SEG_GAP + check_w } else { 0.0 };
    seg_w - 2.0 * SEG_PAD_X - reserved
}

fn label_font_for(kind: SegKind) -> egui::FontId {
    let (size, family) = if kind == SegKind::Current {
        (14.0, "poppins_bold")
    } else {
        (13.0, "poppins_medium")
    };
    egui::FontId::new(size, egui::FontFamily::Name(family.into()))
}

fn segment_kind(step: WorkspaceStep, state: &WorkspaceViewState) -> SegKind {
    if step == state.current_step {
        SegKind::Current
    } else if state.is_completed(step) {
        SegKind::Completed
    } else {
        SegKind::Upcoming
    }
}

struct SegMeasure {
    kicker: f32,
    label: f32,
    check: f32,
}

impl SegMeasure {
    fn inline_width(&self, kind: SegKind) -> f32 {
        let completed_tail = if kind == SegKind::Completed {
            SEG_GAP + self.check
        } else {
            0.0
        };
        SEG_PAD_X.mul_add(2.0, self.kicker + SEG_GAP + self.label + completed_tail)
    }
}

fn measure_segment(
    painter: &egui::Painter,
    step: WorkspaceStep,
    kind: SegKind,
    kicker_font: &egui::FontId,
    check_font: &egui::FontId,
) -> SegMeasure {
    let kicker_text = step.step_kicker().to_uppercase();
    let kicker = painter
        .layout_no_wrap(kicker_text, kicker_font.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let label = painter
        .layout_no_wrap(
            step.label().to_string(),
            label_font_for(kind),
            egui::Color32::WHITE,
        )
        .size()
        .x;
    let check = if kind == SegKind::Completed {
        painter
            .layout_no_wrap(
                "\u{2713}".to_string(),
                check_font.clone(),
                egui::Color32::WHITE,
            )
            .size()
            .x
    } else {
        0.0
    };
    SegMeasure {
        kicker,
        label,
        check,
    }
}

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WorkspaceViewState) {
    let steps = WorkspaceStep::ALL;
    let n16 = u16::try_from(steps.len()).unwrap_or(1);
    let full_w = ui.available_width();
    let seg_w = full_w / f32::from(n16);

    let kicker_font =
        egui::FontId::new(KICKER_SIZE, egui::FontFamily::Name("poppins_medium".into()));
    let check_font = egui::FontId::new(CHECK_SIZE, egui::FontFamily::Name("firacode_nerd".into()));
    let label_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_bold".into()),
        ))
    });
    let kicker_h = ui.fonts(|f| f.row_height(&kicker_font));

    let kinds: Vec<SegKind> = steps
        .iter()
        .map(|step| segment_kind(*step, state))
        .collect();
    let measure_painter = ui.painter().clone();
    let measured: Vec<SegMeasure> = steps
        .iter()
        .zip(kinds.iter())
        .map(|(step, kind)| {
            measure_segment(&measure_painter, *step, *kind, &kicker_font, &check_font)
        })
        .collect();
    let inline_widths: Vec<f32> = measured
        .iter()
        .zip(kinds.iter())
        .map(|(m, kind)| m.inline_width(*kind))
        .collect();

    let stacked = stacked_needed(&inline_widths, seg_w);
    let seg_h = if stacked {
        SEG_PAD_Y.mul_add(2.0, kicker_h + STACK_GAP + label_h)
    } else {
        SEG_PAD_Y.mul_add(2.0, label_h).max(SEG_MIN_H)
    };

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

    for (i, step) in steps.iter().enumerate() {
        let kind = kinds[i];
        let i16 = u16::try_from(i).unwrap_or(0);
        paint_segment(
            painter,
            &SegmentPaint {
                palette,
                step: *step,
                kind,
                index: i16,
                count: n16,
                bar_rect,
                seg_h,
                seg_w,
                stacked,
                label_max_w: label_max_width(seg_w, kind == SegKind::Completed, measured[i].check),
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
    stacked: bool,
    label_max_w: f32,
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

    let text_params = SegTextParams {
        step: paint.step,
        kind: paint.kind,
        seg_rect,
        kicker_color,
        label_color,
        alpha,
        stacked: paint.stacked,
        label_max_w: paint.label_max_w,
    };
    let label_row_cy = paint_segment_text(painter, &text_params);
    paint_completed_check(
        painter,
        palette,
        paint.kind,
        seg_rect,
        paint.stacked,
        label_row_cy,
    );
}

struct SegTextParams {
    step: WorkspaceStep,
    kind: SegKind,
    seg_rect: egui::Rect,
    kicker_color: egui::Color32,
    label_color: egui::Color32,
    alpha: f32,
    stacked: bool,
    label_max_w: f32,
}

fn paint_segment_text(painter: &egui::Painter, params: &SegTextParams) -> f32 {
    let painter = painter.with_clip_rect(params.seg_rect.intersect(painter.clip_rect()));
    let kicker_font =
        egui::FontId::new(KICKER_SIZE, egui::FontFamily::Name("poppins_medium".into()));
    let kicker_text = params.step.step_kicker().to_uppercase();
    let label_font = label_font_for(params.kind);

    if params.stacked {
        paint_stacked_segment_text(&painter, params, &kicker_text, kicker_font, label_font)
    } else {
        paint_inline_segment_text(&painter, params, &kicker_text, kicker_font, label_font)
    }
}

fn paint_stacked_segment_text(
    painter: &egui::Painter,
    params: &SegTextParams,
    kicker_text: &str,
    kicker_font: egui::FontId,
    label_font: egui::FontId,
) -> f32 {
    let seg_rect = params.seg_rect;
    let kicker_rect = painter.text(
        egui::pos2(seg_rect.min.x + SEG_PAD_X, seg_rect.min.y + SEG_PAD_Y),
        egui::Align2::LEFT_TOP,
        kicker_text,
        kicker_font,
        with_alpha(params.kicker_color, params.alpha),
    );

    let label_pos = egui::pos2(seg_rect.min.x + SEG_PAD_X, kicker_rect.bottom() + STACK_GAP);
    let mut job = egui::text::LayoutJob::single_section(
        params.step.label().to_string(),
        egui::text::TextFormat::simple(label_font, with_alpha(params.label_color, params.alpha)),
    );
    job.wrap = egui::text::TextWrapping {
        max_width: params.label_max_w,
        max_rows: 1,
        break_anywhere: true,
        ..Default::default()
    };
    let galley = painter.layout_job(job);
    let label_h = galley.size().y;
    painter.galley(
        label_pos,
        galley,
        with_alpha(params.label_color, params.alpha),
    );
    label_pos.y + label_h / 2.0
}

fn paint_inline_segment_text(
    painter: &egui::Painter,
    params: &SegTextParams,
    kicker_text: &str,
    kicker_font: egui::FontId,
    label_font: egui::FontId,
) -> f32 {
    let seg_rect = params.seg_rect;
    let row_cy = seg_rect.center().y;
    let kicker_rect = painter.text(
        egui::pos2(seg_rect.min.x + SEG_PAD_X, row_cy),
        egui::Align2::LEFT_CENTER,
        kicker_text,
        kicker_font,
        with_alpha(params.kicker_color, params.alpha),
    );

    painter.text(
        egui::pos2(kicker_rect.right() + SEG_GAP, row_cy),
        egui::Align2::LEFT_CENTER,
        params.step.label(),
        label_font,
        with_alpha(params.label_color, params.alpha),
    );
    row_cy
}

fn paint_completed_check(
    painter: &egui::Painter,
    palette: ThemePalette,
    kind: SegKind,
    seg_rect: egui::Rect,
    stacked: bool,
    label_row_cy: f32,
) {
    if kind == SegKind::Completed {
        let painter = painter.with_clip_rect(seg_rect.intersect(painter.clip_rect()));
        let check_font =
            egui::FontId::new(CHECK_SIZE, egui::FontFamily::Name("firacode_nerd".into()));
        let cy = if stacked {
            label_row_cy
        } else {
            seg_rect.center().y
        };
        painter.text(
            egui::pos2(seg_rect.max.x - SEG_PAD_X, cy),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacked_needed_false_when_all_fit() {
        assert!(!stacked_needed(&[100.0, 190.0], 192.0));
    }

    #[test]
    fn stacked_needed_true_when_one_overflows() {
        assert!(stacked_needed(&[100.0, 200.0], 192.0));
    }

    #[test]
    fn stacked_needed_false_when_empty() {
        assert!(!stacked_needed(&[], 192.0));
    }

    #[test]
    fn label_max_width_without_check() {
        assert!((label_max_width(192.0, false, 0.0) - 168.0).abs() < f32::EPSILON);
    }

    #[test]
    fn label_max_width_with_check() {
        assert!((label_max_width(192.0, true, 10.0) - 148.0).abs() < f32::EPSILON);
    }
}
