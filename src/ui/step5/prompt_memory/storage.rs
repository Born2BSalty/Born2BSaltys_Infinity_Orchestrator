// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::platform_defaults::app_config_file;

use super::types::{PromptAnswerDiskEntry, PromptAnswerEntry, PromptAnswersDisk};

static PROMPT_MEMORY: OnceLock<Mutex<HashMap<String, PromptAnswerEntry>>> = OnceLock::new();

pub(super) fn memory() -> &'static Mutex<HashMap<String, PromptAnswerEntry>> {
    PROMPT_MEMORY.get_or_init(|| Mutex::new(load_from_disk().unwrap_or_default()))
}

pub(super) fn load_from_disk() -> Option<HashMap<String, PromptAnswerEntry>> {
    let path = prompt_memory_path();
    let content = fs::read_to_string(path).ok()?;
    parse_content(&content)
}

pub(super) fn save_to_disk(map: &HashMap<String, PromptAnswerEntry>) -> std::io::Result<()> {
    let path = prompt_memory_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut deduped = map.clone();
    dedupe_in_place(&mut deduped);
    let raw = serialize_map(&deduped)?;
    fs::write(path, raw)
}

pub(super) fn serialize_map(map: &HashMap<String, PromptAnswerEntry>) -> std::io::Result<String> {
    let mut entries = Vec::with_capacity(map.len());
    let mut sorted = BTreeMap::new();
    for (k, v) in map {
        sorted.insert(k.clone(), v.clone());
    }
    for (key, value) in sorted {
        let disk_key = if key.starts_with("ENTRY:") {
            String::new()
        } else {
            key
        };
        entries.push(PromptAnswerDiskEntry {
            key: disk_key,
            alias: value.alias,
            answer: value.answer,
            enabled: value.enabled,
            preview: value.preview,
            component_key: value.component_key,
            tp2_file: value.tp2_file,
            component_id: value.component_id,
            component_name: value.component_name,
            prompt_kind: value.prompt_kind,
            source: value.source,
            captured_at: value.captured_at,
            last_used_at: value.last_used_at,
            hit_count: value.hit_count,
        });
    }
    let disk = PromptAnswersDisk { entries };
    serde_json::to_string_pretty(&disk).map_err(|err| std::io::Error::other(err.to_string()))
}

pub(super) fn parse_content(content: &str) -> Option<HashMap<String, PromptAnswerEntry>> {
    if let Ok(disk) = serde_json::from_str::<PromptAnswersDisk>(content) {
        let has_entries_shape = content.to_ascii_lowercase().contains("\"entries\"");
        if has_entries_shape {
            let mut parsed = HashMap::new();
            let mut synthetic_idx = 0usize;
            for item in disk.entries {
                let mut key = item.key.trim().to_string();
                if key.is_empty() {
                    if item.alias.trim().is_empty() {
                        continue;
                    }
                    key = format!("ENTRY:{synthetic_idx}");
                    synthetic_idx = synthetic_idx.saturating_add(1);
                }
                parsed.insert(
                    key,
                    PromptAnswerEntry {
                        alias: item.alias.trim().to_string(),
                        answer: item.answer,
                        enabled: item.enabled,
                        preview: item.preview,
                        component_key: item.component_key,
                        tp2_file: item.tp2_file,
                        component_id: item.component_id,
                        component_name: item.component_name,
                        prompt_kind: item.prompt_kind,
                        source: item.source,
                        captured_at: item.captured_at,
                        last_used_at: item.last_used_at,
                        hit_count: item.hit_count,
                    },
                );
            }
            dedupe_in_place(&mut parsed);
            return Some(parsed);
        }
    }
    if let Ok(new_map) = serde_json::from_str::<HashMap<String, PromptAnswerEntry>>(content) {
        let mut deduped = new_map;
        dedupe_in_place(&mut deduped);
        return Some(deduped);
    }
    if let Ok(old_map) = serde_json::from_str::<HashMap<String, String>>(content) {
        let mut converted = HashMap::new();
        for (k, v) in old_map {
            converted.insert(
                k,
                PromptAnswerEntry {
                    alias: String::new(),
                    answer: v,
                    enabled: true,
                    preview: String::new(),
                    component_key: String::new(),
                    tp2_file: String::new(),
                    component_id: String::new(),
                    component_name: String::new(),
                    prompt_kind: String::new(),
                    source: String::new(),
                    captured_at: 0,
                    last_used_at: 0,
                    hit_count: 0,
                },
            );
        }
        dedupe_in_place(&mut converted);
        return Some(converted);
    }
    None
}

pub(super) fn dedupe_in_place(map: &mut HashMap<String, PromptAnswerEntry>) {
    let mut by_signature: HashMap<String, (String, PromptAnswerEntry)> = HashMap::new();
    let mut keys: Vec<String> = map.keys().cloned().collect();
    keys.sort();
    for key in keys {
        let Some(entry) = map.get(&key).cloned() else {
            continue;
        };
        let signature = entry_signature(&entry);
        if let Some((keep_key, keep_entry)) = by_signature.get_mut(&signature) {
            if prefer_new_key(&key, keep_key) {
                let merged = merge_entries(keep_entry.clone(), entry);
                *keep_key = key;
                *keep_entry = merged;
            } else {
                *keep_entry = merge_entries(keep_entry.clone(), entry);
            }
        } else {
            by_signature.insert(signature, (key, entry));
        }
    }
    map.clear();
    for (_, (key, entry)) in by_signature {
        map.insert(key, entry);
    }
}

fn entry_signature(entry: &PromptAnswerEntry) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        entry.component_key.trim().to_ascii_lowercase(),
        entry.tp2_file.trim().to_ascii_lowercase(),
        entry.component_id.trim().to_ascii_lowercase(),
        entry.component_name.trim().to_ascii_lowercase(),
        entry.alias.trim().to_ascii_lowercase(),
        entry.preview.trim().to_ascii_lowercase(),
        entry.answer.trim(),
        entry.prompt_kind.trim().to_ascii_lowercase(),
    )
}

fn prefer_new_key(new_key: &str, old_key: &str) -> bool {
    let new_is_entry = new_key.starts_with("ENTRY:");
    let old_is_entry = old_key.starts_with("ENTRY:");
    if new_is_entry != old_is_entry {
        return !new_is_entry;
    }
    new_key < old_key
}

fn merge_entries(mut keep: PromptAnswerEntry, incoming: PromptAnswerEntry) -> PromptAnswerEntry {
    keep.enabled = keep.enabled || incoming.enabled;
    keep.hit_count = keep.hit_count.saturating_add(incoming.hit_count);
    if keep.captured_at == 0
        || (incoming.captured_at != 0 && incoming.captured_at < keep.captured_at)
    {
        keep.captured_at = incoming.captured_at;
    }
    if incoming.last_used_at > keep.last_used_at {
        keep.last_used_at = incoming.last_used_at;
    }
    if keep.alias.trim().is_empty() && !incoming.alias.trim().is_empty() {
        keep.alias = incoming.alias;
    }
    if keep.preview.trim().is_empty() && !incoming.preview.trim().is_empty() {
        keep.preview = incoming.preview;
    }
    if keep.answer.trim().is_empty() && !incoming.answer.trim().is_empty() {
        keep.answer = incoming.answer;
    }
    if keep.component_key.trim().is_empty() && !incoming.component_key.trim().is_empty() {
        keep.component_key = incoming.component_key;
    }
    if keep.tp2_file.trim().is_empty() && !incoming.tp2_file.trim().is_empty() {
        keep.tp2_file = incoming.tp2_file;
    }
    if keep.component_id.trim().is_empty() && !incoming.component_id.trim().is_empty() {
        keep.component_id = incoming.component_id;
    }
    if keep.component_name.trim().is_empty() && !incoming.component_name.trim().is_empty() {
        keep.component_name = incoming.component_name;
    }
    if keep.prompt_kind.trim().is_empty() && !incoming.prompt_kind.trim().is_empty() {
        keep.prompt_kind = incoming.prompt_kind;
    }
    if keep.source.trim().is_empty() && !incoming.source.trim().is_empty() {
        keep.source = incoming.source;
    }
    keep
}

fn prompt_memory_path() -> PathBuf {
    app_config_file("prompt_answers.json", ".")
}
