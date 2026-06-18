// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::layout_tokens_global::{
    STEP2_CONTROLS_H, STEP2_FOOTER_H, STEP2_LEFT_MIN_W, STEP2_MARGIN, STEP2_NAV_CLEARANCE_H,
    STEP2_RIGHT_MIN_W, STEP2_SEARCH_H, STEP2_SECTION_GAP, STEP2_SPLIT_GAP, STEP2_SPLITTER_W,
    STEP2_SUBTITLE_H, STEP2_TABS_H, STEP2_TITLE_H,
};
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
        let (layout, response) = build_layout(ui, self.state);
        handle_splitter(ui, self.state, &layout);
        render_step2_content(
            ui,
            self.state,
            self.action,
            self.dev_mode,
            self.exe_fingerprint,
            &layout,
        );
        draw_splitter_lines(ui, &layout);
        render_footer(ui, self.state, layout.footer_rect);
        response
    }
}

struct Step2Layout {
    title_rect: egui::Rect,
    subtitle_rect: egui::Rect,
    search_rect: egui::Rect,
    controls_rect: egui::Rect,
    tabs_rect: egui::Rect,
    left_rect: egui::Rect,
    right_rect: egui::Rect,
    splitter_rect: egui::Rect,
    footer_rect: egui::Rect,
    content_rect: egui::Rect,
    split_total_w: f32,
}

fn build_layout(ui: &mut egui::Ui, state: &mut WizardState) -> (Step2Layout, egui::Response) {
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
    let w = margin.mul_add(-2.0, root_rect.width());
    recompute_selection_counts(state);

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
    let left_w = (split_total_w * state.step2.left_pane_ratio)
        .clamp(min_left_w, (split_total_w - min_right_w).max(min_left_w));
    let right_w = (split_total_w - left_w).max(min_right_w);
    let left_rect =
        egui::Rect::from_min_size(content_rect.min, egui::vec2(left_w, content_rect.height()));
    let splitter_rect = egui::Rect::from_min_size(
        egui::pos2(
            split_gap.mul_add(0.5, left_rect.right()),
            content_rect.top(),
        ),
        egui::vec2(splitter_w, content_rect.height()),
    );
    let right_rect = egui::Rect::from_min_size(
        egui::pos2(
            split_gap.mul_add(0.5, splitter_rect.right()),
            content_rect.top(),
        ),
        egui::vec2(right_w, content_rect.height()),
    );

    (
        Step2Layout {
            title_rect,
            subtitle_rect,
            search_rect,
            controls_rect,
            tabs_rect,
            left_rect,
            right_rect,
            splitter_rect,
            footer_rect,
            content_rect,
            split_total_w,
        },
        response,
    )
}

fn handle_splitter(ui: &egui::Ui, state: &mut WizardState, layout: &Step2Layout) {
    let splitter_id = ui.id().with("step2_splitter");
    let splitter_resp = ui.interact(
        layout.splitter_rect,
        splitter_id,
        egui::Sense::click_and_drag(),
    );
    if splitter_resp.hovered() || splitter_resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if splitter_resp.dragged()
        && let Some(pointer_pos) = ui.ctx().pointer_latest_pos()
    {
        let min_left_w = STEP2_LEFT_MIN_W;
        let min_right_w = STEP2_RIGHT_MIN_W;
        let unclamped = STEP2_SPLIT_GAP.mul_add(-0.5, pointer_pos.x - layout.content_rect.left());
        let new_left = unclamped.clamp(
            min_left_w,
            (layout.split_total_w - min_right_w).max(min_left_w),
        );
        state.step2.left_pane_ratio = (new_left / layout.split_total_w).clamp(0.1, 0.9);
    }
}

fn render_step2_content(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    dev_mode: bool,
    exe_fingerprint: &str,
    layout: &Step2Layout,
) {
    crate::ui::step2::content_step2::render_header(
        ui,
        state,
        layout.title_rect,
        layout.subtitle_rect,
        layout.search_rect,
        dev_mode,
        exe_fingerprint,
    );
    crate::ui::step2::content_step2::render_controls(ui, state, action, layout.controls_rect);
    crate::ui::step2::content_step2::render_tabs(ui, state, action, layout.tabs_rect);
    let mut details_open = true;
    let palette = crate::ui::shared::redesign_tokens::ThemePalette::default();
    crate::ui::step2::list_pane_step2::render_list_pane(
        ui,
        state,
        action,
        layout.left_rect,
        &mut details_open,
        palette,
    );
    crate::ui::step2::details_pane_step2::render_pane(
        ui,
        state,
        action,
        layout.right_rect,
        palette,
        &mut details_open,
    );
    crate::ui::step2::compat_window_step2::render(
        ui,
        state,
        crate::ui::shared::redesign_tokens::ThemePalette::Dark,
    );
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
}

fn draw_splitter_lines(ui: &egui::Ui, layout: &Step2Layout) {
    let vis = &ui.visuals().widgets.noninteractive;
    let splitter_x = layout.splitter_rect.center().x;
    ui.painter().line_segment(
        [
            egui::pos2(splitter_x, layout.splitter_rect.top() + 1.0),
            egui::pos2(splitter_x, layout.splitter_rect.bottom() - 1.0),
        ],
        egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            vis.bg_stroke.color,
        ),
    );

    let x = layout.left_rect.right() - 1.0;
    ui.painter().line_segment(
        [
            egui::pos2(x, layout.left_rect.top() + 1.0),
            egui::pos2(x, layout.left_rect.bottom() - 1.0),
        ],
        egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            vis.bg_stroke.color,
        ),
    );
    let y = layout.left_rect.bottom() - 1.0;
    ui.painter().line_segment(
        [
            egui::pos2(layout.left_rect.left() + 1.0, y),
            egui::pos2(layout.left_rect.right() - 1.0, y),
        ],
        egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            vis.bg_stroke.color,
        ),
    );
}

fn render_footer(ui: &mut egui::Ui, state: &mut WizardState, footer_rect: egui::Rect) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
        recompute_selection_counts(state);
        ui.label(&state.step2.scan_status);
    });
}
