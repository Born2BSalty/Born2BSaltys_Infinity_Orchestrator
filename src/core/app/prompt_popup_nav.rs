// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_jump_targets::{collect_prompt_jump_component_ids, prompt_popup_mod_ref};
use crate::app::prompt_popup_text::{
    PromptToolbarModEntry, collect_step2_prompt_toolbar_entries,
    collect_step3_prompt_toolbar_entries,
};
use crate::app::selection_jump::{step2_jump_to_target, step3_jump_to_target};
use crate::app::state::{PromptPopupMode, Step2ModState, WizardState};

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
    let game_tab = state.step2.active_game_tab.clone();
    let mod_ref = prompt_popup_mod_ref(title);
    step2_jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
    state.step2.jump_to_selected_requested = true;
}

pub(crate) fn collect_active_prompt_toolbar_entries(
    state: &WizardState,
) -> Vec<PromptToolbarModEntry> {
    if state.current_step == 2 {
        let prompt_eval = build_prompt_eval_context(state);
        let items = if state.step3.active_game_tab == "BGEE" {
            &state.step3.bgee_items
        } else {
            &state.step3.bg2ee_items
        };
        collect_step3_prompt_toolbar_entries(items, &prompt_eval)
    } else {
        collect_step2_prompt_toolbar_entries(active_step2_mods(state))
    }
}

pub(crate) fn apply_toolbar_prompt_jump(state: &mut WizardState, mod_ref: &str, component_id: u32) {
    let game_tab = if state.current_step == 2 {
        state.step3.active_game_tab.clone()
    } else {
        state.step2.active_game_tab.clone()
    };
    if state.current_step == 2 {
        let _ = step3_jump_to_target(state, &game_tab, mod_ref, Some(component_id));
    } else {
        step2_jump_to_target(state, &game_tab, mod_ref, Some(component_id));
        state.step2.jump_to_selected_requested = true;
    }
}
