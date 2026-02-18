// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step2Selection, WizardState};

use super::key::{normalize_mod_key, selected_mod_key};

pub(crate) fn current_game_tab(state: &WizardState) -> Option<String> {
    let Some(Step2Selection::Component { game_tab, .. }) = state.step2.selected.clone() else {
        return None;
    };
    Some(game_tab)
}

pub(crate) fn issue_targets_for_current_selection(
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
    let this_mod_key = selected_mod_key(&tp_file, &component_key);
    let this_component = component_id.parse::<u32>().ok();
    state
        .compat
        .issues
        .iter()
        .find(|issue| {
            normalize_mod_key(&issue.affected_mod) == this_mod_key
                && match (issue.affected_component, this_component) {
                    (Some(a), Some(b)) => a == b,
                    (None, _) => true,
                    _ => false,
                }
        })
        .or_else(|| {
            state.compat.issues.iter().find(|issue| {
                normalize_mod_key(&issue.related_mod) == this_mod_key
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

pub(crate) fn current_issue_id_for_selection(state: &WizardState) -> Option<String> {
    let Some(Step2Selection::Component {
        tp_file,
        component_id,
        component_key,
        ..
    }) = state.step2.selected.clone()
    else {
        return None;
    };
    let this_mod_key = selected_mod_key(&tp_file, &component_key);
    let this_component = component_id.parse::<u32>().ok();
    state
        .compat
        .issues
        .iter()
        .find(|issue| {
            normalize_mod_key(&issue.affected_mod) == this_mod_key
                && match (issue.affected_component, this_component) {
                    (Some(a), Some(b)) => a == b,
                    (None, _) => true,
                    _ => false,
                }
        })
        .or_else(|| {
            state.compat.issues.iter().find(|issue| {
                normalize_mod_key(&issue.related_mod) == this_mod_key
                    && match (issue.related_component, this_component) {
                        (Some(a), Some(b)) => a == b,
                        (None, _) => true,
                        _ => false,
                    }
            })
        })
        .map(|i| i.issue_id.clone())
}

pub(crate) fn current_issue_for_selection(state: &WizardState) -> Option<CompatIssueDisplay> {
    let issue_id = current_issue_id_for_selection(state)?;
    state
        .compat
        .issues
        .iter()
        .find(|i| i.issue_id == issue_id)
        .cloned()
}
