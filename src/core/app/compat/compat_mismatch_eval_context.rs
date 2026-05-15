// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::Path;

use crate::app::state::Step1State;

use super::super::compat_rule_runtime::{
    game_dir_for_tab as shared_game_dir_for_tab, normalize_mod_key,
};

pub(crate) fn build_mismatch_context(
    step1: &Step1State,
    tab: &str,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    let include_eet = is_eet_core_selected(&checked_components);
    if tab.eq_ignore_ascii_case("BGEE") {
        return build_bgee_context(step1, tab, checked_components);
    }

    build_bg2ee_context(include_eet, checked_components)
}

fn build_bgee_context(
    step1: &Step1State,
    tab: &str,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    let mut active_games = HashSet::<String>::new();
    let mut active_engines = HashSet::<String>::new();
    let mut active_includes = HashSet::<String>::new();
    let mut uncertain_includes = HashSet::<String>::new();

    active_games.insert("bgee".to_string());
    active_engines.insert("bgee".to_string());
    active_includes.insert("bg1".to_string());
    active_includes.insert("totsc".to_string());

    if let Some(game_dir) = shared_game_dir_for_tab(step1, tab) {
        if detect_sod(game_dir) {
            active_includes.insert("sod".to_string());
        } else {
            uncertain_includes.insert("sod".to_string());
        }
    } else {
        uncertain_includes.insert("sod".to_string());
    }

    MismatchContext {
        active_games,
        active_engines,
        active_includes,
        uncertain_includes,
        checked_components,
    }
}

fn build_bg2ee_context(
    include_eet: bool,
    checked_components: HashSet<(String, String)>,
) -> MismatchContext {
    let mut active_games = HashSet::<String>::new();
    let mut active_engines = HashSet::<String>::new();
    let mut active_includes = HashSet::<String>::new();

    active_engines.insert("bg2ee".to_string());
    if include_eet {
        active_games.insert("eet".to_string());
    } else {
        active_games.insert("bg2ee".to_string());
    }
    active_includes.insert("bg2".to_string());
    active_includes.insert("soa".to_string());
    active_includes.insert("tob".to_string());

    if include_eet {
        active_includes.insert("bg1".to_string());
        active_includes.insert("totsc".to_string());
        active_includes.insert("sod".to_string());
    }

    MismatchContext {
        active_games,
        active_engines,
        active_includes,
        uncertain_includes: HashSet::new(),
        checked_components,
    }
}

fn is_eet_core_selected(checked_components: &HashSet<(String, String)>) -> bool {
    checked_components.contains(&(normalize_mod_key("EET.TP2"), "0".to_string()))
}

#[derive(Debug, Default)]
pub(crate) struct MismatchContext {
    active_games: HashSet<String>,
    active_engines: HashSet<String>,
    active_includes: HashSet<String>,
    uncertain_includes: HashSet<String>,
    checked_components: HashSet<(String, String)>,
}

impl MismatchContext {
    pub(crate) fn has_checked_component(&self, mod_name: &str, component_id: &str) -> bool {
        let component_id = component_id.trim();
        if component_id.is_empty() {
            return false;
        }
        self.checked_components
            .contains(&(normalize_mod_key(mod_name), component_id.to_string()))
    }

    pub(super) fn eval_game_is(&self, values: &[String]) -> TriState {
        if values.is_empty() {
            return TriState::Unknown;
        }
        TriState::from_bool(
            values
                .iter()
                .map(|value| normalize_game_token(value))
                .any(|value| self.active_games.contains(&value)),
        )
    }

    pub(super) fn eval_engine_is(&self, values: &[String]) -> TriState {
        if values.is_empty() {
            return TriState::Unknown;
        }
        TriState::from_bool(
            values
                .iter()
                .map(|value| normalize_game_token(value))
                .any(|value| self.active_engines.contains(&value)),
        )
    }

    pub(super) fn eval_game_includes(&self, values: &[String]) -> TriState {
        if values.is_empty() {
            return TriState::Unknown;
        }
        if values
            .iter()
            .map(|value| normalize_include_token(value))
            .any(|value| self.active_includes.contains(&value))
        {
            TriState::True
        } else if values
            .iter()
            .map(|value| normalize_include_token(value))
            .any(|value| self.uncertain_includes.contains(&value))
        {
            TriState::Unknown
        } else {
            TriState::False
        }
    }

    pub(super) fn eval_mod_is_installed(&self, mod_name: &str, component_id: &str) -> TriState {
        let component_id = component_id.trim();
        if component_id.is_empty() {
            return TriState::Unknown;
        }
        TriState::from_bool(self.has_checked_component(mod_name, component_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriState {
    True,
    False,
    Ignored,
    Unknown,
}

impl TriState {
    pub(super) const fn from_bool(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }

    pub(super) const fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    pub(super) const fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::Ignored, value) | (value, Self::Ignored) => value,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    pub(super) const fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Ignored => Self::Ignored,
            Self::Unknown => Self::Unknown,
        }
    }
}

fn detect_sod(game_dir: &str) -> bool {
    let root = Path::new(game_dir);
    [
        root.join("dlc").join("sod-dlc.zip"),
        root.join("DLC").join("sod-dlc.zip"),
        root.join("movies").join("sodcin01.wbm"),
        root.join("Movies").join("sodcin01.wbm"),
    ]
    .into_iter()
    .any(|path| path.exists())
}

fn normalize_game_token(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '~' | '\'' | '.' | ',' | ';'))
        .to_ascii_lowercase();
    match normalized.as_str() {
        "bg:ee" | "bg-ee" | "bg1ee" => "bgee".to_string(),
        "bgii:ee" | "bgii-ee" | "bg2:ee" => "bg2ee".to_string(),
        "iwd:ee" => "iwdee".to_string(),
        _ => normalized,
    }
}

fn normalize_include_token(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '~' | '\'' | '.' | ',' | ';'))
        .to_ascii_lowercase();
    match normalized.as_str() {
        "soa" => "bg2".to_string(),
        "totsc" => "bg1".to_string(),
        _ => normalized,
    }
}
