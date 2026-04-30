// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::app::scan::ScannedComponent;

use super::super::order::{normalize_component_order_label, parse_begin_label};
use super::super::tp2_blocks::{Tp2ComponentBlock, parse_tp2_component_blocks_in_order};
use super::super::tra::{load_tp2_setup_tra_map, resolve_group_token_label};
use super::block_is_deprecated_placeholder;

pub(super) fn detect_weidu_groups(
    tp2_path: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, String> {
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if ordered_blocks.len() < 2 {
        return HashMap::new();
    }

    let tra_map = load_tp2_setup_tra_map(Path::new(tp2_path));
    let component_ids = components
        .iter()
        .map(|component| component.component_id.trim().to_string())
        .collect::<HashSet<_>>();
    let ordered_components = components
        .iter()
        .map(|component| {
            (
                component.component_id.trim().to_string(),
                normalize_component_order_label(&component.display),
            )
        })
        .collect::<Vec<_>>();

    let mut out = HashMap::<String, String>::new();
    let mut distinct = HashSet::<String>::new();
    let mut matched_components = HashSet::<String>::new();
    let mut component_cursor = 0usize;
    let mut previous_group = None::<String>;
    for (index, block) in ordered_blocks.iter().enumerate() {
        let Some(bound_component_id) = bind_block_to_component_id(
            block,
            &tra_map,
            &component_ids,
            &ordered_components,
            &matched_components,
            &mut component_cursor,
        ) else {
            continue;
        };
        matched_components.insert(bound_component_id.clone());
        let group_label = block
            .group_key
            .as_deref()
            .and_then(|group_token| resolve_group_token_label(group_token, &tra_map))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                inherit_group_for_deprecated_placeholder(
                    &ordered_blocks,
                    index,
                    &tra_map,
                    previous_group.as_deref(),
                )
            });
        let Some(group_label) = group_label else {
            continue;
        };
        previous_group = Some(group_label.clone());
        out.insert(bound_component_id, group_label.clone());
        distinct.insert(group_label.to_ascii_lowercase());
    }

    if distinct.len() < 2 {
        HashMap::new()
    } else {
        out
    }
}

fn bind_block_to_component_id(
    block: &Tp2ComponentBlock,
    tra_map: &HashMap<String, String>,
    component_ids: &HashSet<String>,
    ordered_components: &[(String, String)],
    matched_components: &HashSet<String>,
    component_cursor: &mut usize,
) -> Option<String> {
    let block_component_id = block.component_id.trim();
    if !block_component_id.is_empty()
        && component_ids.contains(block_component_id)
        && !matched_components.contains(block_component_id)
    {
        if let Some(index) = ordered_components
            .iter()
            .position(|(component_id, _)| component_id == block_component_id)
        {
            *component_cursor = index.saturating_add(1);
        }
        return Some(block_component_id.to_string());
    }

    let block_labels = build_block_match_labels(block, tra_map);
    if block_labels.is_empty() {
        return None;
    }

    for (index, (component_id, component_label)) in ordered_components
        .iter()
        .enumerate()
        .skip(*component_cursor)
    {
        if matched_components.contains(component_id) {
            continue;
        }
        if block_labels.iter().any(|label| component_label == label) {
            *component_cursor = index.saturating_add(1);
            return Some(component_id.clone());
        }
    }

    for (index, (component_id, component_label)) in ordered_components.iter().enumerate() {
        if matched_components.contains(component_id) {
            continue;
        }
        if block_labels.iter().any(|label| component_label == label) {
            *component_cursor = index.saturating_add(1);
            return Some(component_id.clone());
        }
    }

    None
}

fn build_block_match_labels(
    block: &Tp2ComponentBlock,
    tra_map: &HashMap<String, String>,
) -> Vec<String> {
    let Some(begin_label) = block
        .body_lines
        .first()
        .and_then(|line| parse_begin_label(line, tra_map))
    else {
        return Vec::new();
    };

    let mut labels = Vec::<String>::new();
    push_match_label(&mut labels, &begin_label);

    if let Some(subcomponent_key) = block.subcomponent_key.as_deref()
        && let Some(parent_label) = resolve_group_token_label(subcomponent_key, tra_map)
    {
        let combined = format!("{} -> {}", parent_label.trim(), begin_label.trim());
        push_match_label(&mut labels, &combined);
    }

    labels
}

fn push_match_label(labels: &mut Vec<String>, value: &str) {
    let normalized = normalize_component_order_label(value);
    if !normalized.is_empty() && !labels.iter().any(|existing| existing == &normalized) {
        labels.push(normalized);
    }
}

fn inherit_group_for_deprecated_placeholder(
    ordered_blocks: &[Tp2ComponentBlock],
    index: usize,
    tra_map: &HashMap<String, String>,
    previous_group: Option<&str>,
) -> Option<String> {
    let previous_group = previous_group?.trim();
    if previous_group.is_empty() || !block_is_deprecated_placeholder(&ordered_blocks[index]) {
        return None;
    }

    let next_group = ordered_blocks.iter().skip(index + 1).find_map(|block| {
        if block_is_deprecated_placeholder(block) && block.group_key.is_none() {
            return None;
        }
        block
            .group_key
            .as_deref()
            .and_then(|group_token| resolve_group_token_label(group_token, tra_map))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })?;

    next_group
        .eq_ignore_ascii_case(previous_group)
        .then(|| previous_group.to_string())
}
