// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::step5::prompt_memory;

pub(super) fn component_label(entry: &prompt_memory::PromptAnswerEntry) -> String {
    if !entry.component_name.trim().is_empty() {
        entry.component_name.clone()
    } else if !entry.tp2_file.trim().is_empty() && !entry.component_id.trim().is_empty() {
        format!("{} #{}", entry.tp2_file, entry.component_id)
    } else if !entry.component_key.trim().is_empty() {
        entry.component_key.clone()
    } else if !entry.tp2_file.trim().is_empty() {
        entry.tp2_file.clone()
    } else {
        "(unknown component)".to_string()
    }
}
