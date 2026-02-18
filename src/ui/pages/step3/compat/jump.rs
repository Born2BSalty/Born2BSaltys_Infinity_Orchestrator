// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step3ItemState, WizardState};

use super::model::normalize_mod_key;

pub(super) fn jump_to_compat_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bgee_items, JumpSide::Auto) {
        jump_to_step3_index(state, "BGEE", idx);
        return true;
    }
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bg2ee_items, JumpSide::Auto) {
        jump_to_step3_index(state, "BG2EE", idx);
        return true;
    }
    false
}

pub(super) fn jump_to_affected_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bgee_items, JumpSide::Affected) {
        jump_to_step3_index(state, "BGEE", idx);
        return true;
    }
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bg2ee_items, JumpSide::Affected) {
        jump_to_step3_index(state, "BG2EE", idx);
        return true;
    }
    false
}

pub(super) fn jump_to_related_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bgee_items, JumpSide::Related) {
        jump_to_step3_index(state, "BGEE", idx);
        return true;
    }
    if let Some(idx) = find_issue_in_items(issue, &state.step3.bg2ee_items, JumpSide::Related) {
        jump_to_step3_index(state, "BG2EE", idx);
        return true;
    }
    false
}

fn jump_to_step3_index(state: &mut WizardState, tab: &str, idx: usize) {
    state.step3.active_game_tab = tab.to_string();
    state.step3.jump_to_selected_requested = true;
    if tab == "BGEE" {
        state.step3.bgee_selected.clear();
        state.step3.bgee_selected.push(idx);
        state.step3.bgee_anchor = Some(idx);
        if let Some(item) = state.step3.bgee_items.get(idx) {
            state.step3.bgee_collapsed_blocks.retain(|b| b != &item.block_id);
        }
    } else {
        state.step3.bg2ee_selected.clear();
        state.step3.bg2ee_selected.push(idx);
        state.step3.bg2ee_anchor = Some(idx);
        if let Some(item) = state.step3.bg2ee_items.get(idx) {
            state.step3.bg2ee_collapsed_blocks.retain(|b| b != &item.block_id);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JumpSide {
    Auto,
    Affected,
    Related,
}

fn find_issue_in_items(
    issue: &CompatIssueDisplay,
    items: &[Step3ItemState],
    side: JumpSide,
) -> Option<usize> {
    let affected_key = normalize_mod_key(issue.affected_mod.as_str());
    let related_key = normalize_mod_key(issue.related_mod.as_str());

    let mut best_affected: Option<usize> = None;
    let mut best_related: Option<usize> = None;

    for (idx, item) in items.iter().enumerate() {
        if item.is_parent {
            continue;
        }
        let item_tp_key = normalize_mod_key(item.tp_file.as_str());
        let item_name_key = normalize_mod_key(item.mod_name.as_str());
        let comp_id = item.component_id.parse::<u32>().ok();

        let affected_match =
            (side == JumpSide::Auto || side == JumpSide::Affected)
                && (item_tp_key == affected_key || item_name_key == affected_key);
        if affected_match {
            if issue.affected_component.is_none() || issue.affected_component == comp_id {
                return Some(idx);
            }
            if best_affected.is_none() {
                best_affected = Some(idx);
            }
        }

        let related_match =
            (side == JumpSide::Auto || side == JumpSide::Related)
                && (item_tp_key == related_key || item_name_key == related_key);
        if related_match {
            if issue.related_component.is_none() || issue.related_component == comp_id {
                if best_related.is_none() {
                    best_related = Some(idx);
                }
            } else if best_related.is_none() {
                best_related = Some(idx);
            }
        }
    }

    match side {
        JumpSide::Affected => best_affected,
        JumpSide::Related => best_related,
        JumpSide::Auto => best_affected.or(best_related),
    }
}
