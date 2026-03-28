// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::step5::prompt_memory::PromptAnswerEntry;

#[derive(Debug)]
pub(crate) struct PromptGroup {
    pub label: String,
    pub items: Vec<(String, PromptAnswerEntry)>,
}

pub(crate) fn group_prompt_entries(entries: Vec<(String, PromptAnswerEntry)>) -> Vec<PromptGroup> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<String, Vec<(String, PromptAnswerEntry)>> = BTreeMap::new();
    for (key, entry) in entries {
        let group_label = prompt_component_label(&entry);
        grouped.entry(group_label).or_default().push((key, entry));
    }
    grouped
        .into_iter()
        .map(|(label, items)| PromptGroup { label, items })
        .collect()
}

fn prompt_component_label(entry: &PromptAnswerEntry) -> String {
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
