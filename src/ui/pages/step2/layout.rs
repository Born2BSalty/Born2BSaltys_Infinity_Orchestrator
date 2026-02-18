// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step2::details::recompute_selection_counts;
use crate::ui::step2::tabs::normalize_active_tab;
use crate::ui::state::WizardState;

use super::{actions_tabs, compat_popup, details_pane, header, list_pane, Step2Action};

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<Step2Action> {
    let mut action = None;
    ui.add(Step2LayoutWidget {
        state,
        action: &mut action,
    });
    action
}

struct Step2LayoutWidget<'a> {
    state: &'a mut WizardState,
    action: &'a mut Option<Step2Action>,
}

impl egui::Widget for Step2LayoutWidget<'_> {
    #[allow(deprecated)]
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let margin = 8.0;
        let gap = 8.0;
        let h_title = 30.0;
        let h_subtitle = 20.0;
        let h_search = 40.0;
        let h_controls = 34.0;
        let h_tabs = 26.0;
        let h_footer = 20.0;
        let nav_clearance = 2.0;
        let split_gap = 4.0;
        let splitter_w = 6.0;
        let min_left_w = 420.0;
        let min_right_w = 240.0;

        let available = ui.available_size_before_wrap();
        let (root_rect, response) = ui.allocate_exact_size(
            egui::vec2(available.x.max(900.0), available.y.max(620.0)),
            egui::Sense::hover(),
        );

        let mut y = root_rect.top() + margin;
        let x = root_rect.left() + margin;
        let w = root_rect.width() - (margin * 2.0);
        normalize_active_tab(self.state);

        let title_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_title));
        y += h_title;
        let subtitle_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_subtitle));
        y += h_subtitle + gap;
        let search_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_search));
        y += h_search + gap;
        let controls_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_controls));
        y += h_controls + gap;
        let tabs_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_tabs));
        y += h_tabs + gap;
        let content_h =
            (root_rect.bottom() - margin - h_footer - gap - nav_clearance - y).max(240.0);
        let content_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, content_h));
        y += content_h + gap;
        let footer_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_footer));

        let split_total_w = (w - split_gap - splitter_w).max(min_left_w + min_right_w);
        let left_w = (split_total_w * self.state.step2.left_pane_ratio)
            .clamp(min_left_w, (split_total_w - min_right_w).max(min_left_w));
        let right_w = (split_total_w - left_w).max(min_right_w);
        let left_rect =
            egui::Rect::from_min_size(content_rect.min, egui::vec2(left_w, content_rect.height()));
        let splitter_rect = egui::Rect::from_min_size(
            egui::pos2(left_rect.right() + split_gap * 0.5, content_rect.top()),
            egui::vec2(splitter_w, content_rect.height()),
        );
        let right_rect = egui::Rect::from_min_size(
            egui::pos2(splitter_rect.right() + split_gap * 0.5, content_rect.top()),
            egui::vec2(right_w, content_rect.height()),
        );

        let splitter_id = ui.id().with("step2_splitter");
        let splitter_resp = ui.interact(splitter_rect, splitter_id, egui::Sense::click_and_drag());
        if splitter_resp.hovered() || splitter_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if splitter_resp.dragged()
            && let Some(pointer_pos) = ui.ctx().pointer_latest_pos()
        {
            let unclamped = pointer_pos.x - content_rect.left() - (split_gap * 0.5);
            let new_left =
                unclamped.clamp(min_left_w, (split_total_w - min_right_w).max(min_left_w));
            self.state.step2.left_pane_ratio = (new_left / split_total_w).clamp(0.1, 0.9);
        }

        header::render_header(ui, self.state, title_rect, subtitle_rect, search_rect);
        actions_tabs::render_controls(ui, self.state, self.action, controls_rect);
        actions_tabs::render_tabs(ui, self.state, self.action, tabs_rect);
        list_pane::render(ui, self.state, self.action, left_rect);
        details_pane::render(ui, self.state, self.action, right_rect);
        compat_popup::render(ui, self.state);

        let vis = &ui.visuals().widgets.noninteractive;
        let splitter_x = splitter_rect.center().x;
        ui.painter().line_segment(
            [
                egui::pos2(splitter_x, splitter_rect.top() + 1.0),
                egui::pos2(splitter_x, splitter_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(1.0, vis.bg_stroke.color),
        );

        let x = left_rect.right() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(x, left_rect.top() + 1.0),
                egui::pos2(x, left_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(1.0, vis.bg_stroke.color),
        );
        let y = left_rect.bottom() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(left_rect.left() + 1.0, y),
                egui::pos2(left_rect.right() - 1.0, y),
            ],
            egui::Stroke::new(1.0, vis.bg_stroke.color),
        );

        ui.allocate_ui_at_rect(footer_rect, |ui| {
            recompute_selection_counts(self.state);
            ui.label(&self.state.step2.scan_status);
        });

        response
    }
}
