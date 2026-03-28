// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::layout_tokens_global::*;
use crate::ui::state::WizardState;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::service_list_ops_step2::recompute_selection_counts;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step2Action> {
    let mut action = None;
    ui.add(Step2LayoutWidget {
        state,
        action: &mut action,
        dev_mode,
        exe_fingerprint,
    });
    action
}

struct Step2LayoutWidget<'a> {
    state: &'a mut WizardState,
    action: &'a mut Option<Step2Action>,
    dev_mode: bool,
    exe_fingerprint: &'a str,
}

impl egui::Widget for Step2LayoutWidget<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let margin = STEP2_MARGIN;
        let gap = STEP2_SECTION_GAP;
        let h_title = STEP2_TITLE_H;
        let h_subtitle = STEP2_SUBTITLE_H;
        let h_search = STEP2_SEARCH_H;
        let h_controls = STEP2_CONTROLS_H;
        let h_tabs = STEP2_TABS_H;
        let h_footer = STEP2_FOOTER_H;
        let nav_clearance = STEP2_NAV_CLEARANCE_H;
        let split_gap = STEP2_SPLIT_GAP;
        let splitter_w = STEP2_SPLITTER_W;
        let min_left_w = STEP2_LEFT_MIN_W;
        let min_right_w = STEP2_RIGHT_MIN_W;

        let available = ui.available_size_before_wrap();
        let (root_rect, response) = ui.allocate_exact_size(
            egui::vec2(available.x.max(900.0), available.y.max(620.0)),
            egui::Sense::hover(),
        );

        let mut y = root_rect.top() + margin;
        let x = root_rect.left() + margin;
        let w = root_rect.width() - (margin * 2.0);
        crate::ui::step2::state_step2::normalize_active_tab(self.state);

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
        let content_h = (root_rect.bottom() - margin - h_footer - gap - nav_clearance - y).max(240.0);
        let content_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, content_h));
        y += content_h + gap;
        let footer_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h_footer));

        let split_total_w = (w - split_gap - splitter_w).max(min_left_w + min_right_w);
        let left_w = (split_total_w * self.state.step2.left_pane_ratio)
            .clamp(min_left_w, (split_total_w - min_right_w).max(min_left_w));
        let right_w = (split_total_w - left_w).max(min_right_w);
        let left_rect = egui::Rect::from_min_size(content_rect.min, egui::vec2(left_w, content_rect.height()));
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
        if splitter_resp.dragged() && let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
            let unclamped = pointer_pos.x - content_rect.left() - (split_gap * 0.5);
            let new_left = unclamped.clamp(min_left_w, (split_total_w - min_right_w).max(min_left_w));
            self.state.step2.left_pane_ratio = (new_left / split_total_w).clamp(0.1, 0.9);
        }

        crate::ui::step2::content_step2::render_header(
            ui,
            self.state,
            title_rect,
            subtitle_rect,
            search_rect,
            self.dev_mode,
            self.exe_fingerprint,
        );
        crate::ui::step2::content_step2::render_controls(ui, self.state, self.action, controls_rect);
        crate::ui::step2::content_step2::render_tabs(ui, self.state, self.action, tabs_rect);
        crate::ui::step2::list_pane_step2::render_list_pane(ui, self.state, self.action, left_rect);
        crate::ui::step2::content_step2::render_details_pane(ui, self.state, self.action, right_rect);
        crate::ui::step2::content_step2::render_compat_popup(ui, self.state);
        crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, self.state);

        let vis = &ui.visuals().widgets.noninteractive;
        let splitter_x = splitter_rect.center().x;
        ui.painter().line_segment(
            [
                egui::pos2(splitter_x, splitter_rect.top() + 1.0),
                egui::pos2(splitter_x, splitter_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                vis.bg_stroke.color,
            ),
        );

        let x = left_rect.right() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(x, left_rect.top() + 1.0),
                egui::pos2(x, left_rect.bottom() - 1.0),
            ],
            egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                vis.bg_stroke.color,
            ),
        );
        let y = left_rect.bottom() - 1.0;
        ui.painter().line_segment(
            [
                egui::pos2(left_rect.left() + 1.0, y),
                egui::pos2(left_rect.right() - 1.0, y),
            ],
            egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                vis.bg_stroke.color,
            ),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
            recompute_selection_counts(self.state);
            ui.label(&self.state.step2.scan_status);
        });

        response
    }
}
