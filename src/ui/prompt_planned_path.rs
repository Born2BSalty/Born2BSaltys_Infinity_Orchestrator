// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step1State;
use crate::parser::prompt_eval_expr::normalize_tp2_stem;

pub(super) fn planned_eet_bgee_path(step1: &Step1State) -> Option<&str> {
    if !step1.game_install.eq_ignore_ascii_case("EET") {
        return None;
    }
    let path = if step1.new_pre_eet_dir_enabled {
        &step1.eet_pre_dir
    } else {
        &step1.eet_bgee_game_folder
    }
    .trim();
    (!path.is_empty()).then_some(path)
}

pub(super) fn render_eet_planned_paths(
    text: &str,
    mod_identity: &str,
    planned_bgee_path: Option<&str>,
) -> String {
    if normalize_tp2_stem(mod_identity) == "eet"
        && let Some(path) = planned_bgee_path
    {
        return text.replace("%bgee_dir%", path);
    }
    text.to_string()
}

#[cfg(test)]
#[path = "prompt_planned_path_tests.rs"]
mod tests;
