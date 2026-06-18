// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step4_action::Step4Action;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_faint};
use crate::ui::workspace::step4::workspace_step4;

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
) -> Option<Step4Action> {
    let mut action: Option<Step4Action> = None;

    let is_dual = workspace_step4::is_dual_game(&orchestrator.wizard_state);
    let save_label = if is_dual {
        "Save weidu.log's"
    } else {
        "Save weidu.log"
    };

    let (active_tab, comp_count, mod_count) = active_tab_counts(&orchestrator.wizard_state);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        if redesign_btn(
            ui,
            palette,
            save_label,
            BtnOpts {
                primary: true,
                ..Default::default()
            },
        )
        .on_hover_text(crate::ui::shared::tooltip_global::STEP4_SAVE_WEIDU_LOG)
        .clicked()
        {
            action = Some(Step4Action::SaveWeiduLog);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{comp_count} components ready to install on {active_tab} \u{00B7} across {mod_count} mods"
                ))
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
            );
        });
    });

    action
}

fn active_tab_counts(state: &WizardState) -> (&'static str, usize, usize) {
    let (tab_label, items) = workspace_step4::active_tab_items(state);
    let leaves: Vec<&crate::app::state::Step3ItemState> =
        items.iter().filter(|i| !i.is_parent).collect();
    let comp_count = leaves.len();
    let mut seen: Vec<&str> = Vec::new();
    for it in &leaves {
        if !seen.iter().any(|t| t.eq_ignore_ascii_case(&it.tp_file)) {
            seen.push(&it.tp_file);
        }
    }
    (tab_label, comp_count, seen.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step3ItemState;

    fn leaf(tp: &str, id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: format!("comp {id}"),
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
        let mut p = leaf(tp, "__PARENT__");
        p.is_parent = true;
        p
    }

    #[test]
    fn counts_leaves_and_unique_mods_skipping_parents() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![
            parent("EEFIXPACK.TP2"),
            leaf("EEFIXPACK.TP2", "0"),
            leaf("EEFIXPACK.TP2", "2"),
            parent("BG1UB.TP2"),
            leaf("BG1UB.TP2", "0"),
        ];
        let (tab, comps, mods) = active_tab_counts(&s);
        assert_eq!(tab, "BGEE");
        assert_eq!(comps, 3, "3 leaves, parents excluded");
        assert_eq!(mods, 2, "2 distinct tp_files");
    }

    #[test]
    fn save_label_switches_on_eet() {
        let mut s = WizardState::default();
        s.step1.game_install = "EET".to_string();
        assert!(workspace_step4::is_dual_game(&s));
        s.step1.game_install = "BGEE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
        s.step1.game_install = "BG2EE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
        s.step1.game_install = "IWDEE".to_string();
        assert!(!workspace_step4::is_dual_game(&s));
    }
}
