// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod alias;
mod io_ops;
mod mutate;
mod query;
mod storage;
mod types;

use std::path::Path;

pub use types::PromptAnswerEntry;
pub use mutate::PromptAnswerContext;

pub fn get_answer(prompt_key: &str) -> Option<String> {
    query::get_answer(prompt_key)
}

pub fn remember_answer_with_context(
    prompt_key: &str,
    answer: &str,
    preview: &str,
    ctx: &PromptAnswerContext,
) {
    mutate::remember_answer_with_context(prompt_key, answer, preview, ctx);
}

pub fn alias_from_preview(preview: &str) -> String {
    mutate::alias_from_preview(preview)
}

pub fn ensure_prompt_entry(prompt_key: &str, preview: &str) {
    mutate::ensure_prompt_entry(prompt_key, preview);
}

pub fn list_entries() -> Vec<(String, PromptAnswerEntry)> {
    query::list_entries()
}

pub fn set_answer(prompt_key: &str, answer: &str) {
    mutate::set_answer(prompt_key, answer);
}

pub fn set_enabled(prompt_key: &str, enabled: bool) {
    mutate::set_enabled(prompt_key, enabled);
}

pub fn set_alias(prompt_key: &str, alias_value: &str) {
    mutate::set_alias(prompt_key, alias_value);
}

pub fn display_name(prompt_key: &str) -> String {
    query::display_name(prompt_key)
}

pub fn get_answer_by_alias(alias_value: &str) -> Option<String> {
    query::get_answer_by_alias(alias_value)
}

pub fn attach_key_to_alias(alias_value: &str, prompt_key: &str, preview: &str) {
    mutate::attach_key_to_alias(alias_value, prompt_key, preview);
}

pub fn delete_entry(prompt_key: &str) {
    mutate::delete_entry(prompt_key);
}

pub fn export_json(path: &Path) -> std::io::Result<usize> {
    io_ops::export_json(path)
}

pub fn import_json(path: &Path) -> std::io::Result<usize> {
    io_ops::import_json(path)
}
