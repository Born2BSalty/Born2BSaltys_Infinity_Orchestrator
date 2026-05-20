// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::hash::{Hash, Hasher};

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step5::page_workspace_step5;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    match orchestrator.workspace_view.current_step {
        WorkspaceStep::Step2 => {
            let before = step2_selection_fingerprint(&orchestrator.wizard_state);
            let action = crate::ui::workspace::step2::workspace_step2::render(ui, orchestrator);
            if step2_selection_fingerprint(&orchestrator.wizard_state) != before {
                orchestrator.mark_workspace_dirty();
            }
            if let Some(a) = action {
                step_action_dispatch::dispatch_step2(a, orchestrator);
            }
        }
        WorkspaceStep::Step3 => {
            let before = step3_fingerprint(&orchestrator.wizard_state);
            crate::ui::workspace::step3::workspace_step3::render(ui, orchestrator);
            if step3_fingerprint(&orchestrator.wizard_state) != before {
                orchestrator.mark_workspace_dirty();
            }
        }
        WorkspaceStep::Step4 => {
            let action = crate::ui::workspace::step4::workspace_step4::render(ui, orchestrator);
            if let Some(a) = action {
                step_action_dispatch::dispatch_step4(a, orchestrator);
            }
        }
        WorkspaceStep::Step5 => {
            let modlist_id = orchestrator.workspace_view.modlist_id.clone();
            page_workspace_step5::render(ui, orchestrator, &modlist_id);
        }
    }
}

fn step3_fingerprint(state: &WizardState) -> u64 {
    let is_bg2ee = state.step3.active_game_tab == "BG2EE";
    let (items, collapsed) = if is_bg2ee {
        (
            &state.step3.bg2ee_items,
            &state.step3.bg2ee_collapsed_blocks,
        )
    } else {
        (&state.step3.bgee_items, &state.step3.bgee_collapsed_blocks)
    };

    let mut h = std::collections::hash_map::DefaultHasher::new();
    is_bg2ee.hash(&mut h);
    items.len().hash(&mut h);
    if let Some(first) = items.first() {
        first.tp_file.hash(&mut h);
        first.component_id.hash(&mut h);
        first.selected_order.hash(&mut h);
    }
    if let Some(last) = items.last() {
        last.tp_file.hash(&mut h);
        last.component_id.hash(&mut h);
        last.selected_order.hash(&mut h);
    }
    collapsed.len().hash(&mut h);
    for block in collapsed {
        block.hash(&mut h);
    }
    h.finish()
}

fn step2_selection_fingerprint(state: &WizardState) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (tag, mods) in [
        (0u8, &state.step2.bgee_mods),
        (1u8, &state.step2.bg2ee_mods),
    ] {
        tag.hash(&mut h);
        for m in mods {
            for c in &m.components {
                if c.checked {
                    m.tp_file.hash(&mut h);
                    c.component_id.hash(&mut h);
                    c.selected_order.hash(&mut h);
                }
            }
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step3ItemState;

    fn item(tp: &str, id: &str, order: usize) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: order,
            block_id: tp.to_string(),
            is_parent: false,
            parent_placeholder: false,
        }
    }

    #[test]
    fn fingerprint_is_stable_when_nothing_changes() {
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1), item("B/B.TP2", "2", 2)];
        let a = step3_fingerprint(&s);
        let b = step3_fingerprint(&s);
        assert_eq!(a, b, "identical state ⇒ identical fingerprint");
    }

    #[test]
    fn fingerprint_changes_on_reorder() {
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1), item("B/B.TP2", "2", 2)];
        let before = step3_fingerprint(&s);
        s.step3.bgee_items = vec![item("B/B.TP2", "2", 1), item("A/A.TP2", "0", 2)];
        assert_ne!(before, step3_fingerprint(&s), "reorder must change it");
    }

    #[test]
    fn fingerprint_changes_on_collapse_and_on_length() {
        let mut s = WizardState::default();
        s.step3.active_game_tab = "BGEE".to_string();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1)];
        let base = step3_fingerprint(&s);
        s.step3.bgee_collapsed_blocks = vec!["A/A.TP2".to_string()];
        let after_collapse = step3_fingerprint(&s);
        assert_ne!(base, after_collapse, "collapse must change it");
        s.step3.bgee_items.push(item("B/B.TP2", "2", 2));
        assert_ne!(
            after_collapse,
            step3_fingerprint(&s),
            "length change must change it"
        );
    }

    #[test]
    fn fingerprint_is_per_active_tab() {
        let mut s = WizardState::default();
        s.step3.bgee_items = vec![item("A/A.TP2", "0", 1)];
        s.step3.active_game_tab = "BGEE".to_string();
        let first_fingerprint = step3_fingerprint(&s);
        s.step3.active_game_tab = "BG2EE".to_string();
        let second_fingerprint = step3_fingerprint(&s);
        assert_ne!(
            first_fingerprint, second_fingerprint,
            "active tab is part of the fingerprint"
        );
    }
}
