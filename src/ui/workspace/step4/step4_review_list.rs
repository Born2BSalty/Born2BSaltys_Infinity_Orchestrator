// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint,
};
use crate::ui::workspace::widgets::weidu_line;

const BOX_PADDING: f32 = 12.0;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    items: &[Step3ItemState],
    active_tab: &str,
) -> f32 {
    let leaves: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();
    let rows = review_rows(items);

    let avail = ui.available_size();
    let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    if ui.is_rect_visible(box_rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: REDESIGN_BORDER_RADIUS_U8,
            se: REDESIGN_BORDER_RADIUS_U8,
        };
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }

    let inner = box_rect.shrink(BOX_PADDING);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));

    if leaves.is_empty() {
        child.label(
            egui::RichText::new(format!("No components selected on {active_tab}."))
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
    } else {
        let max_digits = leaves.len().to_string().len();
        let lineno_w = weidu_line::lineno_column_width(max_digits);

        egui::ScrollArea::vertical()
            .id_salt(("workspace_step4_review_scroll", active_tab.to_string()))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                for (item, line_number) in &rows {
                    weidu_line::render_weidu_line(ui, palette, item, *line_number, lineno_w);
                }
            });
    }

    ui.allocate_rect(box_rect, egui::Sense::hover());
    box_rect.top()
}

fn review_rows(items: &[Step3ItemState]) -> Vec<(&Step3ItemState, Option<usize>)> {
    let mut next_number = 0;
    items
        .iter()
        .filter(|item| !item.is_parent || item.is_separator())
        .map(|item| {
            if item.is_separator() {
                (item, None)
            } else {
                next_number += 1;
                (item, Some(next_number))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(tp: &str, id: &str, label: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: label.to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }
    fn parent(tp: &str) -> Step3ItemState {
        let mut p = leaf(tp, "__PARENT__", "");
        p.is_parent = true;
        p
    }

    #[test]
    fn lineno_width_keys_off_leaf_count_not_total() {
        let mut items = vec![parent("A.TP2")];
        for n in 0..12 {
            items.push(leaf("A.TP2", &n.to_string(), "x"));
        }
        let leaves: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();
        assert_eq!(leaves.len(), 12);
        let w2 = weidu_line::lineno_column_width(leaves.len().to_string().len());
        let w1 = weidu_line::lineno_column_width(1);
        assert!(w2 > w1, "2-digit column wider than 1-digit");
    }

    #[test]
    fn review_rows_number_components_only() {
        let items = vec![
            crate::app::step3_separators::separator_row("Fixes", "SEPARATOR::1".to_string()),
            parent("A.TP2"),
            leaf("A.TP2", "0", "first"),
            crate::app::step3_separators::separator_row("NPCs", "SEPARATOR::2".to_string()),
            leaf("A.TP2", "1", "second"),
        ];

        let rows = review_rows(&items);

        assert_eq!(
            rows.iter().map(|(_, number)| *number).collect::<Vec<_>>(),
            vec![None, Some(1), None, Some(2)]
        );
        assert!(rows[0].0.is_separator());
        assert_eq!(rows[1].0.component_id, "0");
        assert!(rows[2].0.is_separator());
        assert_eq!(rows[3].0.component_id, "1");
    }
}
