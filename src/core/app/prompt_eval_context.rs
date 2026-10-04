// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::app::game_authority::{self, GameSlot};
use crate::app::state::{Step2ModState, WizardState};
use crate::parser::prompt_eval_expr::{PromptEvalContext, normalize_tp2_stem};

#[must_use]
pub fn build_prompt_eval_context(state: &WizardState) -> PromptEvalContext {
    let mut checked_components = HashSet::<(String, String)>::new();
    collect_checked_components(&state.step2.bgee_mods, &mut checked_components);
    collect_checked_components(&state.step2.bg2ee_mods, &mut checked_components);

    let active_games = build_active_games(state, &checked_components);
    let active_engines = build_active_engines(state);

    let game_dir = if game_authority::slot_for_tab(&state.step2.active_game_tab) == GameSlot::First
    {
        let identity = game_authority::first_slot_tab(&state.step1.game_install);
        if identity == game_authority::TAB_IWDEE {
            if state.step1.generate_directory_enabled
                && !state.step1.generate_directory.trim().is_empty()
            {
                Some(state.step1.generate_directory.clone())
            } else {
                Some(state.step1.iwdee_game_folder.clone())
            }
        } else if state.step1.game_install.eq_ignore_ascii_case("EET") {
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
    } else if state.step1.game_install.eq_ignore_ascii_case("EET") {
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
    .filter(|v| !v.trim().is_empty());

    let signature = build_prompt_eval_signature(
        &active_games,
        &active_engines,
        game_dir.as_deref(),
        &checked_components,
    );

    PromptEvalContext {
        active_games,
        active_engines,
        game_dir,
        checked_components,
        signature,
    }
}

fn build_prompt_eval_signature(
    active_games: &HashSet<String>,
    active_engines: &HashSet<String>,
    game_dir: Option<&str>,
    checked_components: &HashSet<(String, String)>,
) -> String {
    let mut games = active_games.iter().cloned().collect::<Vec<_>>();
    games.sort_unstable();
    let mut engines = active_engines.iter().cloned().collect::<Vec<_>>();
    engines.sort_unstable();

    let mut checked = checked_components
        .iter()
        .map(|(mod_key, component_id)| format!("{mod_key}|{component_id}"))
        .collect::<Vec<_>>();
    checked.sort_unstable();

    format!(
        "games={};engines={};dir={};checked={}",
        games.join(","),
        engines.join(","),
        game_dir.unwrap_or(""),
        checked.join(";")
    )
}

fn collect_checked_components(
    mods: &[Step2ModState],
    checked_components: &mut HashSet<(String, String)>,
) {
    for mod_state in mods {
        let mod_key = normalize_tp2_stem(&mod_state.tp_file);
        for component in &mod_state.components {
            if component.checked {
                checked_components
                    .insert((mod_key.clone(), component.component_id.trim().to_string()));
            }
        }
    }
}

fn build_active_games(
    state: &WizardState,
    checked_components: &HashSet<(String, String)>,
) -> HashSet<String> {
    let mut active_games = HashSet::<String>::new();
    if game_authority::slot_for_tab(&state.step2.active_game_tab) == GameSlot::First {
        let identity = game_authority::first_slot_tab(&state.step1.game_install);
        if identity == game_authority::TAB_IWDEE {
            active_games.insert("iwdee".to_string());
        } else {
            active_games.insert("bgee".to_string());
        }
    } else if is_eet_core_selected(checked_components) {
        active_games.insert("eet".to_string());
    } else {
        active_games.insert("bg2ee".to_string());
    }
    active_games
}

fn build_active_engines(state: &WizardState) -> HashSet<String> {
    let mut active_engines = HashSet::<String>::new();
    if game_authority::slot_for_tab(&state.step2.active_game_tab) == GameSlot::First {
        let identity = game_authority::first_slot_tab(&state.step1.game_install);
        if identity == game_authority::TAB_IWDEE {
            active_engines.insert("iwdee".to_string());
        } else {
            active_engines.insert("bgee".to_string());
        }
    } else {
        active_engines.insert("bg2ee".to_string());
    }
    active_engines
}

fn is_eet_core_selected(checked_components: &HashSet<(String, String)>) -> bool {
    checked_components.contains(&("eet".to_string(), "0".to_string()))
}

#[cfg(test)]
mod tests {
    use super::build_prompt_eval_context;
    use crate::app::state::{Step1State, Step2State, WizardState};

    fn state_for(game_install: &str, active_game_tab: &str) -> WizardState {
        WizardState {
            step1: Step1State {
                game_install: game_install.to_string(),
                iwdee_game_folder: "/games/iwdee".to_string(),
                bgee_game_folder: "/games/bgee".to_string(),
                generate_directory_enabled: false,
                ..Step1State::default()
            },
            step2: Step2State {
                active_game_tab: active_game_tab.to_string(),
                ..Step2State::default()
            },
            ..WizardState::default()
        }
    }

    #[test]
    fn prompt_context_for_iwdee_reads_the_iwdee_folder() {
        for tab in ["BGEE", "IWDEE"] {
            let state = state_for("IWDEE", tab);
            let context = build_prompt_eval_context(&state);
            assert_eq!(context.game_dir.as_deref(), Some("/games/iwdee"));
            assert!(context.active_games.contains("iwdee"));
            assert!(context.active_engines.contains("iwdee"));
        }
    }
}
