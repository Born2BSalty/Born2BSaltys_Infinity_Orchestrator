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
        let mut unchecked_ids = Vec::<u32>::new();
        for component in &mod_state.components {
            let Ok(id) = component.component_id.trim().parse::<u32>() else {
                continue;
            };
            if !component.checked {
                unchecked_ids.push(id);
                continue;
            }
            let has_prompt = component
                .prompt_summary
                .as_ref()
                .is_some_and(|summary| !summary.trim().is_empty())
                || !component.prompt_events.is_empty();
            if has_prompt && !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids.retain(|id| !unchecked_ids.contains(id));
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
    let file = lower
        .rfind(['/', '\\'])
        .map_or(lower.as_str(), |idx| &lower[idx + 1..]);
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step2ComponentState;

    fn component(id: &str, checked: bool, prompt: Option<&str>) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: id.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: prompt.map(str::to_string),
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked,
            selected_order: None,
        }
    }

    fn mod_with(components: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: "Mod".to_string(),
            tp_file: "mod.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components,
        }
    }

    #[test]
    fn jump_ids_include_only_ticked_components_with_prompts() {
        let mods = vec![mod_with(vec![
            component("1", true, Some("prompt")),
            component("2", false, Some("prompt")),
            component("3", true, None),
        ])];
        assert_eq!(
            collect_prompt_jump_component_ids(&mods, "mod.tp2", ""),
            vec![1]
        );
    }

    #[test]
    fn jump_ids_parsed_from_text_drop_unticked_components() {
        let mods = vec![mod_with(vec![
            component("1", true, Some("prompt")),
            component("2", false, Some("prompt")),
        ])];
        let text = "Component: 1 first\nComponent: 2 second\nComponent: 9 unknown";
        assert_eq!(
            collect_prompt_jump_component_ids(&mods, "mod.tp2", text),
            vec![1, 9]
        );
    }
}
