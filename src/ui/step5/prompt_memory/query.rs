// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::HashMap;

use super::{PromptAnswerEntry, storage};

pub(super) fn get_answer(prompt_key: &str) -> Option<String> {
    if prompt_key.trim().is_empty() {
        return None;
    }
    let guard = storage::memory().lock().ok()?;
    guard.get(prompt_key).and_then(|entry| {
        if entry.enabled {
            Some(entry.answer.clone())
        } else {
            None
        }
    })
}

pub(super) fn list_entries() -> Vec<(String, PromptAnswerEntry)> {
    let Ok(guard) = storage::memory().lock() else {
        return Vec::new();
    };
    let mut sorted = BTreeMap::new();
    for (k, v) in guard.iter() {
        sorted.insert(k.clone(), v.clone());
    }
    sorted.into_iter().collect()
}

pub(super) fn display_name(prompt_key: &str) -> String {
    let Ok(guard) = storage::memory().lock() else {
        return prompt_key.to_string();
    };
    if let Some(entry) = guard.get(prompt_key)
        && !entry.alias.trim().is_empty()
    {
        if prompt_key.starts_with("ENTRY:") {
            return entry.alias.trim().to_string();
        }
        return format!("{} ({prompt_key})", entry.alias.trim());
    }
    prompt_key.to_string()
}

pub(super) fn get_answer_by_alias(alias_value: &str) -> Option<String> {
    let alias_value = alias_value.trim();
    if alias_value.is_empty() {
        return None;
    }
    let guard = storage::memory().lock().ok()?;
    let mut found: Option<String> = None;
    for entry in guard.values() {
        if !entry.enabled {
            continue;
        }
        if entry.alias.eq_ignore_ascii_case(alias_value) {
            if found.is_some() {
                return None;
            }
            found = Some(entry.answer.clone());
        }
    }
    found
}

pub(super) fn get_component_sequence(component_key: &str) -> Option<String> {
    let component_key = component_key.trim();
    if component_key.is_empty() {
        return None;
    }
    let guard = storage::memory().lock().ok()?;
    let mut found: Option<String> = None;
    for entry in guard.values() {
        if !entry.enabled {
            continue;
        }
        if entry.component_key.eq_ignore_ascii_case(component_key) {
            let answer = entry.answer.trim();
            if answer.is_empty() {
                continue;
            }
            if found.is_some() {
                return None;
            }
            found = Some(answer.to_string());
        }
    }
    found
}

pub(super) fn list_component_sequences() -> HashMap<String, Vec<String>> {
    let mut out = HashMap::new();
    let Ok(guard) = storage::memory().lock() else {
        return out;
    };
    for entry in guard.values() {
        if !entry.enabled {
            continue;
        }
        let component_key = entry.component_key.trim();
        if component_key.is_empty() {
            continue;
        }
        let answer = entry.answer.trim();
        if answer.is_empty() {
            continue;
        }
        let tokens: Vec<String> = answer.split(',').map(|s| s.trim().to_string()).collect();
        if tokens.is_empty() {
            continue;
        }
        out.entry(component_key.to_string()).or_insert(tokens);
    }
    out
}
