// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{Duration, Instant};

use crate::ui::settings::state_settings::SettingsScreenState;

const PATH_VALIDATION_DEBOUNCE: Duration = Duration::from_millis(500);

pub fn mark_dirty(state: &mut SettingsScreenState, field: &'static str, now: Instant) {
    state.path_edit_debounce.insert(field, now);
}

pub fn tick(state: &mut SettingsScreenState, now: Instant) -> bool {
    let ready = state
        .path_edit_debounce
        .values()
        .any(|dirty_at| now.duration_since(*dirty_at) >= PATH_VALIDATION_DEBOUNCE);

    if ready {
        state.path_edit_debounce.clear();
    }

    ready
}
