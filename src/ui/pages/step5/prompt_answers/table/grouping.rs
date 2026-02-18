// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use crate::ui::step5::prompt_memory;

use super::labels::component_label;

#[derive(Debug)]
pub(super) struct PromptGroup {
    pub(super) label: String,
    pub(super) items: Vec<(String, prompt_memory::PromptAnswerEntry)>,
}

pub(super) fn group_entries(
    entries: Vec<(String, prompt_memory::PromptAnswerEntry)>,
) -> Vec<PromptGroup> {
    let mut grouped: BTreeMap<String, Vec<(String, prompt_memory::PromptAnswerEntry)>> =
        BTreeMap::new();
    for (key, entry) in entries {
        let group_label = component_label(&entry);
        grouped.entry(group_label).or_default().push((key, entry));
    }
    grouped
        .into_iter()
        .map(|(label, items)| PromptGroup { label, items })
        .collect()
}
