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

pub(super) fn game_allowed(current_game: &str, allowed_games: &[String]) -> bool {
    if allowed_games
        .iter()
        .any(|g| g.eq_ignore_ascii_case(current_game))
    {
        return true;
    }

    // EET runs BGEE/BG2EE content in a unified runtime.
    // Treat common IE targets as compatible aliases to reduce false GAME_IS mismatches.
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
