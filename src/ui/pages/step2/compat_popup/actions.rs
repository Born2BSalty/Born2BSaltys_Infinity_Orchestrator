// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

use super::filters;
use super::selection;

pub(super) fn jump_to_related_component(state: &mut WizardState) {
    let Some((related_mod, related_component, _, _)) =
        selection::issue_targets_for_current_selection(state)
    else {
        return;
    };
    let Some(game_tab) = selection::current_game_tab(state) else {
        return;
    };
    selection::jump_to_target(state, &game_tab, &related_mod, related_component);
}

pub(super) fn jump_to_affected_component(state: &mut WizardState) {
    let Some((_, _, affected_mod, affected_component)) =
        selection::issue_targets_for_current_selection(state)
    else {
        return;
    };
    let Some(game_tab) = selection::current_game_tab(state) else {
        return;
    };
    selection::jump_to_target(state, &game_tab, &affected_mod, affected_component);
}

pub(super) fn jump_to_next_conflict(state: &mut WizardState) {
    let Some(game_tab) = selection::current_game_tab(state) else {
        return;
    };
    let filter = state.step2.compat_popup_filter.clone();
    let issue_list: Vec<(String, String, Option<u32>)> = state
        .compat
        .issues
        .iter()
        .filter(|i| filters::matches_issue_filter(i, &filter))
        .map(|i| (i.issue_id.clone(), i.affected_mod.clone(), i.affected_component))
        .collect();
    if issue_list.is_empty() {
        return;
    }
    let current_issue_id = selection::current_issue_id_for_selection(state);
    let start = current_issue_id
        .as_ref()
        .and_then(|id| issue_list.iter().position(|i| &i.0 == id))
        .map(|idx| (idx + 1) % issue_list.len())
        .unwrap_or(0);
    let issue = &issue_list[start];
    selection::jump_to_target(state, &game_tab, &issue.1, issue.2);
}
