// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::state::{Step2ComponentState, Step2ModState, WizardState};

#[test]
fn bg2ee_tab_only_counts_as_eet_after_eet_core_is_checked() {
    let mut state = WizardState::default();
    state.step1.game_install = "EET".to_string();
    state.step2.active_game_tab = "BG2EE".to_string();

    let prompt_eval = build_prompt_eval_context(&state);
    assert!(!prompt_eval.active_games.contains("eet"));
    assert!(prompt_eval.active_games.contains("bg2ee"));
    assert!(prompt_eval.active_engines.contains("bg2ee"));

    state.step2.bg2ee_mods.push(eet_core_mod(true));
    let prompt_eval = build_prompt_eval_context(&state);
    assert!(prompt_eval.active_games.contains("eet"));
    assert!(!prompt_eval.active_games.contains("bg2ee"));
    assert!(prompt_eval.active_engines.contains("bg2ee"));
}

#[test]
fn bgee_tab_does_not_count_as_eet_even_when_eet_core_is_checked() {
    let mut state = WizardState::default();
    state.step1.game_install = "EET".to_string();
    state.step2.active_game_tab = "BGEE".to_string();
    state.step2.bg2ee_mods.push(eet_core_mod(true));

    let prompt_eval = build_prompt_eval_context(&state);
    assert!(prompt_eval.active_games.contains("bgee"));
    assert!(!prompt_eval.active_games.contains("eet"));
    assert!(prompt_eval.active_engines.contains("bgee"));
    assert!(!prompt_eval.active_engines.contains("bg2ee"));
}

fn eet_core_mod(checked: bool) -> Step2ModState {
    Step2ModState {
        name: "EET".to_string(),
        tp_file: "EET.TP2".to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked,
        hidden_components: Vec::new(),
        components: vec![Step2ComponentState {
            component_id: "0".to_string(),
            label: "EET core".to_string(),
            weidu_group: None,
            subcomponent_key: None,
            tp2_empty_placeholder_block: false,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: None,
        }],
    }
}
