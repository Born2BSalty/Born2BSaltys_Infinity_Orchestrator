// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_jump_targets::{collect_prompt_jump_component_ids, prompt_popup_mod_ref};
use crate::app::prompt_popup_text::{
    PromptToolbarModEntry, collect_step2_prompt_toolbar_entries,
    collect_step3_prompt_toolbar_entries,
};
use crate::app::selection_jump::{step2_jump_to_target, step3_jump_to_target};
use crate::app::selection_refs::normalize_mod_key;
use crate::app::state::{PromptPopupMode, Step2ModState, Step2Selection, WizardState};

pub(crate) fn open_text_prompt_popup(state: &mut WizardState, title: String, text: String) {
    state.step2.prompt_popup_mode = PromptPopupMode::Text;
    state.step2.prompt_popup_title = title;
    state.step2.prompt_popup_text = text;
    state.step2.prompt_popup_open = true;
}

pub(crate) fn open_toolbar_prompt_popup(state: &mut WizardState, title: &str) {
    state.step2.prompt_popup_mode = PromptPopupMode::ToolbarIndex;
    state.step2.prompt_popup_title = title.to_string();
    state.step2.prompt_popup_text.clear();
    state.step2.prompt_popup_open = true;
}

pub(crate) fn active_step2_mods(state: &WizardState) -> &[Step2ModState] {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

pub(crate) fn collect_text_prompt_jump_ids(
    state: &WizardState,
    title: &str,
    text: &str,
) -> Vec<u32> {
    collect_prompt_jump_component_ids(active_step2_mods(state), title, text)
}

pub(crate) fn apply_text_prompt_jump(state: &mut WizardState, title: &str, component_id: u32) {
    let mod_ref = prompt_popup_mod_ref(title);
    if state.current_step == 2 {
        let game_tab = state.step3.active_game_tab.clone();
        let _ = step3_jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
    } else {
        let game_tab = state.step2.active_game_tab.clone();
        step2_jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
        state.step2.jump_to_selected_requested = true;
    }
}

pub(crate) fn collect_active_prompt_toolbar_entries(
    state: &WizardState,
) -> Vec<PromptToolbarModEntry> {
    let prompt_eval = build_prompt_eval_context(state);
    if state.current_step == 2 {
        let items = if state.step3.active_game_tab == "BGEE" {
            &state.step3.bgee_items
        } else {
            &state.step3.bg2ee_items
        };
        let mods = if state.step3.active_game_tab == "BGEE" {
            &state.step2.bgee_mods
        } else {
            &state.step2.bg2ee_mods
        };
        collect_step3_prompt_toolbar_entries(items, mods, &prompt_eval)
    } else {
        collect_step2_prompt_toolbar_entries(active_step2_mods(state), &prompt_eval)
    }
}

pub(crate) fn apply_toolbar_prompt_jump(
    state: &mut WizardState,
    mod_ref: &str,
    component_id: Option<u32>,
) {
    let game_tab = if state.current_step == 2 {
        state.step3.active_game_tab.clone()
    } else {
        state.step2.active_game_tab.clone()
    };
    if state.current_step == 2 {
        let _ = step3_jump_to_target(state, &game_tab, mod_ref, component_id);
    } else if let Some(component_id) = component_id {
        step2_jump_to_target(state, &game_tab, mod_ref, Some(component_id));
        state.step2.jump_to_selected_requested = true;
    } else {
        select_step2_mod_row(state, &game_tab, mod_ref);
    }
}

fn select_step2_mod_row(state: &mut WizardState, game_tab: &str, mod_ref: &str) {
    let target_key = normalize_mod_key(mod_ref);
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let Some(tp_file) = mods
        .iter()
        .find(|mod_state| normalize_mod_key(&mod_state.tp_file) == target_key)
        .map(|mod_state| mod_state.tp_file.clone())
    else {
        return;
    };
    state.step2.selected = Some(Step2Selection::Mod {
        game_tab: game_tab.to_string(),
        tp_file,
    });
    state.step2.active_game_tab = game_tab.to_string();
    state.step2.jump_to_selected_requested = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_mod(tp_file: &str) -> WizardState {
        let mut state = WizardState {
            current_step: 1,
            ..WizardState::default()
        };
        state.step2.active_game_tab = "BGEE".to_string();
        state.step2.bgee_mods = vec![Step2ModState {
            name: "DLC Merger".to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: Vec::new(),
        }];
        state
    }

    #[test]
    fn mod_level_toolbar_jump_selects_the_parent_mod_row() {
        let mut state = state_with_mod("DLCMERGER/DLCMERGER.TP2");
        apply_toolbar_prompt_jump(&mut state, "dlcmerger/dlcmerger.tp2", None);
        match state.step2.selected.as_ref() {
            Some(Step2Selection::Mod { game_tab, tp_file }) => {
                assert_eq!(game_tab, "BGEE");
                assert_eq!(tp_file, "DLCMERGER/DLCMERGER.TP2");
            }
            _ => panic!("expected the parent mod row to be selected"),
        }
        assert!(state.step2.jump_to_selected_requested);
    }
}
