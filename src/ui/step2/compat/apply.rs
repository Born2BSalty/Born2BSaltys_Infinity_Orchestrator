// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step1State, Step2ModState};
use std::collections::HashMap;
use std::path::Path;

use crate::compat::model::Tp2Rule;
use crate::compat::tp2_parse::parse_tp2_rules;
use super::loader::load_rules;
use super::matcher::{match_rule, rule_disables_component};

pub fn apply_step2_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    let rules = load_rules();
    if rules.is_empty() {
        clear_all_disables(bgee_mods);
        clear_all_disables(bg2ee_mods);
        return;
    }
    apply_for_tab(step1, "BGEE", bgee_mods, &rules);
    apply_for_tab(step1, "BG2EE", bg2ee_mods, &rules);
}

fn clear_all_disables(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = None;
            component.disabled_reason = None;
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

fn apply_for_tab(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
    rules: &[super::model::Step2CompatRule],
) {
    let mut tp2_cache = HashMap::<String, crate::compat::model::Tp2Metadata>::new();
    let current_game = normalize_game_mode(&step1.game_install);

    for mod_state in mods {
        let mod_name = mod_state.name.clone();
        let tp_file = mod_state.tp_file.clone();
        let tp2_path = mod_state.tp2_path.clone();

        let metadata = if tp2_path.trim().is_empty() {
            None
        } else {
            if !tp2_cache.contains_key(&tp2_path) {
                tp2_cache.insert(tp2_path.clone(), parse_tp2_rules(Path::new(&tp2_path)));
            }
            tp2_cache.get(&tp2_path)
        };

        for component in &mut mod_state.components {
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = None;
            component.disabled_reason = None;
            for rule in rules {
                if !match_rule(rule, step1, tab, &mod_name, &tp_file, component) {
                    continue;
                }
                component.compat_kind = Some(rule.kind.trim().to_ascii_lowercase());
                component.compat_source = Some(
                    rule.source
                        .clone()
                        .unwrap_or_else(|| "step2_compat_rules.toml".to_string()),
                );
                component.compat_related_mod = rule.related_mod.clone();
                component.compat_related_component = rule.related_component.clone();
                component.compat_graph = None;
                component.compat_evidence = None;
                if rule_disables_component(rule) {
                    component.disabled = true;
                    component.checked = false;
                    component.selected_order = None;
                }
                if !rule.message.trim().is_empty() {
                    component.disabled_reason = Some(rule.message.clone());
                }
                break;
            }

            if let Some(meta) = metadata
                && let Some(component_id) = component.component_id.trim().parse::<u32>().ok()
                && let Some((allowed_games, rule_evidence)) =
                    find_require_game_allowed_games(meta, component_id)
                && !game_allowed(&current_game, allowed_games)
            {
                component.compat_kind = Some("game_mismatch".to_string());
                component.compat_source =
                    Some(format!("step2_tp2_game_validator | TP2: {}", meta.tp_file));
                component.compat_related_mod = Some(allowed_games.join("|"));
                component.compat_related_component = None;
                component.compat_graph = Some(format!(
                    "{} #{} allowed on: {}",
                    normalize_mod_key(&mod_state.tp_file),
                    component_id,
                    allowed_games.join("|")
                ));
                component.compat_evidence = Some(rule_evidence.to_string());
                component.disabled = true;
                component.checked = false;
                component.selected_order = None;
                component.disabled_reason = Some(format!(
                    "This component is restricted to: {}.",
                    allowed_games.join(", ")
                ));
            }
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

fn find_require_game_allowed_games<'a>(
    meta: &'a crate::compat::model::Tp2Metadata,
    component_id: u32,
) -> Option<(&'a [String], &'a str)> {
    for (cid, rule) in &meta.rules {
        if *cid != component_id {
            continue;
        }
        if let Tp2Rule::RequireGame {
            allowed_games,
            raw_line,
            ..
        } = rule
        {
            return Some((allowed_games.as_slice(), raw_line.as_str()));
        }
    }
    None
}

fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

fn normalize_game_mode(game_mode: &str) -> String {
    match game_mode.to_ascii_uppercase().as_str() {
        "BGEE" => "bgee".to_string(),
        "BG2EE" => "bg2ee".to_string(),
        "EET" => "eet".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn game_allowed(current_game: &str, allowed_games: &[String]) -> bool {
    if allowed_games
        .iter()
        .any(|g| g.eq_ignore_ascii_case(current_game))
    {
        return true;
    }
    if current_game.eq_ignore_ascii_case("eet") {
        return allowed_games.iter().any(|g| {
            g.eq_ignore_ascii_case("bgee")
                || g.eq_ignore_ascii_case("bg2ee")
                || g.eq_ignore_ascii_case("eet")
                || g.eq_ignore_ascii_case("bgt")
                || g.eq_ignore_ascii_case("tob")
        });
    }
    false
}
