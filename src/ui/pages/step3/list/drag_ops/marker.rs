// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;

pub(in crate::ui::pages::step3::list) fn draw_insert_marker(
    ui: &egui::Ui,
    items: &[Step3ItemState],
    drag_from: &Option<usize>,
    drag_over: Option<usize>,
    visible_rows: &[(usize, egui::Rect)],
) {
    if drag_from.is_none() {
        return;
    }
    if let Some(insert_at) = drag_over {
        let row_rects: Vec<egui::Rect> = visible_rows.iter().map(|(_, r)| *r).collect();
        if !row_rects.is_empty() {
            let clamped = insert_at.min(items.len());
            let (x0, x1, y) = if clamped == 0 {
                let r = row_rects[0];
                (r.left(), r.right(), r.top() - 1.0)
            } else if clamped >= row_rects.len() {
                let r = row_rects[row_rects.len() - 1];
                (r.left(), r.right(), r.bottom() + 1.0)
            } else {
                let r = row_rects[clamped];
                (r.left(), r.right(), r.top() - 1.0)
            };
            ui.painter().line_segment(
                [egui::pos2(x0, y), egui::pos2(x1, y)],
                egui::Stroke::new(1.5, ui.visuals().selection.stroke.color),
            );
        }
    }
}
