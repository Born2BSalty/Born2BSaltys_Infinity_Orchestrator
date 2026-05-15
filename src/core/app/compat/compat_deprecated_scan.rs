// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::app::state::{Step2ComponentState, Step2ModState};

use super::compat_rule_runtime::normalize_mod_key;

pub(crate) fn apply_step2_scan_deprecated(mods: &mut [Step2ModState]) {
    let mut deprecated_cache = HashMap::<String, HashMap<String, DeprecatedHit>>::new();

    for mod_state in mods {
        let current_mod_key = normalize_mod_key(&mod_state.tp_file);
        let deprecated_hits = deprecated_cache
            .entry(mod_state.tp2_path.clone())
            .or_insert_with(|| load_component_deprecated_hits(&mod_state.tp2_path));
        let propagated_hits =
            deprecated_empty_subcomponent_placeholder_hits(mod_state, deprecated_hits);

        for component in &mut mod_state.components {
            if component
                .compat_kind
                .as_deref()
                .is_some_and(|kind| !kind.eq_ignore_ascii_case("mismatch"))
            {
                continue;
            }

            let Some(hit) = deprecated_hits
                .get(component.component_id.trim())
                .or_else(|| propagated_hits.get(component.component_id.trim()))
            else {
                continue;
            };

            apply_deprecated(component, &current_mod_key, hit);
        }
    }
}

#[derive(Debug, Clone)]
struct DeprecatedHit {
    source: String,
    raw_evidence: String,
    message: String,
}

fn apply_deprecated(
    component: &mut Step2ComponentState,
    current_mod_key: &str,
    hit: &DeprecatedHit,
) {
    component.disabled = true;
    component.compat_kind = Some("deprecated".to_string());
    component.compat_source = Some(hit.source.clone());
    component.compat_related_mod = None;
    component.compat_related_component = None;
    component.compat_graph = Some(format!(
        "{} #{} deprecated",
        current_mod_key,
        component.component_id.trim()
    ));
    component.compat_evidence = Some(hit.raw_evidence.clone());
    component.disabled_reason = Some(hit.message.clone());
}

fn deprecated_empty_subcomponent_placeholder_hits(
    mod_state: &Step2ModState,
    direct_hits: &HashMap<String, DeprecatedHit>,
) -> HashMap<String, DeprecatedHit> {
    let mut groups = HashMap::<String, Vec<&Step2ComponentState>>::new();
    for component in &mod_state.components {
        let Some(key) = component.subcomponent_key.as_deref() else {
            continue;
        };
        groups.entry(key.to_string()).or_default().push(component);
    }

    let mut out = HashMap::<String, DeprecatedHit>::new();
    for group in groups.values() {
        if !group
            .iter()
            .all(|component| component.tp2_empty_placeholder_block)
        {
            continue;
        }
        let Some(direct_hit) = group
            .iter()
            .find_map(|component| direct_hits.get(component.component_id.trim()))
        else {
            continue;
        };
        for component in group {
            let component_id = component.component_id.trim();
            if direct_hits.contains_key(component_id) {
                continue;
            }
            out.insert(
                component_id.to_string(),
                DeprecatedHit {
                    source: direct_hit.source.clone(),
                    raw_evidence: direct_hit.raw_evidence.clone(),
                    message:
                        "TP2 marks another empty placeholder choice in this SUBCOMPONENT group as deprecated."
                            .to_string(),
                },
            );
        }
    }
    out
}

fn load_component_deprecated_hits(tp2_path: &str) -> HashMap<String, DeprecatedHit> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = deprecated_hit_file_cache();
    let mut cache = cache.lock().expect("compat deprecated cache lock poisoned");
    let stamp = cache_stamp(tp2_path);
    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.hits.clone();
    }
    let hits = load_component_deprecated_hits_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedDeprecatedHits {
            stamp,
            hits: hits.clone(),
        },
    );
    hits
}

fn load_component_deprecated_hits_uncached(tp2_path: &str) -> HashMap<String, DeprecatedHit> {
    let Ok(tp2_text) = fs::read_to_string(tp2_path) else {
        return HashMap::new();
    };

    let mut out = HashMap::<String, DeprecatedHit>::new();
    let lines: Vec<&str> = tp2_text.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if !line.to_ascii_uppercase().starts_with("BEGIN ") {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < lines.len() {
            let next = lines[index].trim_start().to_ascii_uppercase();
            if next.starts_with("BEGIN ") {
                break;
            }
            index += 1;
        }

        let block = &lines[start..index];
        let Some(component_id) = block
            .iter()
            .find_map(|entry| parse_designated_id(&entry.to_ascii_uppercase()))
        else {
            continue;
        };

        let Some(raw_evidence) = find_deprecated_evidence(block) else {
            continue;
        };

        out.insert(
            component_id,
            DeprecatedHit {
                source: deprecated_source(tp2_path),
                message: deprecated_message(&raw_evidence),
                raw_evidence,
            },
        );
    }

    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedDeprecatedHits {
    stamp: FileCacheStamp,
    hits: HashMap<String, DeprecatedHit>,
}

fn deprecated_hit_file_cache() -> &'static Mutex<HashMap<String, CachedDeprecatedHits>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedDeprecatedHits>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_stamp(tp2_path: &str) -> FileCacheStamp {
    fs::metadata(tp2_path).map_or(
        FileCacheStamp {
            modified: None,
            len: 0,
        },
        |meta| FileCacheStamp {
            modified: meta.modified().ok(),
            len: meta.len(),
        },
    )
}

fn find_deprecated_evidence(block: &[&str]) -> Option<String> {
    let begin_line = block.first()?.trim_start();
    if let Some(inline) = inline_begin_deprecated(begin_line) {
        return Some(inline.to_string());
    }

    block.iter().skip(1).find_map(|line| {
        let trimmed = line.trim_start();
        (!trimmed.starts_with("//") && trimmed.to_ascii_uppercase().starts_with("DEPRECATED"))
            .then(|| trimmed.to_string())
    })
}

fn inline_begin_deprecated(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }

    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("BEGIN ") {
        return None;
    }

    let designated_idx = upper.find("DESIGNATED")?;
    let designated_tail = &trimmed[designated_idx + "DESIGNATED".len()..];
    let designated_tail = designated_tail.trim_start();
    let digit_len = designated_tail
        .chars()
        .take_while(char::is_ascii_digit)
        .map(char::len_utf8)
        .sum::<usize>();
    let search_region = &designated_tail[digit_len..];
    let dep_idx = search_region.to_ascii_uppercase().find("DEPRECATED")?;
    Some(search_region[dep_idx..].trim_start())
}

fn deprecated_message(raw_evidence: &str) -> String {
    let Some(dep_idx) = raw_evidence.to_ascii_uppercase().find("DEPRECATED") else {
        return "TP2 marks this component as deprecated.".to_string();
    };
    let tail = raw_evidence[dep_idx + "DEPRECATED".len()..].trim_start();
    if let Some(start) = tail.find('~')
        && let Some(end_rel) = tail[start + 1..].find('~')
    {
        let message = tail[start + 1..start + 1 + end_rel].trim();
        if !message.is_empty() {
            return message.to_string();
        }
    }

    "TP2 marks this component as deprecated.".to_string()
}

fn parse_designated_id(upper_line: &str) -> Option<String> {
    if upper_line.trim_start().starts_with("//") {
        return None;
    }
    let index = upper_line.find("DESIGNATED")?;
    let tail = upper_line[index + "DESIGNATED".len()..].trim_start();
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        None
    } else {
        let normalized = digits.trim_start_matches('0');
        if normalized.is_empty() {
            Some("0".to_string())
        } else {
            Some(normalized.to_string())
        }
    }
}

fn deprecated_source(tp2_path: &str) -> String {
    tp2_path.trim().to_string()
}
