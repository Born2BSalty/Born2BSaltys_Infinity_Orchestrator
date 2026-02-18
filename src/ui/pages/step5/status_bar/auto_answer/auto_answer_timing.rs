// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

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
