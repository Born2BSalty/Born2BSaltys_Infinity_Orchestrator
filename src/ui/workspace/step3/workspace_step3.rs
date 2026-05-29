// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::redesign_text_faint;
use crate::ui::shared::tab_open_seam::paint_active_tab_seam_cover;
use crate::ui::step3::state_step3;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::step3::step3_list_body;
use crate::ui::workspace::step3::step3_tab_row;

const STEP3_BODY_HINT: &str =
    "Right-click a component for more actions, including uncheck and prompt tools.";
const BODY_HINT_H: f32 = 22.0;
const BODY_HINT_GAP: f32 = 10.0;
const TAB_ROW_H: f32 = 30.0;
const TAB_TO_LIST_OVERLAP: f32 = 1.5;
const LIST_MIN_H: f32 = 160.0;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    // Pre-compute layout rects from the ui before any content is painted.
    let root = ui.available_rect_before_wrap();
    let x = root.left();
    let w = root.width();
    let mut y = root.top();
    let body_hint_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, BODY_HINT_H));
    y += BODY_HINT_H + BODY_HINT_GAP;
    let tab_row_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
    y += TAB_ROW_H - TAB_TO_LIST_OVERLAP;
    let list_h = (root.bottom() - y).max(LIST_MIN_H);
    let list_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, list_h));

    // Compute toolbar summary and active markers; these are owned values.
    let (toolbar_summary, active_markers) = {
        let state = &mut orchestrator.wizard_state;
        state_step3::normalize_active_tab(state);
        let toolbar_summary = toolbar_support_step3::build_toolbar_summary(state);
        state.step3.bgee_has_conflict = toolbar_summary.show_bgee
            && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bgee_markers);
        state.step3.bg2ee_has_conflict = toolbar_summary.show_bg2ee
            && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bg2ee_markers);
        let active_markers = if state.step3.active_game_tab == "BGEE" {
            toolbar_summary.bgee_markers.clone()
        } else {
            toolbar_summary.bg2ee_markers.clone()
        };
        (toolbar_summary, active_markers)
    };

    ui.scope_builder(egui::UiBuilder::new().max_rect(body_hint_rect), |ui| {
        ui.label(
            egui::RichText::new(STEP3_BODY_HINT)
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
    });

    let active_tab_rect = step3_tab_row::render(
        ui,
        &mut orchestrator.wizard_state,
        palette,
        &toolbar_summary,
        tab_row_rect,
    );

    clipped_pane(ui, list_rect, |ui| {
        step3_list_body::render(ui, orchestrator, &active_markers);
    });

    if let Some(tab_rect) = active_tab_rect {
        paint_active_tab_seam_cover(ui.painter(), palette, tab_rect, list_rect.top());
    }

    let state = &mut orchestrator.wizard_state;
    crate::ui::step2::content_step2::render_compat_popup(ui, state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
}

fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    ui.allocate_rect(rect, egui::Sense::hover());
}
