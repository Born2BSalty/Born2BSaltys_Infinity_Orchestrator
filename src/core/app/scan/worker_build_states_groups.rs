// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ui::scan::ScannedComponent;

use super::order::{normalize_component_order_label, parse_begin_label};
use super::tp2_blocks::{parse_tp2_component_blocks_in_order, split_subcomponent_display_label};
use super::tra::{load_tp2_setup_tra_map, resolve_group_token_label};

#[derive(Debug, Clone)]
pub(super) struct DerivedCollapsibleGroup {
    pub header: String,
    pub is_umbrella: bool,
}

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
            .or_else(|| inherit_group_for_deprecated_placeholder(
                &ordered_blocks,
                index,
                &tra_map,
                previous_group.as_deref(),
            ));
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
    block: &super::tp2_blocks::Tp2ComponentBlock,
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

pub(super) fn detect_derived_collapsible_groups(
    tp_file: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, DerivedCollapsibleGroup> {
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if ordered_blocks.len() < 3 {
        return HashMap::new();
    }

    let deprecated_placeholder_ids = ordered_blocks
        .iter()
        .filter(|block| block_is_deprecated_placeholder(block))
        .map(|block| block.component_id.trim().to_string())
        .collect::<HashSet<_>>();

    let display_by_id: HashMap<String, String> = components
        .iter()
        .map(|component| {
            (
                component.component_id.trim().to_string(),
                component.display.trim().to_string(),
            )
        })
        .collect();

    let mut same_mod_installed_guards = HashMap::<String, String>::new();
    for block in &ordered_blocks {
        let mut targets = block
            .body_lines
            .iter()
            .filter_map(|line| parse_same_mod_installed_guard_target(tp_file, line))
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        if targets.len() == 1 {
            same_mod_installed_guards.insert(block.component_id.clone(), targets[0].clone());
        }
    }

    let mut out = HashMap::<String, DerivedCollapsibleGroup>::new();
    let mut index = 0usize;
    while index < ordered_blocks.len() {
        let umbrella = &ordered_blocks[index];
        let umbrella_id = umbrella.component_id.trim();
        let Some(umbrella_display) = display_by_id.get(umbrella_id) else {
            index += 1;
            continue;
        };
        if split_subcomponent_display_label(umbrella_display).is_some() {
            index += 1;
            continue;
        }

        let mut child_ids = Vec::<String>::new();
        let mut next_index = index + 1;
        while next_index < ordered_blocks.len() {
            let child = &ordered_blocks[next_index];
            if same_mod_installed_guards
                .get(child.component_id.trim())
                .is_some_and(|target| target == umbrella_id)
            {
                child_ids.push(child.component_id.clone());
                next_index += 1;
            } else {
                break;
            }
        }

        if child_ids.len() >= 2 {
            let header = derive_collapsible_group_header(umbrella_display);
            out.insert(
                umbrella_id.to_string(),
                DerivedCollapsibleGroup {
                    header: header.clone(),
                    is_umbrella: true,
                },
            );
            for child_id in child_ids {
                if display_by_id.contains_key(child_id.trim()) {
                    out.insert(
                        child_id,
                        DerivedCollapsibleGroup {
                            header: header.clone(),
                            is_umbrella: false,
                        },
                    );
                }
            }
            index = next_index;
            continue;
        }
        index += 1;
    }

    let mut subcomponent_family_members = HashMap::<String, Vec<String>>::new();
    let mut subcomponent_family_headers = HashMap::<String, String>::new();
    let mut subcomponent_family_conflicts = HashSet::<String>::new();
    for block in &ordered_blocks {
        let child_id = block.component_id.trim();
        if child_id.is_empty() || !display_by_id.contains_key(child_id) {
            continue;
        }
        let Some(parent_id) = block
            .subcomponent_key
            .as_deref()
            .and_then(parse_subcomponent_parent_component_id)
        else {
            continue;
        };
        if !display_by_id.contains_key(parent_id.as_str()) {
            continue;
        }
        let Some(child_display) = display_by_id.get(child_id) else {
            continue;
        };
        let Some((header, _)) = split_subcomponent_display_label(child_display) else {
            continue;
        };
        if let Some(existing_header) = subcomponent_family_headers.get(parent_id.as_str()) {
            if !existing_header.eq_ignore_ascii_case(&header) {
                subcomponent_family_conflicts.insert(parent_id.clone());
                continue;
            }
        } else {
            subcomponent_family_headers.insert(parent_id.clone(), header.clone());
        }
        let family_members = subcomponent_family_members.entry(parent_id).or_default();
        if !family_members.iter().any(|existing| existing == child_id) {
            family_members.push(child_id.to_string());
        }
    }
    for parent_id in &subcomponent_family_conflicts {
        subcomponent_family_members.remove(parent_id);
        subcomponent_family_headers.remove(parent_id);
    }
    for (parent_id, child_ids) in subcomponent_family_members {
        if child_ids.is_empty()
            || out.contains_key(parent_id.as_str())
            || child_ids.iter().any(|child_id| out.contains_key(child_id.as_str()))
        {
            continue;
        }
        let Some(header) = subcomponent_family_headers.get(parent_id.as_str()).cloned() else {
            continue;
        };
        out.insert(
            parent_id.clone(),
            DerivedCollapsibleGroup {
                header: header.clone(),
                is_umbrella: true,
            },
        );
        for child_id in child_ids {
            out.insert(
                child_id,
                DerivedCollapsibleGroup {
                    header: header.clone(),
                    is_umbrella: false,
                },
            );
        }
    }

    let mut index = 0usize;
    while index < components.len() {
        let Some((header, _)) = split_subcomponent_display_label(&components[index].display) else {
            index += 1;
            continue;
        };

        let mut member_ids = vec![components[index].component_id.trim().to_string()];
        let mut same_header_count = 1usize;
        let mut bridged_placeholder_count = 0usize;
        let mut next_index = index + 1;

        while next_index < components.len() {
            let next_component = &components[next_index];
            if let Some((next_header, _)) = split_subcomponent_display_label(&next_component.display)
            {
                if next_header.eq_ignore_ascii_case(&header) {
                    member_ids.push(next_component.component_id.trim().to_string());
                    same_header_count += 1;
                    next_index += 1;
                    continue;
                }
                break;
            }

            if placeholder_bridges_subcomponent_family(
                components,
                next_index,
                &header,
                &deprecated_placeholder_ids,
            ) {
                member_ids.push(next_component.component_id.trim().to_string());
                bridged_placeholder_count += 1;
                next_index += 1;
                continue;
            }
            break;
        }

        if same_header_count >= 2 && bridged_placeholder_count > 0 {
            for component_id in member_ids {
                out.entry(component_id).or_insert_with(|| DerivedCollapsibleGroup {
                    header: header.clone(),
                    is_umbrella: false,
                });
            }
            index = next_index;
            continue;
        }

        index += 1;
    }

    out
}

fn parse_subcomponent_parent_component_id(token: &str) -> Option<String> {
    let trimmed = token.trim();
    let raw_digits = if let Some(rest) = trimmed.strip_prefix('@') {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        (!digits.is_empty()).then_some(digits)
    } else if trimmed.chars().all(|c| c.is_ascii_digit()) {
        Some(trimmed.to_string())
    } else {
        None
    }?;
    let normalized = raw_digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("0".to_string())
    } else {
        Some(normalized.to_string())
    }
}

fn derive_collapsible_group_header(umbrella_display: &str) -> String {
    let trimmed = umbrella_display.trim();
    let without_parenthetical = if trimmed.ends_with(')') {
        trimmed
            .rsplit_once('(')
            .map(|(head, _)| head.trim_end())
            .filter(|head| !head.is_empty())
            .unwrap_or(trimmed)
    } else {
        trimmed
    };
    let lower = without_parenthetical.to_ascii_lowercase();
    if lower.starts_with("install all spell tweaks") {
        return "Spell Tweaks".to_string();
    }
    let derived = lower
        .strip_prefix("install all ")
        .map(str::trim)
        .or_else(|| lower.strip_prefix("all ").map(str::trim));
    if let Some(rest) = derived
        && !rest.is_empty()
    {
        return title_case_words(rest);
    }
    without_parenthetical.to_string()
}

fn title_case_words(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_ascii_lowercase());
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn inherit_group_for_deprecated_placeholder(
    ordered_blocks: &[super::tp2_blocks::Tp2ComponentBlock],
    index: usize,
    tra_map: &HashMap<String, String>,
    previous_group: Option<&str>,
) -> Option<String> {
    let previous_group = previous_group?.trim();
    if previous_group.is_empty() || !block_is_deprecated_placeholder(&ordered_blocks[index]) {
        return None;
    }

    let next_group = ordered_blocks
        .iter()
        .skip(index + 1)
        .find_map(|block| {
            if block_is_deprecated_placeholder(block) && block.group_key.is_none() {
                return None;
            }
            block.group_key
                .as_deref()
                .and_then(|group_token| resolve_group_token_label(group_token, tra_map))
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })?;

    next_group
        .eq_ignore_ascii_case(previous_group)
        .then(|| previous_group.to_string())
}

fn block_is_deprecated_placeholder(block: &super::tp2_blocks::Tp2ComponentBlock) -> bool {
    let mut saw_deprecated = false;
    for line in &block.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();
        if upper.contains("DEPRECATED") {
            saw_deprecated = true;
        }
        if upper.starts_with("BEGIN ")
            || upper.starts_with("GROUP ")
            || upper.starts_with("LABEL ")
            || upper.starts_with("SUBCOMPONENT ")
            || upper.starts_with("FORCED_SUBCOMPONENT ")
            || upper.starts_with("REQUIRE_")
            || upper.starts_with("DESIGNATED ")
        {
            continue;
        }
        return false;
    }
    saw_deprecated
}

fn placeholder_bridges_subcomponent_family(
    components: &[ScannedComponent],
    index: usize,
    header: &str,
    deprecated_placeholder_ids: &HashSet<String>,
) -> bool {
    let component = &components[index];
    if !deprecated_placeholder_ids.contains(component.component_id.trim()) {
        return false;
    }

    let placeholder_label = component.display.trim();
    if placeholder_label.is_empty() {
        return false;
    }

    let mut next_index = index + 1;
    while next_index < components.len() {
        let next_component = &components[next_index];
        if let Some((next_header, next_choice)) =
            split_subcomponent_display_label(&next_component.display)
        {
            if next_header.eq_ignore_ascii_case(header) {
                if next_choice.trim().eq_ignore_ascii_case(placeholder_label) {
                    return true;
                }
                next_index += 1;
                continue;
            }
            break;
        }

        if deprecated_placeholder_ids.contains(next_component.component_id.trim()) {
            next_index += 1;
            continue;
        }
        break;
    }

    false
}

fn parse_same_mod_installed_guard_target(tp_file: &str, line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("REQUIRE_PREDICATE") || !upper.contains("!MOD_IS_INSTALLED") {
        return None;
    }

    let guard_index = upper.find("!MOD_IS_INSTALLED")?;
    let after = trimmed[guard_index + "!MOD_IS_INSTALLED".len()..].trim_start();
    let quote = after.chars().next()?;
    if quote != '~' && quote != '"' {
        return None;
    }

    let rest = &after[quote.len_utf8()..];
    let end = rest.find(quote)?;
    let raw_path = rest[..end].trim();
    if raw_path.is_empty() {
        return None;
    }

    let normalized_path = raw_path.replace('\\', "/");
    let raw_file = Path::new(&normalized_path)
        .file_name()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    if raw_file != tp_file.to_ascii_lowercase() {
        return None;
    }

    let tail = rest[end + quote.len_utf8()..].trim_start();
    let component_id: String = tail.chars().take_while(|c| c.is_ascii_digit()).collect();
    if component_id.is_empty() {
        None
    } else {
        Some(component_id)
    }
}

#[cfg(test)]
#[path = "worker_build_states_groups_tests.rs"]
mod tests;
