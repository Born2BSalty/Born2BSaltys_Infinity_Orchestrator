// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

use super::auto_answer_context;

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
    let ctx = auto_answer_context::prompt_context(term, prompt_kind.to_string(), "bio_inputs");
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
    term.focus();
    state.step5.last_scripted_skip_signature = None;
    state.step5.last_scripted_fallback_signature = None;
    state.step5.last_scripted_cycle_signature = Some(prompt_cycle_signature.to_string());
    state.step5.last_scripted_send_unix_ms = Some(now_ms);
    state.step5.last_scripted_prompt_key = Some(prompt_key.to_string());
    state.step5.last_status_text = if scripted.is_empty() {
        "Scripted @wlb-input sent: <blank>".to_string()
    } else {
        format!("Scripted @wlb-input sent: {}", scripted)
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
        .map(|t| now_ms.saturating_sub(t) >= post_send_delay_ms)
        .unwrap_or(false);
    let same_prompt_as_last_scripted =
        state.step5.last_scripted_prompt_key.as_deref() == Some(prompt_key);
    timed_out && same_prompt_as_last_scripted
}
