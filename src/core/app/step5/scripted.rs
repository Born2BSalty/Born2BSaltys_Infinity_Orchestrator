// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;

use super::prompt_memory;

pub(super) fn send_scripted(
    state: &mut WizardState,
    term: &mut EmbeddedTerminal,
    prompt_key: &str,
    preview_line: &str,
    prompt_kind: &str,
    prompt_cycle_signature: &str,
    now_ms: u64,
) -> bool {
    let Some(scripted) = term.take_next_scripted_input_for_current() else {
        return false;
    };
    let ctx = super::auto_answer::prompt_context(term, prompt_kind.to_string(), "bio_inputs");
    prompt_memory::remember_answer_with_context(prompt_key, &scripted, preview_line, &ctx);
    term.send_line(&scripted);
    term.echo_sent(&scripted);
    term.append_marker(&format!(
        "Scripted @wlb-input sent {}",
        if scripted.is_empty() {
            "<blank>"
        } else {
            scripted.as_str()
        }
    ));
    state.step5.last_scripted_skip_signature = None;
    state.step5.last_scripted_fallback_signature = None;
    state.step5.last_scripted_cycle_signature = Some(prompt_cycle_signature.to_string());
    state.step5.last_scripted_send_unix_ms = Some(now_ms);
    state.step5.last_scripted_prompt_key = Some(prompt_key.to_string());
    state.step5.last_status_text = if scripted.is_empty() {
        "Scripted @wlb-input sent: <blank>".to_string()
    } else {
        format!("Scripted @wlb-input sent: {scripted}")
    };
    true
}

pub(super) fn allow_json_fallback_after_scripted(
    state: &WizardState,
    term: &EmbeddedTerminal,
    prompt_key: &str,
    now_ms: u64,
    post_send_delay_ms: u64,
) -> bool {
    if term.scripted_inputs_loaded_count() == 0 {
        return true;
    }
    let timed_out = state
        .step5
        .last_scripted_send_unix_ms
        .is_some_and(|t| now_ms.saturating_sub(t) >= post_send_delay_ms);
    let same_prompt_as_last_scripted =
        state.step5.last_scripted_prompt_key.as_deref() == Some(prompt_key);
    timed_out && same_prompt_as_last_scripted
}

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
                super::auto_answer::prompt_context(term, prompt_kind.to_string(), "json_fallback");
            prompt_memory::remember_answer_with_context(prompt_key, found, preview_line, &ctx);
        }
    }
    let Some(answer) = answer else {
        return false;
    };
    let matched_label = prompt_memory::display_name(prompt_key);
    term.send_line(&answer);
    term.echo_sent(&answer);
    let ctx = super::auto_answer::prompt_context(term, prompt_kind.to_string(), "json_fallback");
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
    state.step5.last_auto_prompt_key = Some(prompt_cycle_signature.to_string());
    let shown = if answer.is_empty() {
        "<blank>"
    } else {
        answer.as_str()
    };
    state.step5.last_status_text = format!("Auto-answered prompt: {shown}");
    true
}
