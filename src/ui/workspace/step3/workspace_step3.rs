// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::game_authority::{self, GameSlot};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::tab_open_seam::paint_active_tab_seam_cover;
use crate::ui::step3::state_step3;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::step3::step3_list_body;
use crate::ui::workspace::step3::step3_tab_row;

const TAB_ROW_H: f32 = 30.0;
const TAB_TO_LIST_OVERLAP: f32 = 1.5;
const LIST_MIN_H: f32 = 160.0;
const WIZARD_STEP3_INDEX: usize = 2;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    showing_wizard_step3(orchestrator, |orchestrator| {
        render_step3(ui, orchestrator);
    });
}

fn showing_wizard_step3(
    orchestrator: &mut OrchestratorApp,
    body: impl FnOnce(&mut OrchestratorApp),
) {
    let step_before = orchestrator.wizard_state.current_step;
    orchestrator.wizard_state.current_step = WIZARD_STEP3_INDEX;
    body(orchestrator);
    orchestrator.wizard_state.current_step = step_before;
}

fn render_step3(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    let root = ui.available_rect_before_wrap();
    let x = root.left();
    let w = root.width();
    let mut y = root.top();
    let tab_row_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
    y += TAB_ROW_H - TAB_TO_LIST_OVERLAP;
    let list_h = (root.bottom() - y).max(LIST_MIN_H);
    let list_rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, list_h));

    let (toolbar_summary, active_markers) = {
        let state = &mut orchestrator.wizard_state;
        state_step3::normalize_active_tab(state);
        let toolbar_summary = toolbar_support_step3::build_toolbar_summary(state);
        state.step3.bgee_has_conflict = toolbar_summary.show_bgee
            && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bgee_markers);
        state.step3.bg2ee_has_conflict = toolbar_summary.show_bg2ee
            && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bg2ee_markers);
        let active_markers =
            if game_authority::slot_for_tab(&state.step3.active_game_tab) == GameSlot::First {
                toolbar_summary.bgee_markers.clone()
            } else {
                toolbar_summary.bg2ee_markers.clone()
            };
        (toolbar_summary, active_markers)
    };

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

    let outcome = {
        let state = &mut orchestrator.wizard_state;
        let outcome = crate::ui::step2::content_step2::render_compat_popup(ui, state);
        crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
        outcome
    };
    crate::ui::workspace::related_jump::apply_related_jump_outcome(orchestrator, outcome);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::prompt_popup_nav::apply_toolbar_prompt_jump;
    use crate::app::state::Step3ItemState;

    fn row(mod_name: &str, component_id: &str, is_parent: bool) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: component_id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: format!("{mod_name}::block0"),
            is_parent,
            parent_placeholder: false,
        }
    }

    fn app_with_two_mods_on_step3(tag: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.wizard_state.current_step = 0;
        app.wizard_state.step3.active_game_tab = game_authority::TAB_BGEE.to_string();
        app.wizard_state.step3.bgee_items = vec![
            row("ModA", "__PARENT__", true),
            row("ModA", "1", false),
            row("ModB", "__PARENT__", true),
            row("ModB", "1", false),
        ];
        app.wizard_state.step3.bgee_selected = vec![1];
        app
    }

    #[test]
    fn a_jump_made_while_step_3_renders_lands_on_the_step_3_row() {
        let mut app = app_with_two_mods_on_step3("step3jumpscope");

        showing_wizard_step3(&mut app, |app| {
            apply_toolbar_prompt_jump(&mut app.wizard_state, "ModB", Some(1));
        });

        assert_eq!(app.wizard_state.step3.bgee_selected, vec![3]);
        assert!(app.wizard_state.step3.jump_to_selected_requested);
        assert_eq!(app.wizard_state.current_step, 0);
    }

    #[test]
    fn the_same_jump_outside_the_step_3_render_leaves_step_3_alone() {
        let mut app = app_with_two_mods_on_step3("step3jumpoutside");

        apply_toolbar_prompt_jump(&mut app.wizard_state, "ModB", Some(1));

        assert_eq!(app.wizard_state.step3.bgee_selected, vec![1]);
        assert!(!app.wizard_state.step3.jump_to_selected_requested);
    }
}
