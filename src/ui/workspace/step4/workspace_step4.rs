// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{Step3ItemState, WizardState};
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, ThemePalette, redesign_pill_danger, redesign_pill_text,
};
use crate::ui::workspace::step4::{step4_exact_log_viewer, step4_review_list, step4_save_row};
use crate::ui::workspace::widgets::game_tab::game_tab;

const TAB_GAP: f32 = 4.0;
const TAB_TO_BODY_OVERLAP: f32 = 1.5;
const SAVE_ROW_GAP: f32 = 10.0;

#[must_use]
pub fn is_dual_game(state: &WizardState) -> bool {
    state.step1.game_install == "EET"
}

#[must_use]
pub fn active_tab_items(state: &WizardState) -> (&'static str, &[Step3ItemState]) {
    match state.step1.game_install.as_str() {
        "BG2EE" => ("BG2EE", &state.step3.bg2ee_items),
        "EET" if state.step3.active_game_tab == "BG2EE" => ("BG2EE", &state.step3.bg2ee_items),
        _ => ("BGEE", &state.step3.bgee_items),
    }
}

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step4Action> {
    let palette = orchestrator.theme_palette;
    let mut action: Option<Step4Action> = None;

    if is_dual_game(&orchestrator.wizard_state) {
        let t = &mut orchestrator.wizard_state.step3.active_game_tab;
        if t != "BGEE" && t != "BG2EE" {
            *t = "BGEE".to_string();
        }
    }

    if let Some(a) = step4_save_row::render(ui, orchestrator, palette) {
        action = Some(a);
    }
    ui.add_space(SAVE_ROW_GAP);

    if is_dual_game(&orchestrator.wizard_state) {
        render_game_tab_strip(
            ui,
            palette,
            &mut orchestrator.wizard_state.step3.active_game_tab,
        );
        ui.add_space(-TAB_TO_BODY_OVERLAP);
    }

    let exact_log_mode = orchestrator
        .wizard_state
        .step1
        .installs_exactly_from_weidu_logs();

    if exact_log_mode {
        if let Some(a) = step4_exact_log_viewer::render(ui, orchestrator, palette) {
            action = Some(a);
        }
    } else {
        let (active_tab, items) = active_tab_items(&orchestrator.wizard_state);
        step4_review_list::render(ui, palette, items, active_tab);
    }

    surface_save_error(ui, palette, orchestrator);

    action
}

fn render_game_tab_strip(ui: &mut egui::Ui, palette: ThemePalette, active: &mut String) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TAB_GAP;
        game_tab(ui, palette, "BGEE", active);
        game_tab(ui, palette, "BG2EE", active);
    });
}

fn surface_save_error(ui: &mut egui::Ui, palette: ThemePalette, orchestrator: &OrchestratorApp) {
    let last = orchestrator.wizard_state.step5.last_status_text.trim();
    if !last.starts_with("Save weidu.log failed") {
        return;
    }
    if orchestrator
        .wizard_state
        .step2
        .scan_status
        .starts_with("Saved weidu.log file(s)")
    {
        return;
    }
    ui.add_space(6.0);
    let pad_x = 8.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let text_color = redesign_pill_text(palette);
    let galley = ui
        .painter()
        .layout_no_wrap(last.to_string(), font.clone(), text_color);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0),
        egui::Sense::hover(),
    );
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
            redesign_pill_danger(palette),
        );
        painter.text(
            egui::pos2(rect.left() + pad_x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            last,
            font,
            text_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("c{id}"),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }

    #[test]
    fn dual_game_is_eet_only() {
        let mut s = WizardState::default();
        for (g, dual) in [
            ("EET", true),
            ("BGEE", false),
            ("BG2EE", false),
            ("IWDEE", false),
        ] {
            s.step1.game_install = g.to_string();
            assert_eq!(is_dual_game(&s), dual, "game {g}");
        }
    }

    #[test]
    fn active_tab_items_match_bio_resolution() {
        let mut s = WizardState::default();
        s.step3.bgee_items = vec![leaf("A.TP2", "0")];
        s.step3.bg2ee_items = vec![leaf("B.TP2", "0"), leaf("B.TP2", "1")];

        s.step1.game_install = "BGEE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
        assert_eq!(it.len(), 1);

        s.step1.game_install = "BG2EE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BG2EE");
        assert_eq!(it.len(), 2);

        s.step1.game_install = "EET".to_string();
        s.step3.active_game_tab = "BG2EE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BG2EE");
        assert_eq!(it.len(), 2);
        s.step3.active_game_tab = "BGEE".to_string();
        let (t, it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
        assert_eq!(it.len(), 1);

        s.step1.game_install = "IWDEE".to_string();
        let (t, _it) = active_tab_items(&s);
        assert_eq!(t, "BGEE");
    }
}
