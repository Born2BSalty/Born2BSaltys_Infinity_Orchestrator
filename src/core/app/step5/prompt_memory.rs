// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "prompt_memory_alias.rs"]
mod alias;
#[path = "prompt_memory_io_ops.rs"]
mod io_ops;
#[path = "prompt_memory_mutate.rs"]
pub mod mutate;
#[path = "prompt_memory_query.rs"]
mod query;
#[path = "prompt_memory_storage.rs"]
mod storage;
#[path = "prompt_memory_types.rs"]
pub mod types;

use std::path::Path;

pub(crate) use mutate::PromptAnswerContext;
pub(crate) use types::PromptAnswerEntry;

pub(crate) fn get_answer(prompt_key: &str) -> Option<String> {
    query::get_answer(prompt_key)
}

pub(crate) fn remember_answer_with_context(
    prompt_key: &str,
    answer: &str,
    preview: &str,
    ctx: &PromptAnswerContext,
) {
    mutate::remember_answer_with_context(prompt_key, answer, preview, ctx);
}

pub(crate) fn alias_from_preview(preview: &str) -> String {
    mutate::alias_from_preview(preview)
}

pub(crate) fn ensure_prompt_entry(prompt_key: &str, preview: &str) {
    mutate::ensure_prompt_entry(prompt_key, preview);
}

pub(crate) fn list_entries() -> Vec<(String, PromptAnswerEntry)> {
    query::list_entries()
}

pub(crate) fn set_answer(prompt_key: &str, answer: &str) {
    mutate::set_answer(prompt_key, answer);
}

pub(crate) fn set_enabled(prompt_key: &str, enabled: bool) {
    mutate::set_enabled(prompt_key, enabled);
}

pub(crate) fn set_alias(prompt_key: &str, alias_value: &str) {
    mutate::set_alias(prompt_key, alias_value);
}

pub(crate) fn display_name(prompt_key: &str) -> String {
    query::display_name(prompt_key)
}

pub(crate) fn get_answer_by_alias(alias_value: &str) -> Option<String> {
    query::get_answer_by_alias(alias_value)
}

pub(crate) fn list_component_sequences() -> std::collections::HashMap<String, Vec<String>> {
    query::list_component_sequences()
}

pub(crate) fn attach_key_to_alias(alias_value: &str, prompt_key: &str, preview: &str) {
    mutate::attach_key_to_alias(alias_value, prompt_key, preview);
}

pub(crate) fn delete_entry(prompt_key: &str) {
    mutate::delete_entry(prompt_key);
}

pub(crate) fn clear_all() {
    mutate::clear_all();
}

pub(crate) fn take_last_persist_error() -> Option<String> {
    mutate::take_last_persist_error()
}

pub(crate) fn take_last_load_error() -> Option<String> {
    storage::take_last_load_error()
}

pub(crate) fn export_json(path: &Path) -> std::io::Result<usize> {
    io_ops::export_json(path)
}

pub(crate) fn import_json(path: &Path) -> std::io::Result<usize> {
    io_ops::import_json(path)
}
