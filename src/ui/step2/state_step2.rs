// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::ui::state::{Step2ModState, Step2State, WizardState};

#[derive(Debug, Clone, Default)]
pub struct PromptEvalContext {
    pub active_games: HashSet<String>,
    pub game_dir: Option<String>,
    pub checked_components: HashSet<(String, String)>,
}

pub fn build_prompt_eval_context(state: &WizardState) -> PromptEvalContext {
    let mut checked_components = HashSet::<(String, String)>::new();
    collect_checked_components(&state.step2.bgee_mods, &mut checked_components);
    collect_checked_components(&state.step2.bg2ee_mods, &mut checked_components);

    let active_games = build_active_games(state);

    let game_dir = match state.step2.active_game_tab.as_str() {
        "BGEE" => {
            if state.step1.game_install.eq_ignore_ascii_case("EET") {
                if state.step1.new_pre_eet_dir_enabled && !state.step1.eet_pre_dir.trim().is_empty() {
                    Some(state.step1.eet_pre_dir.clone())
                } else {
                    Some(state.step1.eet_bgee_game_folder.clone())
                }
            } else if state.step1.generate_directory_enabled
                && !state.step1.generate_directory.trim().is_empty()
            {
                Some(state.step1.generate_directory.clone())
            } else {
                Some(state.step1.bgee_game_folder.clone())
            }
        }
        _ => {
            if state.step1.game_install.eq_ignore_ascii_case("EET") {
                if state.step1.new_eet_dir_enabled && !state.step1.eet_new_dir.trim().is_empty() {
                    Some(state.step1.eet_new_dir.clone())
                } else {
                    Some(state.step1.eet_bg2ee_game_folder.clone())
                }
            } else if state.step1.generate_directory_enabled
                && !state.step1.generate_directory.trim().is_empty()
            {
                Some(state.step1.generate_directory.clone())
            } else {
                Some(state.step1.bg2ee_game_folder.clone())
            }
        }
    }
    .filter(|v| !v.trim().is_empty());

    PromptEvalContext {
        active_games,
        game_dir,
        checked_components,
    }
}

fn collect_checked_components(
    mods: &[Step2ModState],
    checked_components: &mut HashSet<(String, String)>,
) {
    for mod_state in mods {
        let mod_key = normalize_tp2_stem(&mod_state.tp_file);
        for component in &mod_state.components {
            if component.checked {
                checked_components.insert((mod_key.clone(), component.component_id.trim().to_string()));
            }
        }
    }
}

fn build_active_games(state: &WizardState) -> HashSet<String> {
    let mut active_games = HashSet::<String>::new();
    match state.step2.active_game_tab.as_str() {
        "BGEE" => {
            active_games.insert("bgee".to_string());
            if state.step1.game_install.eq_ignore_ascii_case("EET") {
                active_games.insert("eet".to_string());
            }
        }
        _ => {
            active_games.insert("bg2ee".to_string());
            if state.step1.game_install.eq_ignore_ascii_case("EET") {
                active_games.insert("eet".to_string());
            }
        }
    }
    active_games
}

pub fn normalize_tp2_stem(value: &str) -> String {
    let lower = value.replace('\\', "/").to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let no_ext = file.strip_suffix(".tp2").unwrap_or(file);
    no_ext.strip_prefix("setup-").unwrap_or(no_ext).to_string()
}

#[derive(Debug, Clone, Default)]
pub struct Step2Details {
    pub mod_name: Option<String>,
    pub component_label: Option<String>,
    pub component_id: Option<String>,
    pub component_lang: Option<String>,
    pub component_version: Option<String>,
    pub selected_order: Option<usize>,
    pub is_checked: Option<bool>,
    pub is_disabled: Option<bool>,
    pub compat_kind: Option<String>,
    pub compat_role: Option<String>,
    pub compat_code: Option<String>,
    pub disabled_reason: Option<String>,
    pub compat_source: Option<String>,
    pub compat_related_mod: Option<String>,
    pub compat_related_component: Option<String>,
    pub compat_related_target: Option<String>,
    pub compat_graph: Option<String>,
    pub compat_evidence: Option<String>,
    pub compat_component_block: Option<String>,
    pub raw_line: Option<String>,
    pub tp_file: Option<String>,
    pub tp2_path: Option<String>,
    pub readme_path: Option<String>,
    pub web_url: Option<String>,
}

pub fn normalize_active_tab(state: &mut WizardState) {
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let active_is_visible =
        (state.step2.active_game_tab == "BGEE" && show_bgee)
            || (state.step2.active_game_tab == "BG2EE" && show_bg2ee);
    if active_is_visible {
        return;
    }
    if show_bgee {
        state.step2.active_game_tab = "BGEE".to_string();
    } else if show_bg2ee {
        state.step2.active_game_tab = "BG2EE".to_string();
    }
}

pub fn active_mods_mut(step2: &mut Step2State) -> &mut Vec<Step2ModState> {
    if step2.active_game_tab == "BGEE" {
        &mut step2.bgee_mods
    } else {
        &mut step2.bg2ee_mods
    }
}
