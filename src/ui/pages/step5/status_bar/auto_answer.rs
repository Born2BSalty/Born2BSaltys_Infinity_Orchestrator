// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

mod auto_answer_context;
mod auto_answer_timing;
mod json_fallback;
mod readiness;
mod scripted;

pub(super) fn try_auto_answer_prompt(
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
        // Keep scripted queue debounce stable across streaming prompt block mutations.
        let component_key = term.current_scripted_component_key().unwrap_or_default();
        format!("scripted|{}|{}", component_key, prompt_kind)
    } else {
        format!("{}|{}", prompt_key, prompt_kind)
    };
    let now_ms = auto_answer_timing::now_unix_millis();
    readiness::update_prompt_readiness(state, &prompt_signature, now_ms);
    let headers_ready = term.prompt_headers_ready();
    let scripted_debounce_ms = auto_answer_timing::initial_delay_base_ms(state);
    let fallback_debounce_ms = auto_answer_timing::adaptive_prompt_debounce_ms(
        auto_answer_timing::initial_delay_base_ms(state),
        option_count,
        line_count,
        char_count,
    );
    let ready_for_scripted = readiness::ready_for_scripted(state, now_ms, scripted_debounce_ms);
    let ready_for_fallback =
        readiness::ready_for_fallback(state, now_ms, headers_ready, fallback_debounce_ms);
    if has_scripted_candidate {
        if !ready_for_scripted {
            return;
        }
    } else if !ready_for_fallback {
        return;
    }
    let prompt_cycle = auto_answer_context::prompt_cycle_count(term);
    let prompt_cycle_signature = format!("{prompt_key}|{prompt_cycle}");
    let post_send_delay_ms = auto_answer_timing::post_send_delay_ms(state);
    let settle_active = readiness::settle_active(state, now_ms, post_send_delay_ms);
    if settle_active {
        return;
    }
    if has_scripted_candidate
        && scripted::send_scripted(
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
    let allow_json_fallback = scripted::allow_json_fallback_after_scripted(
        state,
        term,
        &prompt_key,
        now_ms,
        post_send_delay_ms,
    );
    if !allow_json_fallback {
        return;
    }
    let _ = json_fallback::try_send_json_fallback(
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
