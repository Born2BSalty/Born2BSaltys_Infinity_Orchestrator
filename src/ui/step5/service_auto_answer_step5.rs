// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::step5::prompt_memory;
use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn try_auto_answer_prompt(
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    waiting_for_input: bool,
) {
    if state.step5.cancel_requested || state.step5.cancel_pending || state.step5.cancel_confirm_open {
        reset_auto_answer_state(state);
        return;
    }
    if !waiting_for_input {
        reset_auto_answer_state(state);
        return;
    }
    if !state.step5.input_line.trim().is_empty() {
        return;
    }
    let Some(term) = terminal else {
        return;
    };
    let Some(prompt_info) = term.current_prompt_info() else {
        return;
    };
    let prompt_key = prompt_info.key.clone();
    let preview_line = prompt_info.preview_line.clone();
    let legacy_key = prompt_info.legacy_key.clone();
    let option_count = prompt_info.option_count;
    let line_count = prompt_info.line_count;
    let char_count = prompt_info.char_count;
    let has_scripted_candidate = term.peek_next_scripted_input_for_current().is_some();
    let prompt_kind = term.prompt_kind_name(&prompt_info).to_string();
    let prompt_signature = if has_scripted_candidate {
        let component_key = term.current_scripted_component_key().unwrap_or_default();
        format!("scripted|{}|{}", component_key, prompt_kind)
    } else {
        format!("{}|{}", prompt_key, prompt_kind)
    };
    let now_ms = now_unix_millis();
    crate::ui::step5::service_readiness_step5::update_prompt_readiness(
        state,
        &prompt_signature,
        now_ms,
    );
    let headers_ready = term.prompt_headers_ready();
    let scripted_debounce_ms = initial_delay_base_ms(state);
    let fallback_debounce_ms = adaptive_prompt_debounce_ms(
        initial_delay_base_ms(state),
        option_count,
        line_count,
        char_count,
    );
    let ready_for_scripted = crate::ui::step5::service_readiness_step5::ready_for_scripted(
        state,
        now_ms,
        scripted_debounce_ms,
    );
    let ready_for_fallback = crate::ui::step5::service_readiness_step5::ready_for_fallback(
        state,
        now_ms,
        headers_ready,
        fallback_debounce_ms,
    );
    if has_scripted_candidate {
        if !ready_for_scripted {
            return;
        }
    } else if !ready_for_fallback {
        return;
    }
    let prompt_cycle = prompt_cycle_count(term);
    let prompt_cycle_signature = format!("{prompt_key}|{prompt_cycle}");
    let post_send_delay_ms = post_send_delay_ms(state);
    let settle_active = crate::ui::step5::service_readiness_step5::settle_active(
        state,
        now_ms,
        post_send_delay_ms,
    );
    if settle_active {
        return;
    }
    if has_scripted_candidate
        && crate::ui::step5::service_scripted_step5::send_scripted(
            state,
            term,
            &prompt_key,
            &preview_line,
            &prompt_kind,
            &prompt_cycle_signature,
            now_ms,
        )
    {
        return;
    }
    let allow_json_fallback =
        crate::ui::step5::service_scripted_step5::allow_json_fallback_after_scripted(
        state,
        term,
        &prompt_key,
        now_ms,
        post_send_delay_ms,
    );
    if !allow_json_fallback {
        return;
    }
    let _ = crate::ui::step5::service_json_fallback_step5::try_send_json_fallback(
        state,
        term,
        &prompt_key,
        legacy_key.as_deref(),
        &preview_line,
        &prompt_kind,
        &prompt_cycle_signature,
    );
}

fn reset_auto_answer_state(state: &mut WizardState) {
    state.step5.last_auto_prompt_key = None;
    state.step5.last_scripted_skip_signature = None;
    state.step5.last_scripted_fallback_signature = None;
    state.step5.last_scripted_cycle_signature = None;
    state.step5.last_scripted_send_unix_ms = None;
    state.step5.last_scripted_prompt_key = None;
    state.step5.paused_scripted_component_key = None;
    state.step5.prompt_ready_signature = None;
    state.step5.prompt_ready_seen_count = 0;
    state.step5.prompt_ready_first_seen_unix_ms = None;
}

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

pub(super) fn now_unix_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(super) fn adaptive_prompt_debounce_ms(
    base_delay_ms: u64,
    option_count: usize,
    line_count: usize,
    char_count: usize,
) -> u64 {
    let mut delay = base_delay_ms;
    if option_count > 500 || line_count > 700 || char_count > 40_000 {
        delay += 3000;
    } else if option_count > 200 || line_count > 300 || char_count > 18_000 {
        delay += 2000;
    } else if option_count > 50 || line_count > 120 || char_count > 8_000 {
        delay += 1000;
    }
    delay
}

pub(super) fn initial_delay_base_ms(state: &WizardState) -> u64 {
    if state.step1.auto_answer_initial_delay_enabled {
        state.step1.auto_answer_initial_delay_ms.clamp(500, 15_000) as u64
    } else {
        2000
    }
}

pub(super) fn post_send_delay_ms(state: &WizardState) -> u64 {
    if state.step1.auto_answer_post_send_delay_enabled {
        state.step1.auto_answer_post_send_delay_ms.clamp(500, 15_000) as u64
    } else {
        5000
    }
}
