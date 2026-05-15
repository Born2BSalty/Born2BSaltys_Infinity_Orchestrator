// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::app::scan::ScannedComponent;

use super::super::tp2_blocks::{
    Tp2ComponentBlock, parse_tp2_component_blocks_in_order, split_subcomponent_display_label,
};
use super::{DerivedCollapsibleGroup, block_is_deprecated_placeholder};

pub(super) fn detect_derived_collapsible_groups(
    tp_file: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, DerivedCollapsibleGroup> {
    let ordered_blocks = parse_tp2_component_blocks_in_order(tp2_text);
    if ordered_blocks.len() < 3 {
        return HashMap::new();
    }

    let deprecated_placeholder_ids = deprecated_placeholder_ids(&ordered_blocks);
    let display_by_id = display_by_component_id(components);
    let same_mod_installed_guards = same_mod_installed_guards(tp_file, &ordered_blocks);
    let mut out = HashMap::<String, DerivedCollapsibleGroup>::new();
    detect_same_mod_umbrella_groups(
        &ordered_blocks,
        &display_by_id,
        &same_mod_installed_guards,
        &mut out,
    );
    detect_subcomponent_parent_groups(&ordered_blocks, &display_by_id, &mut out);
    detect_bridged_subcomponent_groups(components, &deprecated_placeholder_ids, &mut out);

    out
}

fn deprecated_placeholder_ids(ordered_blocks: &[Tp2ComponentBlock]) -> HashSet<String> {
    ordered_blocks
        .iter()
        .filter(|block| block_is_deprecated_placeholder(block))
        .map(|block| block.component_id.trim().to_string())
        .collect()
}

fn display_by_component_id(components: &[ScannedComponent]) -> HashMap<String, String> {
    components
        .iter()
        .map(|component| {
            (
                component.component_id.trim().to_string(),
                component.display.trim().to_string(),
            )
        })
        .collect()
}

fn same_mod_installed_guards(
    tp_file: &str,
    ordered_blocks: &[Tp2ComponentBlock],
) -> HashMap<String, String> {
    let mut guards = HashMap::<String, String>::new();
    for block in ordered_blocks {
        let mut targets = block
            .body_lines
            .iter()
            .filter_map(|line| parse_same_mod_installed_guard_target(tp_file, line))
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        if targets.len() == 1 {
            guards.insert(block.component_id.clone(), targets[0].clone());
        }
    }
    guards
}

fn detect_same_mod_umbrella_groups(
    ordered_blocks: &[Tp2ComponentBlock],
    display_by_id: &HashMap<String, String>,
    same_mod_installed_guards: &HashMap<String, String>,
    out: &mut HashMap<String, DerivedCollapsibleGroup>,
) {
    let mut index = 0usize;
    while index < ordered_blocks.len() {
        let (next_index, group) = same_mod_umbrella_group_at(
            index,
            ordered_blocks,
            display_by_id,
            same_mod_installed_guards,
        );
        if let Some((umbrella_id, header, child_ids)) = group {
            out.insert(
                umbrella_id,
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
        } else {
            index += 1;
        }
    }
}

fn same_mod_umbrella_group_at(
    index: usize,
    ordered_blocks: &[Tp2ComponentBlock],
    display_by_id: &HashMap<String, String>,
    same_mod_installed_guards: &HashMap<String, String>,
) -> (usize, Option<(String, String, Vec<String>)>) {
    let umbrella = &ordered_blocks[index];
    let umbrella_id = umbrella.component_id.trim();
    let Some(umbrella_display) = display_by_id.get(umbrella_id) else {
        return (index + 1, None);
    };
    if split_subcomponent_display_label(umbrella_display).is_some() {
        return (index + 1, None);
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
        (
            next_index,
            Some((umbrella_id.to_string(), header, child_ids)),
        )
    } else {
        (index + 1, None)
    }
}

fn detect_subcomponent_parent_groups(
    ordered_blocks: &[Tp2ComponentBlock],
    display_by_id: &HashMap<String, String>,
    out: &mut HashMap<String, DerivedCollapsibleGroup>,
) {
    let mut subcomponent_family_members = HashMap::<String, Vec<String>>::new();
    let mut subcomponent_family_headers = HashMap::<String, String>::new();
    let mut subcomponent_family_conflicts = HashSet::<String>::new();
    for block in ordered_blocks {
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
            || child_ids
                .iter()
                .any(|child_id| out.contains_key(child_id.as_str()))
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
}

fn detect_bridged_subcomponent_groups(
    components: &[ScannedComponent],
    deprecated_placeholder_ids: &HashSet<String>,
    out: &mut HashMap<String, DerivedCollapsibleGroup>,
) {
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
            if let Some((next_header, _)) =
                split_subcomponent_display_label(&next_component.display)
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
                deprecated_placeholder_ids,
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
                out.entry(component_id)
                    .or_insert_with(|| DerivedCollapsibleGroup {
                        header: header.clone(),
                        is_umbrella: false,
                    });
            }
            index = next_index;
            continue;
        }

        index += 1;
    }
}

fn parse_subcomponent_parent_component_id(token: &str) -> Option<String> {
    let trimmed = token.trim();
    let raw_digits = trimmed.strip_prefix('@').map_or_else(
        || {
            trimmed
                .chars()
                .all(|c| c.is_ascii_digit())
                .then(|| trimmed.to_string())
        },
        |rest| {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (!digits.is_empty()).then_some(digits)
        },
    )?;
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
    let component_id: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if component_id.is_empty() {
        None
    } else {
        Some(component_id)
    }
}
