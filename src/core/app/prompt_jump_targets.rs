// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step2ModState;

pub(crate) fn collect_prompt_jump_component_ids(
    mods: &[Step2ModState],
    title: &str,
    text: &str,
) -> Vec<u32> {
    let mut ids = parse_prompt_jump_component_ids(text);
    let mod_ref = prompt_popup_mod_ref(title);
    let target_mod_key = normalize_mod_key(&mod_ref);
    for mod_state in mods {
        if normalize_mod_key(&mod_state.tp_file) != target_mod_key {
            continue;
        }
        for component in &mod_state.components {
            let has_prompt = component
                .prompt_summary
                .as_ref()
                .map(|summary| !summary.trim().is_empty())
                .unwrap_or(false)
                || !component.prompt_events.is_empty();
            if !has_prompt {
                continue;
            }
            if let Ok(id) = component.component_id.trim().parse::<u32>()
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
    }
    ids.sort_unstable();
    ids
}

pub(crate) fn prompt_popup_mod_ref(title: &str) -> String {
    title
        .split(" #")
        .next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| title.trim().to_string())
}

fn parse_prompt_jump_component_ids(text: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Component:") else {
            continue;
        };
        let id_token = rest.split_whitespace().next().unwrap_or_default();
        if let Ok(id) = id_token.parse::<u32>()
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}

fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}
