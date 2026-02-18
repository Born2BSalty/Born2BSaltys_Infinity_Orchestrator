// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn prompt_cycle_count(term: &EmbeddedTerminal) -> u64 {
    let upper = term.console_excerpt(12_000).to_ascii_uppercase();
    let input_count = upper.match_indices("USER INPUT REQUIRED").count() as u64;
    if input_count > 0 {
        return input_count;
    }
    upper.match_indices("QUESTION IS").count() as u64
}

pub(super) fn prompt_context(
    term: &EmbeddedTerminal,
    prompt_kind: String,
    source: &str,
) -> prompt_memory::PromptAnswerContext {
    prompt_memory::PromptAnswerContext {
        component_key: term.current_scripted_component_key().unwrap_or_default(),
        tp2_file: term.current_scripted_component_tp2().unwrap_or_default(),
        component_id: term.current_scripted_component_id().unwrap_or_default(),
        component_name: term.current_scripted_component_name().unwrap_or_default(),
        prompt_kind,
        source: source.to_string(),
    }
}
