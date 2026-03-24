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
