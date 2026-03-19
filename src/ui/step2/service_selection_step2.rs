// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{CompatIssueDisplay, Step2Selection, WizardState};

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

pub fn selection_selected_mod_key(tp_file: &str, component_key: &str) -> String {
    if !component_key.trim().is_empty()
        && let Some(tp2) = parse_component_tp2_from_raw(component_key)
    {
        return selection_normalize_mod_key(&tp2);
    }
    selection_normalize_mod_key(tp_file)
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
                c.component_id.trim().parse::<u32>().ok() == Some(target_component)
                    && c_key == target_key
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

pub fn current_game_tab(state: &WizardState) -> Option<String> {
    let Some(Step2Selection::Component { game_tab, .. }) = state.step2.selected.clone() else {
        return None;
    };
    Some(game_tab)
}

pub fn issue_targets_for_current_selection(
    state: &WizardState,
) -> Option<(String, Option<u32>, String, Option<u32>)> {
    let Some(Step2Selection::Component {
        tp_file,
        component_id,
        component_key,
        ..
    }) = state.step2.selected.clone()
    else {
        return None;
    };
    let this_mod_key = selection_selected_mod_key(&tp_file, &component_key);
    let this_component = component_id.parse::<u32>().ok();
    state
        .compat
        .issues
        .iter()
        .find(|issue| {
            selection_normalize_mod_key(&issue.affected_mod) == this_mod_key
                && match (issue.affected_component, this_component) {
                    (Some(a), Some(b)) => a == b,
                    (None, _) => true,
                    _ => false,
                }
        })
        .or_else(|| {
            state.compat.issues.iter().find(|issue| {
                selection_normalize_mod_key(&issue.related_mod) == this_mod_key
                    && match (issue.related_component, this_component) {
                        (Some(a), Some(b)) => a == b,
                        (None, _) => true,
                        _ => false,
                    }
            })
        })
        .map(|issue| {
            (
                issue.related_mod.clone(),
                issue.related_component,
                issue.affected_mod.clone(),
                issue.affected_component,
            )
        })
}

pub fn current_issue_id_for_selection(state: &WizardState) -> Option<String> {
    let Some(Step2Selection::Component {
        tp_file,
        component_id,
        component_key,
        ..
    }) = state.step2.selected.clone()
    else {
        return None;
    };
    let this_mod_key = selection_selected_mod_key(&tp_file, &component_key);
    let this_component = component_id.parse::<u32>().ok();
    state
        .compat
        .issues
        .iter()
        .find(|issue| {
            selection_normalize_mod_key(&issue.affected_mod) == this_mod_key
                && match (issue.affected_component, this_component) {
                    (Some(a), Some(b)) => a == b,
                    (None, _) => true,
                    _ => false,
                }
        })
        .or_else(|| {
            state.compat.issues.iter().find(|issue| {
                selection_normalize_mod_key(&issue.related_mod) == this_mod_key
                    && match (issue.related_component, this_component) {
                        (Some(a), Some(b)) => a == b,
                        (None, _) => true,
                        _ => false,
                    }
            })
        })
        .map(|i| i.issue_id.clone())
}

pub fn current_issue_for_selection(state: &WizardState) -> Option<CompatIssueDisplay> {
    let issue_id = current_issue_id_for_selection(state)?;
    state
        .compat
        .issues
        .iter()
        .find(|i| i.issue_id == issue_id)
        .cloned()
}

pub fn rule_source_open_path(state: &WizardState) -> Option<String> {
    let issue = current_issue_for_selection(state)?;
    let src = issue.source.trim();
    if let Some(tp2_token) = src.strip_prefix("TP2:") {
        let target_mod_key = selection_normalize_mod_key(&issue.affected_mod);
        for mod_state in state
            .step2
            .bgee_mods
            .iter()
            .chain(state.step2.bg2ee_mods.iter())
        {
            if selection_normalize_mod_key(&mod_state.tp_file) == target_mod_key
                && !mod_state.tp2_path.trim().is_empty()
            {
                return Some(mod_state.tp2_path.clone());
            }
        }
        let fallback = tp2_token.trim();
        if !fallback.is_empty() {
            return Some(fallback.to_string());
        }
        return None;
    }

    let mut path = src;
    if let Some((lhs, rhs)) = src.rsplit_once(':')
        && rhs.trim().chars().all(|c| c.is_ascii_digit())
    {
        path = lhs;
    }
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}
pub use crate::ui::step2::service_details_step2::selected_details;
