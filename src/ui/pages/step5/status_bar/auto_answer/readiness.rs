// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

pub(super) fn update_prompt_readiness(
    state: &mut WizardState,
    prompt_signature: &str,
    now_ms: u64,
) {
    if state.step5.prompt_ready_signature.as_deref() != Some(prompt_signature) {
        state.step5.prompt_ready_signature = Some(prompt_signature.to_string());
        state.step5.prompt_ready_seen_count = 1;
        state.step5.prompt_ready_first_seen_unix_ms = Some(now_ms);
        state.step5.last_scripted_fallback_signature = None;
    } else {
        state.step5.prompt_ready_seen_count = state.step5.prompt_ready_seen_count.saturating_add(1);
    }
}

pub(super) fn ready_for_scripted(state: &WizardState, now_ms: u64, debounce_ms: u64) -> bool {
    let stable_polls_ready_scripted = state.step5.prompt_ready_seen_count >= 1;
    let scripted_debounce_ready = state
        .step5
        .prompt_ready_first_seen_unix_ms
        .map(|t| now_ms.saturating_sub(t) >= debounce_ms)
        .unwrap_or(false);
    stable_polls_ready_scripted && scripted_debounce_ready
}

pub(super) fn ready_for_fallback(
    state: &WizardState,
    now_ms: u64,
    headers_ready: bool,
    debounce_ms: u64,
) -> bool {
    let stable_polls_ready_fallback = state.step5.prompt_ready_seen_count >= 2;
    let fallback_debounce_ready = state
        .step5
        .prompt_ready_first_seen_unix_ms
        .map(|t| now_ms.saturating_sub(t) >= debounce_ms)
        .unwrap_or(false);
    headers_ready && stable_polls_ready_fallback && fallback_debounce_ready
}

pub(super) fn settle_active(state: &WizardState, now_ms: u64, post_send_delay_ms: u64) -> bool {
    state
        .step5
        .last_scripted_send_unix_ms
        .map(|t| now_ms.saturating_sub(t) < post_send_delay_ms)
        .unwrap_or(false)
}
