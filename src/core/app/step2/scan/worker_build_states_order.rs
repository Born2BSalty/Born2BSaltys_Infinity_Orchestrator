// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use crate::app::scan::ScannedComponent;

use super::tp2_blocks::parse_tp2_component_blocks_in_order;
use super::tra::{load_tp2_setup_tra_map, resolve_group_token_label};

pub(super) fn reorder_components_by_tp2_order(
    components: &mut [ScannedComponent],
    tp2_path: &str,
    tp2_text: &str,
) {
    let tra_map = load_tp2_setup_tra_map(Path::new(tp2_path));
    let (order_by_designated_id, order_by_label, order_by_begin_at_id) =
        parse_tp2_component_order(tp2_text, &tra_map);
    if order_by_designated_id.is_empty()
        && order_by_label.is_empty()
        && order_by_begin_at_id.is_empty()
    {
        return;
    }

    let indexed_keys: Vec<(usize, usize)> = components
        .iter()
        .enumerate()
        .map(|(original_index, component)| {
            let sort_key = order_by_designated_id
                .get(component.component_id.trim())
                .copied()
                .or_else(|| {
                    order_by_label
                        .get(&normalize_component_order_label(&component.display))
                        .copied()
                })
                .or_else(|| {
                    order_by_begin_at_id
                        .get(component.component_id.trim())
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
    for (dst, (_, _, component)) in components.iter_mut().zip(keyed_components) {
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
) -> (
    HashMap<String, usize>,
    HashMap<String, usize>,
    HashMap<String, usize>,
) {
    let mut out_by_designated_id = HashMap::<String, usize>::new();
    let mut out_by_label = HashMap::<String, usize>::new();
    let mut out_by_begin_at_id = HashMap::<String, usize>::new();
    for (begin_index, block) in parse_tp2_component_blocks_in_order(tp2_text)
        .into_iter()
        .enumerate()
    {
        let component_id = block.component_id.trim();
        if !component_id.is_empty() {
            let target = if block.begin_at_component_id {
                &mut out_by_begin_at_id
            } else {
                &mut out_by_designated_id
            };
            target
                .entry(component_id.to_string())
                .or_insert(begin_index);
        }
        for label in build_block_order_labels(&block, tra_map) {
            out_by_label.entry(label).or_insert(begin_index);
        }
    }

    (out_by_designated_id, out_by_label, out_by_begin_at_id)
}

fn build_block_order_labels(
    block: &super::tp2_blocks::Tp2ComponentBlock,
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
    push_order_label(&mut labels, &begin_label);

    if let Some(subcomponent_key) = block.subcomponent_key.as_deref()
        && let Some(parent_label) = resolve_group_token_label(subcomponent_key, tra_map)
    {
        let combined = format!("{} -> {}", parent_label.trim(), begin_label.trim());
        push_order_label(&mut labels, &combined);
    }

    labels
}

fn push_order_label(labels: &mut Vec<String>, value: &str) {
    let normalized = normalize_component_order_label(value);
    if !normalized.is_empty() && !labels.iter().any(|existing| existing == &normalized) {
        labels.push(normalized);
    }
}

pub(super) fn parse_begin_label(line: &str, tra_map: &HashMap<String, String>) -> Option<String> {
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
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
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

pub(super) fn normalize_component_order_label(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::reorder_components_by_tp2_order;
    use crate::app::scan::ScannedComponent;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        std::env::temp_dir().join(format!("bio-{name}-{}-{nanos}", process::id()))
    }

    fn component(id: &str, display: &str) -> ScannedComponent {
        ScannedComponent {
            tp_file: Some("EpicThieving.tp2".to_string()),
            component_id: id.to_string(),
            display: display.to_string(),
            raw_line: display.to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
        }
    }

    #[test]
    fn reorder_uses_declared_language_labels_for_implicit_first_component() {
        let root = temp_test_dir("order-implicit-begin");
        let mod_dir = root.join("EpicThieving");
        let tra_dir = mod_dir.join("tra/english");
        fs::create_dir_all(&tra_dir).expect("create temp tra dir");
        let tp2_path = mod_dir.join("EpicThieving.tp2");
        let tp2_text = "LANGUAGE\n\"English\"\nENGLISH\n ~EpicThieving/tra/english/english.tra~\n\nBEGIN @2\n\nBEGIN @3 DESIGNATED 100\n";
        fs::write(&tp2_path, tp2_text).expect("write temp tp2");
        fs::write(
            tra_dir.join("english.tra"),
            "@2 = ~Epic Locks~\n@3 = ~Epic Traps~\n",
        )
        .expect("write temp tra");

        let mut components = vec![component("100", "Epic Traps"), component("0", "Epic Locks")];

        reorder_components_by_tp2_order(
            &mut components,
            tp2_path.to_string_lossy().as_ref(),
            tp2_text,
        );

        assert_eq!(components[0].component_id, "0");
        assert_eq!(components[1].component_id, "100");

        let _ = fs::remove_dir_all(root);
    }
}
