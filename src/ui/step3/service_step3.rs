// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::{CompatIssueDisplay, Step3ItemState, WizardState};

pub fn apply_row_selection(
    selected: &mut Vec<usize>,
    anchor: &mut Option<usize>,
    items: &[Step3ItemState],
    visible_indices: &[usize],
    idx: usize,
    modifiers: egui::Modifiers,
) {
    if modifiers.shift {
        selected.clear();
        let start = anchor.unwrap_or(idx);
        let start_pos = visible_indices.iter().position(|v| *v == start);
        let end_pos = visible_indices.iter().position(|v| *v == idx);
        if let (Some(a), Some(b)) = (start_pos, end_pos) {
            let (from, to) = if a <= b { (a, b) } else { (b, a) };
            let start_item = &items[start];
            let end_item = &items[idx];
            if !start_item.is_parent && !end_item.is_parent {
                if start_item.block_id == end_item.block_id {
                    for &v in &visible_indices[from..=to] {
                        if !items[v].is_parent && items[v].block_id == start_item.block_id {
                            selected.push(v);
                        }
                    }
                } else {
                    selected.push(idx);
                    *anchor = Some(idx);
                }
            } else {
                for &v in &visible_indices[from..=to] {
                    selected.push(v);
                }
            }
        } else {
            selected.push(idx);
        }
        selected.sort_unstable();
        selected.dedup();
    } else if modifiers.ctrl {
        if let Some(pos) = selected.iter().position(|v| *v == idx) {
            selected.remove(pos);
        } else {
            selected.push(idx);
            selected.sort_unstable();
            selected.dedup();
        }
        *anchor = Some(idx);
    } else {
        selected.clear();
        selected.push(idx);
        *anchor = Some(idx);
    }
}

pub fn export_step3_compat_report(issues: &[CompatIssueDisplay]) -> std::io::Result<PathBuf> {
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let out_path = out_dir.join(format!("compat_step3_{ts}.txt"));

    let mut text = String::new();
    text.push_str("Step 3 Compatibility Report\n");
    text.push_str("===========================\n\n");
    let error_count = issues.iter().filter(|i| i.is_blocking).count();
    let warning_count = issues.len().saturating_sub(error_count);
    text.push_str(&format!("Errors: {error_count} | Warnings: {warning_count}\n\n"));
    if issues.is_empty() {
        text.push_str("No compatibility issues.\n");
    } else {
        for issue in issues {
            let sev = if issue.is_blocking { "ERROR" } else { "WARN" };
            let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            text.push_str(&format!("- [{sev}] {} {affected} -> {related}\n", issue.code));
            if !issue.reason.trim().is_empty() {
                text.push_str(&format!("  reason: {}\n", issue.reason));
            }
            if !issue.source.trim().is_empty() {
                text.push_str(&format!("  source: {}\n", issue.source));
            }
            text.push_str(&format!("  graph: {}\n", issue_graph(issue)));
            if let Some(raw) = issue.raw_evidence.as_deref() && !raw.trim().is_empty() {
                text.push_str(&format!("  rule_detail: {raw}\n"));
            }
        }
    }

    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn format_issue_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}

fn parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
    let prefix = "Requires one of:";
    let body = reason.strip_prefix(prefix)?.trim();
    let parts = body
        .split(" OR ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() > 1 {
        Some(parts)
    } else {
        None
    }
}

fn is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue.reason.to_ascii_lowercase().contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}

fn issue_graph(issue: &CompatIssueDisplay) -> String {
    if is_duplicate_selection_issue(issue) {
        return format!(
            "{} appears multiple times in selection",
            format_issue_target(&issue.affected_mod, issue.affected_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = issue
            .related_mod
            .split('|')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "{} allowed on: {}",
            format_issue_target(&issue.affected_mod, issue.affected_component),
            if games.is_empty() { "N/A".to_string() } else { games }
        );
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT")
        || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        return format!(
            "{} conflicts with {}",
            format_issue_target(&issue.affected_mod, issue.affected_component),
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("INCLUDED") {
        return format!(
            "{} is included by {}",
            format_issue_target(&issue.affected_mod, issue.affected_component),
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("ORDER_BLOCK") {
        let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
        let related = format_issue_target(&issue.related_mod, issue.related_component);
        let is_require_order = issue
            .raw_evidence
            .as_deref()
            .map(|raw| raw.trim_start().to_ascii_uppercase().starts_with("REQUIRE"))
            .unwrap_or(false);
        return if is_require_order {
            format!("{affected} must be installed after {related}")
        } else {
            format!("{affected} must be installed before {related}")
        };
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = parse_or_targets_from_reason(&issue.reason) {
            return format!(
                "{} requires one of: {}",
                format_issue_target(&issue.affected_mod, issue.affected_component),
                or_targets.join(" | ")
            );
        }
        return format!(
            "{} requires {}",
            format_issue_target(&issue.affected_mod, issue.affected_component),
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return format!(
            "{} has optional patch for {}",
            format_issue_target(&issue.affected_mod, issue.affected_component),
            format_issue_target(&issue.related_mod, issue.related_component)
        );
    }
    format!(
        "{} -> {}",
        format_issue_target(&issue.affected_mod, issue.affected_component),
        format_issue_target(&issue.related_mod, issue.related_component)
    )
}

pub fn jump_to_compat_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
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

pub fn jump_to_affected_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
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

pub fn jump_to_related_issue(state: &mut WizardState, issue: &CompatIssueDisplay) -> bool {
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
    let mut exact_related: Option<usize> = None;
    let mut best_related: Option<usize> = None;

    for (idx, item) in items.iter().enumerate() {
        if item.is_parent {
            continue;
        }
        let item_tp_key = normalize_mod_key(item.tp_file.as_str());
        let item_name_key = normalize_mod_key(item.mod_name.as_str());
        let comp_id = item.component_id.parse::<u32>().ok();

        let affected_match = (side == JumpSide::Auto || side == JumpSide::Affected)
            && (item_tp_key == affected_key || item_name_key == affected_key);
        if affected_match {
            if issue.affected_component.is_none() || issue.affected_component == comp_id {
                return Some(idx);
            }
            if best_affected.is_none() {
                best_affected = Some(idx);
            }
        }

        let related_match = (side == JumpSide::Auto || side == JumpSide::Related)
            && (item_tp_key == related_key || item_name_key == related_key);
        if related_match {
            if issue.related_component.is_none() || issue.related_component == comp_id {
                if exact_related.is_none() {
                    exact_related = Some(idx);
                }
            } else if best_related.is_none() {
                best_related = Some(idx);
            }
        }
    }

    match side {
        JumpSide::Affected => best_affected,
        JumpSide::Related => exact_related.or(best_related),
        JumpSide::Auto => best_affected.or(exact_related).or(best_related),
    }
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

pub(crate) mod component_uncheck {
    pub(crate) use crate::ui::step3::service_component_uncheck_step3::*;
}

pub(crate) mod prompt_actions {
    pub(crate) use crate::ui::step3::service_prompt_actions_step3::*;
}

pub(crate) mod drag_ops {
    pub(crate) use crate::ui::step3::service_drag_ops_step3::*;
}
