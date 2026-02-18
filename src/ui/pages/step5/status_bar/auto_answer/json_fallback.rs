// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

use super::auto_answer_context;

pub(super) fn try_send_json_fallback(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    prompt_key: &str,
    legacy_key: Option<&str>,
    preview_line: &str,
    prompt_kind: &str,
    prompt_cycle_signature: &str,
) -> bool {
    if state.step5.last_auto_prompt_key.as_deref() == Some(prompt_cycle_signature) {
        return false;
    }
    let alias = prompt_memory::alias_from_preview(preview_line);
    let mut answer = prompt_memory::get_answer_by_alias(&alias);
    if answer.is_some() {
        prompt_memory::attach_key_to_alias(&alias, prompt_key, preview_line);
    }
    if answer.is_none() {
        answer = prompt_memory::get_answer(prompt_key);
    }
    if answer.is_none()
        && let Some(legacy_key) = legacy_key
    {
        answer = prompt_memory::get_answer(legacy_key);
        if let Some(found) = answer.as_deref() {
            let ctx =
                auto_answer_context::prompt_context(term, prompt_kind.to_string(), "json_fallback");
            prompt_memory::remember_answer_with_context(prompt_key, found, preview_line, &ctx);
        }
    }
    let Some(answer) = answer else {
        return false;
    };
    let matched_label = prompt_memory::display_name(prompt_key);
    term.send_line(&answer);
    term.echo_sent(&answer);
    let ctx = auto_answer_context::prompt_context(term, prompt_kind.to_string(), "json_fallback");
    prompt_memory::remember_answer_with_context(prompt_key, &answer, preview_line, &ctx);
    term.append_marker(&format!(
        "Auto-answer matched {} and sent {}",
        matched_label,
        if answer.is_empty() {
            "<blank>"
        } else {
            answer.as_str()
        }
    ));
    term.focus();
    state.step5.last_auto_prompt_key = Some(prompt_cycle_signature.to_string());
    let shown = if answer.is_empty() {
        "<blank>"
    } else {
        answer.as_str()
    };
    state.step5.last_status_text = format!("Auto-answered prompt: {shown}");
    true
}
