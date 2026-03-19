// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io;
use std::path::PathBuf;

use crate::ui::state::{CompatState, Step1State, Step2ModState, Step2State};

pub(crate) fn create_default_compat_rules_file() -> io::Result<PathBuf> {
    crate::ui::step2::service_compat_defaults_step2::create_default_compat_rules_file()
}

pub(crate) fn apply_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    compat_apply::apply_step2_compat_rules(step1, bgee_mods, bg2ee_mods);
}

pub(crate) fn export_compat_report(step2: &Step2State, compat: &CompatState) -> io::Result<PathBuf> {
    crate::ui::step2::service_compat_report_step2::export_compat_report(step2, compat)
}

pub(crate) mod compat_model {
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Step2CompatRulesFile {
    #[serde(default)]
    pub rules: Vec<Step2CompatRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Step2CompatRule {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default, alias = "mod_name")]
    pub r#mod: String,
    #[serde(default)]
    pub component: Option<String>,
    #[serde(default)]
    pub component_id: Option<String>,
    #[serde(default)]
    pub mode: Option<StringOrMany>,
    #[serde(default)]
    pub tab: Option<StringOrMany>,
    #[serde(default, alias = "issue")]
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub related_mod: Option<String>,
    #[serde(default)]
    pub related_component: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringOrMany {
    One(String),
    Many(Vec<String>),
}

impl StringOrMany {
    pub fn normalized_items(&self) -> Vec<String> {
        match self {
            Self::One(v) => vec![v.to_ascii_uppercase()],
            Self::Many(v) => v.iter().map(|s| s.to_ascii_uppercase()).collect(),
        }
    }
}

fn default_true() -> bool {
    true
}

}


mod compat_matcher {
use std::path::Path;

use crate::ui::state::{Step1State, Step2ComponentState};

use super::compat_model::Step2CompatRule;

pub fn match_rule(
    rule: &Step2CompatRule,
    step1: &Step1State,
    tab: &str,
    mod_name: &str,
    tp_file: &str,
    component: &Step2ComponentState,
) -> bool {
    if !rule_matches_mode(rule, step1) {
        return false;
    }
    if !rule_matches_tab(rule, tab) {
        return false;
    }
    if !rule_matches_mod(rule, mod_name, tp_file) {
        return false;
    }
    if !rule_matches_component_id(rule, component) {
        return false;
    }
    if !rule_matches_component_text(rule, component) {
        return false;
    }
    true
}

pub fn rule_disables_component(rule: &Step2CompatRule) -> bool {
    matches!(
        rule.kind.trim().to_ascii_lowercase().as_str(),
        "included" | "not_needed" | "not_compatible"
    )
}

fn rule_matches_mode(rule: &Step2CompatRule, step1: &Step1State) -> bool {
    let Some(mode) = &rule.mode else {
        return true;
    };
    let wanted = step1.game_install.to_ascii_uppercase();
    mode.normalized_items().iter().any(|m| m == &wanted)
}

fn rule_matches_tab(rule: &Step2CompatRule, tab: &str) -> bool {
    let Some(tab_value) = &rule.tab else {
        return true;
    };
    let wanted = tab.to_ascii_uppercase();
    tab_value.normalized_items().iter().any(|t| t == &wanted)
}

fn rule_matches_mod(rule: &Step2CompatRule, mod_name: &str, tp_file: &str) -> bool {
    let expected = normalize_key(rule.r#mod.as_str());
    let mod_name_norm = normalize_key(mod_name);
    let tp_file_norm = normalize_key(tp_file);
    let tp_stem = normalize_tp2_stem(tp_file);
    expected == mod_name_norm || expected == tp_file_norm || expected == tp_stem
}

fn rule_matches_component_id(rule: &Step2CompatRule, component: &Step2ComponentState) -> bool {
    let Some(component_id) = &rule.component_id else {
        return true;
    };
    component.component_id.trim() == component_id.trim()
}

fn rule_matches_component_text(rule: &Step2CompatRule, component: &Step2ComponentState) -> bool {
    let Some(component_text) = &rule.component else {
        return true;
    };
    component
        .label
        .to_ascii_lowercase()
        .contains(&component_text.to_ascii_lowercase())
}

fn normalize_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace('\\', "/")
        .trim()
        .to_string()
}

fn normalize_tp2_stem(value: &str) -> String {
    let filename = Path::new(value)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase();
    let stem = filename.strip_suffix(".tp2").unwrap_or(&filename);
    stem.strip_prefix("setup-")
        .unwrap_or(stem)
        .to_string()
}

}

mod compat_apply {
use crate::ui::state::{Step1State, Step2ModState};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::compat::model::{PathRequirementKind, Tp2Rule};
use crate::compat::tp2_parse::parse_tp2_rules;
use super::compat_matcher::{match_rule, rule_disables_component};

pub fn apply_step2_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    let rules = crate::ui::step2::service_compat_loader_step2::load_rules();
    apply_for_tab(step1, "BGEE", bgee_mods, &rules);
    apply_for_tab(step1, "BG2EE", bg2ee_mods, &rules);
}

fn apply_for_tab(
    step1: &Step1State,
    tab: &str,
    mods: &mut [Step2ModState],
    rules: &[super::compat_model::Step2CompatRule],
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
                && let Some(rule) = find_require_path_rule(meta, component_id)
                && let Some(game_dir) = effective_target_game_dir(step1, tab)
                && !path_requirement_satisfied(rule, &game_dir)
            {
                component.compat_kind = Some("path_requirement".to_string());
                component.compat_source =
                    Some(format!("step2_tp2_path_validator | TP2: {}", meta.tp_file));
                component.compat_related_mod = Some(path_requirement_summary(rule));
                component.compat_related_component = None;
                component.compat_graph = Some(format!(
                    "{} #{} requires {}",
                    normalize_mod_key(&mod_state.tp_file),
                    component_id,
                    path_requirement_summary(rule)
                ));
                component.compat_evidence = Some(path_requirement_raw_line(rule).to_string());
                component.disabled = true;
                component.checked = false;
                component.selected_order = None;
                component.disabled_reason = Some(
                    path_requirement_message(rule)
                        .unwrap_or_else(|| path_requirement_fallback_message(rule)),
                );
                continue;
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

fn find_require_path_rule(
    meta: &crate::compat::model::Tp2Metadata,
    component_id: u32,
) -> Option<&Tp2Rule> {
    meta.rules.iter().find_map(|(cid, rule)| {
        (*cid == component_id && matches!(rule, Tp2Rule::RequirePath { .. })).then_some(rule)
    })
}

fn effective_target_game_dir(step1: &Step1State, tab: &str) -> Option<String> {
    let path = match tab {
        "BGEE" => {
            if step1.game_install.eq_ignore_ascii_case("EET") {
                if step1.new_pre_eet_dir_enabled && !step1.eet_pre_dir.trim().is_empty() {
                    Some(step1.eet_pre_dir.clone())
                } else {
                    Some(step1.eet_bgee_game_folder.clone())
                }
            } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
                Some(step1.generate_directory.clone())
            } else {
                Some(step1.bgee_game_folder.clone())
            }
        }
        _ => {
            if step1.game_install.eq_ignore_ascii_case("EET") {
                if step1.new_eet_dir_enabled && !step1.eet_new_dir.trim().is_empty() {
                    Some(step1.eet_new_dir.clone())
                } else {
                    Some(step1.eet_bg2ee_game_folder.clone())
                }
            } else if step1.generate_directory_enabled && !step1.generate_directory.trim().is_empty() {
                Some(step1.generate_directory.clone())
            } else {
                Some(step1.bg2ee_game_folder.clone())
            }
        }
    }?;
    let trimmed = path.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn path_requirement_satisfied(rule: &Tp2Rule, game_dir: &str) -> bool {
    let Tp2Rule::RequirePath {
        kind,
        path,
        must_exist,
        ..
    } = rule else {
        return true;
    };

    if !matches!(kind, PathRequirementKind::Directory) {
        return true;
    }

    if path.contains('%') {
        return true;
    }

    let resolved = resolve_requirement_path(game_dir, path);
    let exists = match kind {
        PathRequirementKind::Directory => resolved.is_dir(),
        PathRequirementKind::File => resolved.is_file(),
    };

    if *must_exist { exists } else { !exists }
}

fn resolve_requirement_path(game_dir: &str, path: &str) -> PathBuf {
    let normalized = path.replace('\\', "/");
    let candidate = Path::new(&normalized);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        Path::new(game_dir).join(candidate)
    }
}

fn path_requirement_summary(rule: &Tp2Rule) -> String {
    let Tp2Rule::RequirePath {
        kind,
        path,
        must_exist,
        ..
    } = rule else {
        return String::new();
    };
    let kind_label = match kind {
        PathRequirementKind::Directory => "directory",
        PathRequirementKind::File => "file",
    };
    let state = if *must_exist { "exists" } else { "missing" };
    format!("{kind_label} {path} must be {state}")
}

fn path_requirement_message(rule: &Tp2Rule) -> Option<String> {
    match rule {
        Tp2Rule::RequirePath { message, .. } => message.clone(),
        _ => None,
    }
}

fn path_requirement_fallback_message(rule: &Tp2Rule) -> String {
    let Tp2Rule::RequirePath {
        kind,
        path,
        must_exist,
        ..
    } = rule else {
        return "This component is not installable in the current setup.".to_string();
    };
    let kind_label = match kind {
        PathRequirementKind::Directory => "directory",
        PathRequirementKind::File => "file",
    };
    if *must_exist {
        format!("This component requires {kind_label} '{path}' to exist in the target game directory.")
    } else {
        format!("This component requires {kind_label} '{path}' to be absent from the target game directory.")
    }
}

fn path_requirement_raw_line(rule: &Tp2Rule) -> &str {
    match rule {
        Tp2Rule::RequirePath { raw_line, .. } => raw_line,
        _ => "",
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
        });
    }
    false
}

}
