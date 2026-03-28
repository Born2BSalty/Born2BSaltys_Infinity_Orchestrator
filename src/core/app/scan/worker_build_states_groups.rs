// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::Path;

use crate::ui::scan::ScannedComponent;

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
        .collect::<std::collections::HashSet<_>>();

    let mut out = HashMap::<String, String>::new();
    let mut distinct = std::collections::HashSet::<String>::new();
    for block in &ordered_blocks {
        let component_id = block.component_id.trim();
        if !component_ids.contains(component_id) {
            continue;
        }
        let Some(group_token) = block.group_key.as_deref() else {
            continue;
        };
        let Some(group_label) = resolve_group_token_label(group_token, &tra_map) else {
            continue;
        };
        let cleaned = group_label.trim();
        if cleaned.is_empty() {
            continue;
        }
        out.insert(component_id.to_string(), cleaned.to_string());
        distinct.insert(cleaned.to_ascii_lowercase());
    }

    if distinct.len() < 2 {
        HashMap::new()
    } else {
        out
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

    out
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
