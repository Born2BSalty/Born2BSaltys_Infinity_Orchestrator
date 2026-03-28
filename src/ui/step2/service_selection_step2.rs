// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{Step2Selection, WizardState};

pub fn selection_normalize_mod_key(value: &str) -> String {
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

pub fn jump_to_target(
    state: &mut WizardState,
    game_tab: &str,
    mod_ref: &str,
    component_ref: Option<u32>,
) {
    let target_key = selection_normalize_mod_key(mod_ref);
    let mods = if game_tab.eq_ignore_ascii_case("BGEE") {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    for mod_state in mods {
        if let Some(target_component) = component_ref {
            if let Some(component) = mod_state.components.iter().find(|c| {
                let c_key = parse_component_tp2_from_raw(&c.raw_line)
                    .map(|tp2| selection_normalize_mod_key(&tp2))
                    .unwrap_or_else(|| selection_normalize_mod_key(&mod_state.tp_file));
                c.component_id.trim().parse::<u32>().ok() == Some(target_component) && c_key == target_key
            }) {
                state.step2.selected = Some(Step2Selection::Component {
                    game_tab: game_tab.to_string(),
                    tp_file: mod_state.tp_file.clone(),
                    component_id: component.component_id.clone(),
                    component_key: component.raw_line.clone(),
                });
                return;
            }
        } else if let Some(component) = mod_state.components.iter().find(|c| {
            parse_component_tp2_from_raw(&c.raw_line)
                .map(|tp2| selection_normalize_mod_key(&tp2))
                .unwrap_or_else(|| selection_normalize_mod_key(&mod_state.tp_file))
                == target_key
        }) {
            state.step2.selected = Some(Step2Selection::Component {
                game_tab: game_tab.to_string(),
                tp_file: mod_state.tp_file.clone(),
                component_id: component.component_id.clone(),
                component_key: component.raw_line.clone(),
            });
            return;
        }
        if selection_normalize_mod_key(&mod_state.tp_file) != target_key {
            continue;
        }
        state.step2.selected = Some(Step2Selection::Mod {
            game_tab: game_tab.to_string(),
            tp_file: mod_state.tp_file.clone(),
        });
        return;
    }
}

pub fn rule_source_open_path(state: &WizardState) -> Option<String> {
    if let Some(issue) = state.step2.compat_popup_issue_override.as_ref() {
        return extract_source_path(issue.source.as_str());
    }
    extract_source_path(
        selected_details(state)
            .compat_source
            .as_deref()
            .unwrap_or_default(),
    )
}

fn extract_source_path(src: &str) -> Option<String> {
    let mut path = src.trim();
    if let Some((lhs, rhs)) = path.rsplit_once(':')
        && rhs.trim().chars().all(|c| c.is_ascii_digit())
    {
        path = lhs.trim();
    }
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}
pub use crate::ui::step2::service_details_step2::selected_details;
