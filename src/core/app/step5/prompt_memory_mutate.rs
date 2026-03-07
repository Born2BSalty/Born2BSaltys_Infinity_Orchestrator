// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{PromptAnswerEntry, alias, storage};

#[derive(Debug, Clone, Default)]
pub struct PromptAnswerContext {
    pub component_key: String,
    pub tp2_file: String,
    pub component_id: String,
    pub component_name: String,
    pub prompt_kind: String,
    pub source: String,
}

pub(super) fn alias_from_preview(preview: &str) -> String {
    alias::suggest_alias_from_preview(preview)
}

pub(super) fn remember_answer_with_context(
    prompt_key: &str,
    answer: &str,
    preview: &str,
    ctx: &PromptAnswerContext,
) {
    if prompt_key.trim().is_empty() {
        return;
    }
    let now = now_unix_secs();
    if let Ok(mut guard) = storage::memory().lock() {
        let mut entry = guard.get(prompt_key).cloned().unwrap_or_default();
        entry.answer = answer.to_string();
        entry.enabled = true;
        if entry.captured_at == 0 {
            entry.captured_at = now;
        }
        entry.last_used_at = now;
        entry.hit_count = entry.hit_count.saturating_add(1);
        if !ctx.component_key.trim().is_empty() {
            entry.component_key = ctx.component_key.trim().to_string();
        }
        if !ctx.tp2_file.trim().is_empty() {
            entry.tp2_file = ctx.tp2_file.trim().to_string();
        }
        if !ctx.component_id.trim().is_empty() {
            entry.component_id = ctx.component_id.trim().to_string();
        }
        if !ctx.component_name.trim().is_empty() {
            entry.component_name = ctx.component_name.trim().to_string();
        }
        if !ctx.prompt_kind.trim().is_empty() {
            entry.prompt_kind = ctx.prompt_kind.trim().to_string();
        }
        if !ctx.source.trim().is_empty() {
            entry.source = ctx.source.trim().to_string();
        }
        if !preview.trim().is_empty() {
            entry.preview = preview.to_string();
            if entry.alias.trim().is_empty() {
                entry.alias = alias::suggest_alias_from_preview(preview);
            }
        }
        guard.insert(prompt_key.to_string(), entry);
        storage::dedupe_in_place(&mut guard);
        let _ = storage::save_to_disk(&guard);
    }
}

pub(super) fn ensure_prompt_entry(prompt_key: &str, preview: &str) {
    if prompt_key.trim().is_empty() {
        return;
    }
    if let Ok(mut guard) = storage::memory().lock() {
        if let Some(entry) = guard.get_mut(prompt_key) {
            if !preview.trim().is_empty() {
                entry.preview = preview.to_string();
                if entry.alias.trim().is_empty() {
                    entry.alias = alias::suggest_alias_from_preview(preview);
                }
            }
            let _ = storage::save_to_disk(&guard);
            return;
        }
        guard.entry(prompt_key.to_string()).or_insert_with(|| PromptAnswerEntry {
            alias: alias::suggest_alias_from_preview(preview),
            answer: String::new(),
            enabled: false,
            preview: preview.to_string(),
            component_key: String::new(),
            tp2_file: String::new(),
            component_id: String::new(),
            component_name: String::new(),
            prompt_kind: String::new(),
            source: String::new(),
            captured_at: 0,
            last_used_at: 0,
            hit_count: 0,
        });
        storage::dedupe_in_place(&mut guard);
        let _ = storage::save_to_disk(&guard);
    }
}

pub(super) fn set_answer(prompt_key: &str, answer: &str) {
    if let Ok(mut guard) = storage::memory().lock()
        && let Some(entry) = guard.get_mut(prompt_key)
    {
        entry.answer = answer.to_string();
        let _ = storage::save_to_disk(&guard);
    }
}

pub(super) fn set_enabled(prompt_key: &str, enabled: bool) {
    if let Ok(mut guard) = storage::memory().lock()
        && let Some(entry) = guard.get_mut(prompt_key)
    {
        entry.enabled = enabled;
        let _ = storage::save_to_disk(&guard);
    }
}

pub(super) fn set_alias(prompt_key: &str, alias_value: &str) {
    if let Ok(mut guard) = storage::memory().lock()
        && let Some(entry) = guard.get_mut(prompt_key)
    {
        entry.alias = alias_value.trim().to_string();
        let _ = storage::save_to_disk(&guard);
    }
}

pub(super) fn attach_key_to_alias(alias_value: &str, prompt_key: &str, preview: &str) {
    let alias_value = alias_value.trim();
    if alias_value.is_empty() || prompt_key.trim().is_empty() {
        return;
    }
    if let Ok(mut guard) = storage::memory().lock() {
        let mut source_key: Option<String> = None;
        for (key, entry) in guard.iter() {
            if entry.alias.eq_ignore_ascii_case(alias_value) && key != prompt_key {
                source_key = Some(key.clone());
                break;
            }
        }

        if let Some(src_key) = source_key
            && let Some(src_entry) = guard.remove(&src_key)
        {
            let mut dst_entry = guard.remove(prompt_key).unwrap_or_default();
            if dst_entry.answer.trim().is_empty() && !src_entry.answer.trim().is_empty() {
                dst_entry.answer = src_entry.answer;
            }
            dst_entry.enabled = src_entry.enabled;
            if dst_entry.alias.trim().is_empty() {
                dst_entry.alias = src_entry.alias;
            }
            if dst_entry.preview.trim().is_empty() && !src_entry.preview.trim().is_empty() {
                dst_entry.preview = src_entry.preview;
            }
            if !preview.trim().is_empty() {
                dst_entry.preview = preview.to_string();
            }
            guard.insert(prompt_key.to_string(), dst_entry);
            let _ = storage::save_to_disk(&guard);
            return;
        }

        if let Some(existing) = guard.get_mut(prompt_key) {
            if existing.alias.trim().is_empty() {
                existing.alias = alias_value.to_string();
            }
            if !preview.trim().is_empty() {
                existing.preview = preview.to_string();
            }
            storage::dedupe_in_place(&mut guard);
            let _ = storage::save_to_disk(&guard);
        }
    }
}

pub(super) fn delete_entry(prompt_key: &str) {
    if let Ok(mut guard) = storage::memory().lock() {
        guard.remove(prompt_key);
        let _ = storage::save_to_disk(&guard);
    }
}

fn now_unix_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
