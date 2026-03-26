// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};

use super::SelectedComponent;

pub(super) fn build_selected_set(selected: &[SelectedComponent]) -> HashSet<(String, u32)> {
    selected
        .iter()
        .map(|c| (normalize_mod_key(&c.tp_file), c.component_id))
        .collect()
}

pub(super) fn build_order_map(selected: &[SelectedComponent]) -> HashMap<(String, u32), usize> {
    selected
        .iter()
        .map(|c| ((normalize_mod_key(&c.tp_file), c.component_id), c.order))
        .collect()
}

pub(super) fn matching_orders_for_target(
    order_map: &HashMap<(String, u32), usize>,
    target_mod: &str,
    target_component: Option<u32>,
) -> Vec<(u32, usize)> {
    match target_component {
        Some(cid) => order_map
            .get(&(target_mod.to_string(), cid))
            .map(|order| vec![(cid, *order)])
            .unwrap_or_default(),
        None => order_map
            .iter()
            .filter_map(|((mod_name, cid), order)| {
                if mod_name == target_mod {
                    Some((*cid, *order))
                } else {
                    None
                }
            })
            .collect(),
    }
}

pub(super) fn matching_orders_for_targets(
    order_map: &HashMap<(String, u32), usize>,
    targets: &[(String, Option<u32>)],
) -> Vec<(String, u32, usize)> {
    let mut matches = Vec::new();
    for (target_mod, target_component) in targets {
        match target_component {
            Some(cid) => {
                if let Some(order) = order_map.get(&(target_mod.clone(), *cid)) {
                    matches.push((target_mod.clone(), *cid, *order));
                }
            }
            None => {
                for ((mod_key, cid), order) in order_map {
                    if mod_key == target_mod {
                        matches.push((target_mod.clone(), *cid, *order));
                    }
                }
            }
        }
    }
    matches
}

pub(super) fn normalize_mod_key(tp_file: &str) -> String {
    let lower = tp_file.to_ascii_lowercase();
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

pub(super) fn normalize_game_mode(game_mode: &str) -> String {
    match game_mode.to_ascii_uppercase().as_str() {
        "BGEE" => "bgee".to_string(),
        "BG2EE" => "bg2ee".to_string(),
        "EET" => "eet".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

pub(super) fn game_includes(current_game: &str, required_games: &[String]) -> bool {
    required_games
        .iter()
        .all(|required| single_game_included(current_game, required))
}

fn single_game_included(current_game: &str, required_game: &str) -> bool {
    if current_game.eq_ignore_ascii_case(required_game) {
        return true;
    }

    match current_game.to_ascii_lowercase().as_str() {
        "bgee" => required_game.eq_ignore_ascii_case("bgee"),
        "bg2ee" => {
            required_game.eq_ignore_ascii_case("bg2ee")
                || required_game.eq_ignore_ascii_case("soa")
                || required_game.eq_ignore_ascii_case("tob")
        }
        "eet" => {
            required_game.eq_ignore_ascii_case("bgee")
                || required_game.eq_ignore_ascii_case("bg2ee")
                || required_game.eq_ignore_ascii_case("eet")
                || required_game.eq_ignore_ascii_case("soa")
                || required_game.eq_ignore_ascii_case("tob")
                || required_game.eq_ignore_ascii_case("bgt")
        }
        _ => false,
    }
}

pub(super) fn game_allowed(current_game: &str, allowed_games: &[String]) -> bool {
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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SameModBlockMeaning {
    Included,
    Order,
}

pub(super) fn classify_same_mod_block(
    metadata: &super::super::model::Tp2Metadata,
    affected_tp_file: &str,
    target_mod: &str,
    raw_line: &str,
) -> Option<SameModBlockMeaning> {
    if normalize_mod_key(affected_tp_file) != normalize_mod_key(target_mod) {
        return None;
    }

    let resolved = resolve_rule_message(metadata, raw_line)?;
    let lower = resolved.to_ascii_lowercase();
    if lower.contains("contents have already been installed")
        || lower.contains("already been installed")
        || lower.contains("already included")
        || lower.contains("superseded")
    {
        return Some(SameModBlockMeaning::Included);
    }
    if lower.contains("must be installed before")
        || lower.contains("installed before")
        || lower.contains("not after")
    {
        return Some(SameModBlockMeaning::Order);
    }
    None
}

pub(super) fn resolve_rule_message(
    metadata: &super::super::model::Tp2Metadata,
    raw_line: &str,
) -> Option<String> {
    let tra_ref = extract_trailing_tra_reference(raw_line)?;
    metadata.setup_tra.get(tra_ref.as_str()).cloned()
}

pub(super) fn resolved_reason_or(
    metadata: &super::super::model::Tp2Metadata,
    raw_line: &str,
    fallback: String,
) -> String {
    resolve_rule_message(metadata, raw_line).unwrap_or(fallback)
}

pub(super) fn component_block_for(
    metadata: &super::super::model::Tp2Metadata,
    component_id: u32,
) -> Option<String> {
    metadata.component_blocks.get(&component_id).cloned()
}

fn extract_trailing_tra_reference(raw_line: &str) -> Option<String> {
    for token in raw_line.split_whitespace().rev() {
        let trimmed = token.trim_matches(|c: char| matches!(c, ')' | '(' | '[' | ']' | ',' | ';'));
        if let Some(rest) = trimmed.strip_prefix('@')
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
        {
            return Some(rest.to_string());
        }
    }
    None
}
