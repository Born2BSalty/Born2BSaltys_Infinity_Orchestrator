// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use crate::ui::scan::ScannedComponent;

use super::tp2_blocks::parse_designated_id;
use super::tra::{load_tp2_setup_tra_map, resolve_group_token_label};

pub(super) fn reorder_components_by_tp2_order(
    components: &mut [ScannedComponent],
    tp2_path: &str,
    tp2_text: &str,
) {
    let tra_map = load_tp2_setup_tra_map(Path::new(tp2_path));
    let (order_by_id, order_by_label) = parse_tp2_component_order(tp2_text, &tra_map);
    if order_by_id.is_empty() && order_by_label.is_empty() {
        return;
    }

    let indexed_keys: Vec<(usize, usize)> = components
        .iter()
        .enumerate()
        .map(|(original_index, component)| {
            let sort_key = order_by_id
                .get(component.component_id.trim())
                .copied()
                .or_else(|| {
                    order_by_label
                        .get(&normalize_component_order_label(&component.display))
                        .copied()
                })
                .unwrap_or(usize::MAX);
            (original_index, sort_key)
        })
        .collect();

    let matched = indexed_keys
        .iter()
        .filter(|(_, sort_key)| *sort_key != usize::MAX)
        .count();
    let total = components.len();
    if matched == 0 {
        return;
    }
    let coverage_ok = matched == total || matched * 2 >= total;
    if !coverage_ok {
        return;
    }

    let mut keyed_components: Vec<(usize, usize, ScannedComponent)> = components
        .iter()
        .cloned()
        .enumerate()
        .map(|(original_index, component)| {
            let sort_key = indexed_keys[original_index].1;
            (sort_key, original_index, component)
        })
        .collect();
    keyed_components.sort_by_key(|(sort_key, original_index, _)| (*sort_key, *original_index));
    for (dst, (_, _, component)) in components.iter_mut().zip(keyed_components.into_iter()) {
        *dst = component;
    }
}

pub(super) fn display_is_blank_version_only(value: &str) -> bool {
    let trimmed = value.trim();
    let Some((head, tail)) = trimmed.rsplit_once(':') else {
        return false;
    };
    if !head.trim().is_empty() {
        return false;
    }
    let versionish = tail.trim();
    let normalized = versionish.strip_prefix('v').unwrap_or(versionish);
    !normalized.is_empty()
        && normalized
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

fn parse_tp2_component_order(
    tp2_text: &str,
    tra_map: &HashMap<String, String>,
) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let mut out_by_id = HashMap::<String, usize>::new();
    let mut out_by_label = HashMap::<String, usize>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    let mut begin_index = 0usize;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        if !trimmed.to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }

        let mut end = index + 1;
        while end < lines.len() {
            let next = lines[end].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            end += 1;
        }

        let block = &lines[index..end];
        if let Some(label) = parse_begin_label(trimmed, tra_map) {
            out_by_label
                .entry(normalize_component_order_label(&label))
                .or_insert(begin_index);
        }
        for line in block {
            if let Some(id) = parse_designated_id(&line.to_ascii_uppercase()) {
                out_by_id.entry(id).or_insert(begin_index);
                break;
            }
        }

        begin_index += 1;
        index = end;
    }

    (out_by_id, out_by_label)
}

fn parse_begin_label(line: &str, tra_map: &HashMap<String, String>) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("BEGIN ") {
        return None;
    }
    let tail = trimmed["BEGIN".len()..].trim_start();
    if let Some(rest) = tail.strip_prefix('@') {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        let token = format!("@{digits}");
        return resolve_group_token_label(&token, tra_map);
    }
    let quote = tail.chars().next()?;
    if quote != '~' && quote != '"' {
        return None;
    }
    let rest = &tail[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn normalize_component_order_label(value: &str) -> String {
    let trimmed = value.trim();
    let base = trimmed
        .rsplit_once(':')
        .and_then(|(head, tail)| {
            let versionish = tail.trim();
            let normalized = versionish.strip_prefix('v').unwrap_or(versionish);
            (!normalized.is_empty()
                && normalized
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '.' || c == '_' || c == '-'))
            .then_some(head.trim())
        })
        .unwrap_or(trimmed);
    base.to_ascii_lowercase()
}
