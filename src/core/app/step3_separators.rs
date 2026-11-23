// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::game_authority::GameSlot;
use crate::app::state::Step3ItemState;
use crate::app::step3_history::{self, Step3HistoryEntry, Step3TouchedRows};

pub const SEPARATOR_COMPONENT_ID: &str = "__SEPARATOR__";
pub const SEPARATOR_BLOCK_PREFIX: &str = "SEPARATOR::";
pub const DEFAULT_SEPARATOR_NAME: &str = "New separator";
pub const SEPARATOR_NAME_MAX_CHARS: usize = 60;

const LOG_LINE_HASHES: &str = "######";
const MIN_LOG_LINE_HASHES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparatorAnchor {
    pub name: String,
    pub block_id: Option<String>,
    pub collapsed: bool,
    pub following: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparatorRename {
    pub tab: GameSlot,
    pub block_id: String,
    pub text: String,
    pub focus_pending: bool,
}

const fn is_name_edge(c: char) -> bool {
    c == '#' || c.is_whitespace()
}

#[must_use]
pub fn sanitize_separator_name(raw: &str) -> Option<String> {
    let one_line: String = raw
        .chars()
        .filter(|c| *c != '@')
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let trimmed = one_line.trim_matches(is_name_edge);
    let capped: String = trimmed.chars().take(SEPARATOR_NAME_MAX_CHARS).collect();
    let cleaned = capped.trim_matches(is_name_edge);
    (!cleaned.is_empty()).then(|| cleaned.to_string())
}

#[must_use]
pub fn separator_row(name: &str, block_id: String) -> Step3ItemState {
    Step3ItemState {
        tp_file: String::new(),
        component_id: SEPARATOR_COMPONENT_ID.to_string(),
        mod_name: name.to_string(),
        component_label: name.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: usize::MAX,
        block_id,
        is_parent: true,
        parent_placeholder: false,
    }
}

fn separator_seq(block_id: &str) -> Option<usize> {
    block_id
        .strip_prefix(SEPARATOR_BLOCK_PREFIX)
        .and_then(|seq| seq.parse::<usize>().ok())
}

fn largest_separator_seq(items: &[Step3ItemState]) -> usize {
    items
        .iter()
        .filter(|item| item.is_separator())
        .filter_map(|item| separator_seq(&item.block_id))
        .max()
        .unwrap_or(0)
}

#[must_use]
pub fn next_separator_block_id(items: &[Step3ItemState]) -> String {
    format!(
        "{SEPARATOR_BLOCK_PREFIX}{}",
        largest_separator_seq(items) + 1
    )
}

#[must_use]
pub fn section_indices(items: &[Step3ItemState], separator_idx: usize) -> Vec<usize> {
    if separator_idx >= items.len() {
        return Vec::new();
    }
    let end = items
        .iter()
        .enumerate()
        .skip(separator_idx + 1)
        .find(|(_, item)| item.is_separator())
        .map_or(items.len(), |(idx, _)| idx);
    (separator_idx..end).collect()
}

pub fn move_section(
    items: &mut Vec<Step3ItemState>,
    separator_idx: usize,
    before: Option<usize>,
) -> bool {
    let section = section_indices(items, separator_idx);
    let (Some(&start), Some(&end)) = (section.first(), section.last()) else {
        return false;
    };
    let target = before.unwrap_or(items.len());
    let lands_on_a_separator =
        before.is_none_or(|idx| items.get(idx).is_some_and(Step3ItemState::is_separator));
    if !lands_on_a_separator || (start..=end + 1).contains(&target) {
        return false;
    }
    let moving: Vec<Step3ItemState> = items.drain(start..=end).collect();
    let insert_at = if target > end {
        target - moving.len()
    } else {
        target
    };
    items.splice(insert_at..insert_at, moving);
    true
}

#[must_use]
pub fn first_separator(items: &[Step3ItemState]) -> Option<usize> {
    items.iter().position(Step3ItemState::is_separator)
}

#[must_use]
pub fn enclosing_separator(items: &[Step3ItemState], idx: usize) -> Option<usize> {
    items
        .iter()
        .take(idx.saturating_add(1))
        .rposition(Step3ItemState::is_separator)
}

fn component_pair(item: &Step3ItemState) -> (String, String) {
    (item.tp_file.to_ascii_uppercase(), item.component_id.clone())
}

fn matches_pair(item: &Step3ItemState, pair: &(String, String)) -> bool {
    !item.is_parent && item.component_id == pair.1 && item.tp_file.eq_ignore_ascii_case(&pair.0)
}

#[must_use]
pub fn capture_separator_anchors(
    items: &[Step3ItemState],
    collapsed_blocks: &[String],
) -> Vec<SeparatorAnchor> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.is_separator())
        .map(|(idx, item)| SeparatorAnchor {
            name: item.mod_name.clone(),
            block_id: Some(item.block_id.clone()),
            collapsed: collapsed_blocks.contains(&item.block_id),
            following: items[idx + 1..]
                .iter()
                .filter(|row| !row.is_parent)
                .map(component_pair)
                .collect(),
        })
        .collect()
}

#[must_use]
pub fn resolve_separator_block_ids(
    children: &[Step3ItemState],
    anchors: &[SeparatorAnchor],
) -> Vec<SeparatorAnchor> {
    let mut next_seq = anchors
        .iter()
        .filter_map(|anchor| anchor.block_id.as_deref().and_then(separator_seq))
        .max()
        .unwrap_or(0)
        .max(largest_separator_seq(children))
        + 1;
    anchors
        .iter()
        .map(|anchor| {
            let mut resolved = anchor.clone();
            if resolved.block_id.is_none() {
                resolved.block_id = Some(format!("{SEPARATOR_BLOCK_PREFIX}{next_seq}"));
                next_seq += 1;
            }
            resolved
        })
        .collect()
}

fn anchor_position(children: &[Step3ItemState], anchor: &SeparatorAnchor) -> usize {
    anchor
        .following
        .iter()
        .find_map(|pair| children.iter().position(|child| matches_pair(child, pair)))
        .unwrap_or(children.len())
}

pub fn insert_separators(children: &mut Vec<Step3ItemState>, anchors: &[SeparatorAnchor]) {
    if anchors.is_empty() {
        return;
    }
    let resolved = resolve_separator_block_ids(children, anchors);
    let positions: Vec<usize> = resolved
        .iter()
        .map(|anchor| anchor_position(children, anchor))
        .collect();
    let separators_at = |at: usize| {
        resolved
            .iter()
            .zip(&positions)
            .filter(move |(_, position)| **position == at)
            .map(|(anchor, _)| {
                separator_row(&anchor.name, anchor.block_id.clone().unwrap_or_default())
            })
    };
    let original = std::mem::take(children);
    let original_len = original.len();
    let mut out = Vec::with_capacity(original_len + resolved.len());
    for (idx, child) in original.into_iter().enumerate() {
        out.extend(separators_at(idx));
        out.push(child);
    }
    out.extend(separators_at(original_len));
    *children = out;
}

#[must_use]
pub fn format_separator_log_line(name: &str) -> String {
    format!("// {LOG_LINE_HASHES} {name} {LOG_LINE_HASHES}")
}

#[must_use]
pub fn parse_separator_log_line(line: &str) -> Option<String> {
    let body = line.trim().strip_prefix("//")?.trim_start();
    let after_open = body.trim_start_matches('#');
    if body.len() - after_open.len() < MIN_LOG_LINE_HASHES {
        return None;
    }
    let inner = after_open.trim_end_matches('#');
    if after_open.len() - inner.len() < MIN_LOG_LINE_HASHES {
        return None;
    }
    let name = inner.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn block_start(items: &[Step3ItemState], idx: usize) -> usize {
    let item = &items[idx];
    if item.is_parent {
        return idx;
    }
    items[..idx]
        .iter()
        .rposition(|row| row.is_mod_header() && row.block_id == item.block_id)
        .unwrap_or(idx)
}

pub fn add_separator(
    items: &mut Vec<Step3ItemState>,
    above_idx: Option<usize>,
    undo: &mut Vec<Step3HistoryEntry>,
    redo: &mut Vec<Step3HistoryEntry>,
) -> String {
    step3_history::push_undo_snapshot(items, Step3TouchedRows::default(), undo, redo);
    let at = above_idx
        .filter(|idx| *idx < items.len())
        .map_or(items.len(), |idx| block_start(items, idx));
    let block_id = next_separator_block_id(items);
    items.insert(at, separator_row(DEFAULT_SEPARATOR_NAME, block_id.clone()));
    block_id
}

#[cfg(test)]
#[path = "step3_separators_tests.rs"]
mod tests;
