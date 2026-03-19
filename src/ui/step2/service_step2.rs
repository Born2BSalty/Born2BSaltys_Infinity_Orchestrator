// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step2ModState;
use crate::ui::state::{CompatState, Step1State, Step2State, WizardState};
use std::io;
use std::path::PathBuf;

pub fn mod_matches_filter(mod_state: &Step2ModState, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    if mod_state.name.to_lowercase().contains(filter) {
        return true;
    }
    mod_state
        .components
        .iter()
        .any(|component| component.label.to_lowercase().contains(filter))
}

pub fn clear_all(mods: &mut [Step2ModState], next_selection_order: &mut usize) {
    for mod_state in mods {
        mod_state.checked = false;
        for component in &mut mod_state.components {
            component.checked = false;
            component.selected_order = None;
        }
    }
    *next_selection_order = 1;
}

pub fn select_visible(mods: &mut [Step2ModState], filter: &str, next_selection_order: &mut usize) {
    for mod_state in mods {
        if !mod_matches_filter(mod_state, filter) {
            continue;
        }
        let mod_name_match = filter.is_empty() || mod_state.name.to_lowercase().contains(filter);
        for component in &mut mod_state.components {
            let is_visible = mod_name_match || component.label.to_lowercase().contains(filter);
            if !is_visible || component.disabled {
                continue;
            }
            component.checked = true;
            if component.selected_order.is_none() {
                component.selected_order = Some(*next_selection_order);
                *next_selection_order += 1;
            }
        }
        enforce_meta_mode_after_bulk(mod_state);
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

fn enforce_meta_mode_after_bulk(mod_state: &mut Step2ModState) {
    let any_normal_checked = mod_state
        .components
        .iter()
        .any(|c| c.checked && !c.disabled && !c.is_meta_mode_component);
    if any_normal_checked {
        for comp in &mut mod_state.components {
            if comp.is_meta_mode_component {
                comp.checked = false;
                comp.selected_order = None;
            }
        }
    }
}

pub fn create_default_compat_rules_file() -> io::Result<PathBuf> {
    crate::ui::step2::service_compat_rules_step2::create_default_compat_rules_file()
}

pub fn apply_compat_rules(
    step1: &Step1State,
    bgee_mods: &mut [Step2ModState],
    bg2ee_mods: &mut [Step2ModState],
) {
    crate::ui::step2::service_compat_rules_step2::apply_compat_rules(step1, bgee_mods, bg2ee_mods);
}

pub fn export_compat_report(step2: &Step2State, compat: &CompatState) -> io::Result<PathBuf> {
    crate::ui::step2::service_compat_rules_step2::export_compat_report(step2, compat)
}

pub fn parse_lang(raw_line: &str) -> Option<String> {
    raw_line
        .split_whitespace()
        .find(|p| p.starts_with('#'))
        .map(|p| p.trim_start_matches('#').to_string())
        .filter(|s| !s.is_empty())
}

pub fn parse_version(raw_line: &str) -> Option<String> {
    let comment = raw_line.split_once("//")?.1.trim();
    let (_, v) = comment.rsplit_once(':')?;
    let version = v.trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

pub fn recompute_selection_counts(state: &mut WizardState) {
    let mods = if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mut total = 0usize;
    let mut selected = 0usize;
    for mod_state in mods {
        for component in &mod_state.components {
            total += 1;
            if component.checked {
                selected += 1;
            }
        }
    }
    state.step2.total_count = total;
    state.step2.selected_count = selected;
}

pub use crate::ui::step2::service_selection_step2::{
    current_game_tab, current_issue_for_selection, current_issue_id_for_selection,
    issue_targets_for_current_selection, jump_to_target, rule_source_open_path, selected_details,
};
