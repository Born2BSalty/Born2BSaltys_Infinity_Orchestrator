// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use super::compat_tp2_blocks::{
    FamilyIdentity, ForbidFlag, collect_component_flags, collect_tp2_component_blocks,
};

use super::compat_rule_runtime::normalize_mod_key;
use super::compat_setup_tra::load_tp2_setup_tra_map;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConflictOrigin {
    pub(crate) component_id: String,
    pub(crate) line: usize,
    pub(crate) inherited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComponentConflict {
    pub(crate) raw_line: String,
    pub(crate) target_mod: String,
    pub(crate) target_component_id: String,
    pub(crate) message: Option<String>,
    pub(crate) origin: Option<ConflictOrigin>,
}

impl ComponentConflict {
    pub(super) fn evidence(&self) -> String {
        self.origin
            .as_ref()
            .filter(|origin| origin.inherited)
            .map_or_else(
                || self.raw_line.clone(),
                |origin| format!(
                    "{}\nShared requirement inherited from first subcomponent #{}, TP2 line {}.",
                    self.raw_line, origin.component_id, origin.line
                ),
            )
    }
}

pub(crate) fn load_component_conflicts(tp2_path: &str) -> HashMap<String, Vec<ComponentConflict>> {
    if tp2_path.trim().is_empty() {
        return HashMap::new();
    }
    let cache = conflict_cache();
    let mut cache = cache.lock().expect("compat conflict cache lock poisoned");
    let stamp = cache_stamp(tp2_path);

    if let Some(entry) = cache.get(tp2_path)
        && entry.stamp == stamp
    {
        return entry.conflicts.clone();
    }

    let conflicts = load_component_conflicts_uncached(tp2_path);
    cache.insert(
        tp2_path.to_string(),
        CachedConflicts {
            stamp,
            conflicts: conflicts.clone(),
        },
    );
    conflicts
}

fn load_component_conflicts_uncached(tp2_path: &str) -> HashMap<String, Vec<ComponentConflict>> {
    let Ok(tp2_text) = crate::app::compat_text::read_compat_text(std::path::Path::new(tp2_path))
    else {
        return HashMap::new();
    };
    let tra_map = load_tp2_setup_tra_map(std::path::Path::new(tp2_path));

    let mut out = HashMap::<String, Vec<ComponentConflict>>::new();
    for (component_id, block) in collect_tp2_component_blocks(&tp2_text) {
        let conflicts = collect_component_conflicts(&block, &tra_map, &component_id, &tp2_text);
        if !conflicts.is_empty() {
            out.insert(component_id, conflicts);
        }
    }
    let mut first_forbids = HashMap::<FamilyIdentity, Vec<ComponentConflict>>::new();
    for component in collect_component_flags(&tp2_text) {
        let direct = component
            .forbids
            .iter()
            .filter_map(|flag| header_conflict(&component.component_id, flag, &tra_map))
            .collect::<Vec<_>>();
        let mut merged = direct.clone();
        merged.extend(out.remove(&component.component_id).unwrap_or_default());
        if let Some(family) = component.family {
            if let Some(inherited) = first_forbids.get(&family) {
                merged.extend(inherited.iter().cloned().map(|mut conflict| {
                    if let Some(origin) = &mut conflict.origin {
                        origin.inherited = true;
                    }
                    conflict
                }));
            } else {
                first_forbids.insert(family, direct);
            }
        }
        let mut targets = std::collections::HashSet::new();
        merged.retain(|conflict| {
            targets.insert((
                conflict.target_mod.clone(),
                conflict.target_component_id.clone(),
            ))
        });
        if !merged.is_empty() {
            out.insert(component.component_id, merged);
        }
    }
    out
}

fn header_conflict(
    component_id: &str,
    flag: &ForbidFlag,
    tra_map: &HashMap<String, String>,
) -> Option<ComponentConflict> {
    let target = flag.target_reference.trim_matches(['~', '"']);
    if target.contains('%') {
        return None;
    }
    let target_mod = normalize_mod_key(target);
    if target_mod.is_empty() {
        return None;
    }
    Some(ComponentConflict {
        raw_line: flag.raw_line.clone(),
        target_mod,
        target_component_id: normalize_component_id(&flag.component_reference)?,
        message: resolve_message_token(&flag.message_token, tra_map),
        origin: Some(ConflictOrigin {
            component_id: component_id.to_string(),
            line: flag.line,
            inherited: false,
        }),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileCacheStamp {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct CachedConflicts {
    stamp: FileCacheStamp,
    conflicts: HashMap<String, Vec<ComponentConflict>>,
}

fn conflict_cache() -> &'static Mutex<HashMap<String, CachedConflicts>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CachedConflicts>>> = OnceLock::new();
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

fn collect_component_conflicts(
    block: &[&str],
    tra_map: &HashMap<String, String>,
    component_id: &str,
    tp2_text: &str,
) -> Vec<ComponentConflict> {
    let mut out = Vec::<ComponentConflict>::new();
    for line in block {
        let offset = line.as_ptr().addr() - tp2_text.as_ptr().addr();
        let line_number = tp2_text[..offset]
            .bytes()
            .filter(|&byte| byte == b'\n')
            .count()
            + 1;
        out.extend(
            parse_conflict_line(line, tra_map)
                .into_iter()
                .map(|mut conflict| {
                    conflict.origin = Some(ConflictOrigin {
                        component_id: component_id.to_string(),
                        line: line_number,
                        inherited: false,
                    });
                    conflict
                }),
        );
    }
    out
}

fn parse_conflict_line(line: &str, tra_map: &HashMap<String, String>) -> Vec<ComponentConflict> {
    let stripped = strip_inline_comments(line);
    let trimmed = stripped.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let upper = trimmed.to_ascii_uppercase();
    let mut out = Vec::<ComponentConflict>::new();
    let mut search_from = 0usize;

    while let Some(found) = upper[search_from..].find("FORBID_COMPONENT") {
        let start = search_from + found;
        let mut index = start + "FORBID_COMPONENT".len();
        let Some(tp2_ref) = read_statement_token(trimmed, &mut index) else {
            break;
        };
        let Some(component_ref) = read_statement_token(trimmed, &mut index) else {
            break;
        };
        let target_mod = normalize_mod_key(&tp2_ref);
        let Some(target_component_id) = normalize_component_id(&component_ref) else {
            break;
        };
        let message = read_statement_token(trimmed, &mut index)
            .and_then(|value| resolve_message_token(&value, tra_map));
        let next_start = upper[index..]
            .find("FORBID_COMPONENT")
            .map_or_else(|| trimmed.len(), |offset| index + offset);
        if !target_mod.is_empty() {
            out.push(ComponentConflict {
                raw_line: trimmed[start..next_start].trim().to_string(),
                target_mod,
                target_component_id,
                message,
                origin: None,
            });
        }
        search_from = next_start;
    }

    out
}

fn strip_inline_comments(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut index = 0usize;
    let mut quote = None::<char>;

    while index < chars.len() {
        let ch = chars[index];
        if let Some(active) = quote {
            out.push(ch);
            if ch == active {
                quote = None;
            }
            index += 1;
            continue;
        }

        if ch == '~' || ch == '"' {
            quote = Some(ch);
            out.push(ch);
            index += 1;
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            break;
        }
        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            break;
        }

        out.push(ch);
        index += 1;
    }

    out
}

fn read_statement_token(input: &str, index: &mut usize) -> Option<String> {
    let bytes = input.as_bytes();
    while *index < bytes.len() && bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    if *index >= bytes.len() {
        return None;
    }

    let quote = bytes[*index];
    if quote == b'~' || quote == b'"' {
        *index += 1;
        let start = *index;
        while *index < bytes.len() && bytes[*index] != quote {
            *index += 1;
        }
        let value = input[start..(*index).min(bytes.len())].trim().to_string();
        if *index < bytes.len() {
            *index += 1;
        }
        return Some(value);
    }

    let start = *index;
    while *index < bytes.len() && !bytes[*index].is_ascii_whitespace() {
        *index += 1;
    }
    Some(input[start..*index].trim().to_string())
}

fn normalize_component_id(value: &str) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let normalized = digits.trim_start_matches('0');
    if normalized.is_empty() {
        Some("0".to_string())
    } else {
        Some(normalized.to_string())
    }
}

fn resolve_message_token(value: &str, tra_map: &HashMap<String, String>) -> Option<String> {
    let trimmed = value
        .trim()
        .trim_matches(|ch: char| matches!(ch, '~' | '"' | '\''));
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('@') {
        return tra_map.get(trimmed).cloned();
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
#[path = "compat_conflict_parse_tests.rs"]
mod tests;
