// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::app::scan::ScannedComponent;
use crate::app::scan::discovery::display_name_from_group_key;
use crate::app::scan::parse::dedup_components;
use crate::app::scan::readme::find_best_readme;
use crate::app::state::{Step2ComponentState, Step2HiddenComponentAudit, Step2ModState};

use self::groups::DerivedCollapsibleGroup;

const NESTED_UTILITY_HIDE_REASON: &str = "nested_other_no_log_record_utility";

#[path = "worker_build_states_groups.rs"]
mod groups;
#[path = "worker_build_states_hidden.rs"]
mod hidden;
#[path = "worker_build_states_meta.rs"]
mod meta;
#[path = "worker_build_states_order.rs"]
mod order;
#[path = "worker_build_states_tp2_blocks.rs"]
mod tp2_blocks;
#[path = "worker_build_states_tra.rs"]
mod tra;

pub(super) fn to_mod_states(
    map: BTreeMap<String, Vec<ScannedComponent>>,
    tp2_map: &BTreeMap<String, String>,
    mods_root: &Path,
) -> Vec<Step2ModState> {
    let mut mods: Vec<Step2ModState> = map
        .into_iter()
        .map(|(group_key, comps)| build_mod_state(&group_key, comps, tp2_map, mods_root))
        .collect();

    mods.retain(|mod_state| !should_drop_hidden_only_utility_mod(mod_state));

    let mut counts: HashMap<String, usize> = HashMap::new();
    for mod_state in &mods {
        *counts
            .entry(mod_state.name.to_ascii_lowercase())
            .or_insert(0) += 1;
    }
    for mod_state in &mut mods {
        if counts
            .get(&mod_state.name.to_ascii_lowercase())
            .copied()
            .unwrap_or(0)
            > 1
            && let Ok(relative) = Path::new(&mod_state.tp2_path).strip_prefix(mods_root)
            && let Some(parent) = relative.parent()
        {
            let rel_parent = parent.to_string_lossy().replace('\\', "/");
            mod_state.name = format!("{} ({})", mod_state.name, rel_parent);
        }
    }

    mods
}

fn build_mod_state(
    group_key: &str,
    comps: Vec<ScannedComponent>,
    tp2_map: &BTreeMap<String, String>,
    mods_root: &Path,
) -> Step2ModState {
    let tp2_path = tp2_map.get(group_key).cloned().unwrap_or_default();
    let display_name = display_name_from_group_key(group_key);
    let readme_path = find_best_readme(mods_root, &tp2_path, &display_name);
    let ini_path = find_best_ini(&tp2_path);
    let tp_file = Path::new(&tp2_path).file_name().map_or_else(
        || display_name.clone(),
        |value| value.to_string_lossy().to_string(),
    );
    let tp2_text = read_tp2_text(&tp2_path);
    let tp2_component_blocks = tp2_text
        .as_deref()
        .map(tp2_blocks::parse_tp2_component_blocks)
        .unwrap_or_default();
    let mut deduped_components = dedup_components(comps);
    if let Some(tp2_text) = tp2_text.as_deref() {
        order::reorder_components_by_tp2_order(&mut deduped_components, &tp2_path, tp2_text);
    }
    let derived_weidu_groups = tp2_text
        .as_deref()
        .map(|tp2_text| groups::detect_weidu_groups(&tp2_path, tp2_text, &deduped_components))
        .unwrap_or_default();
    let derived_collapsible_groups = tp2_text
        .as_deref()
        .map(|tp2_text| {
            groups::detect_derived_collapsible_groups(&tp_file, tp2_text, &deduped_components)
        })
        .unwrap_or_default();
    let hidden_components =
        remove_hidden_components(&mut deduped_components, &tp2_path, tp2_text.as_deref());
    let mod_prompt_summary = mod_prompt_summary(&deduped_components);
    let mod_prompt_events = mod_prompt_events(&deduped_components);
    let meta_mode_component_ids =
        meta::detect_meta_mode_component_ids(&tp2_path, mods_root, tp2_text.as_deref());
    Step2ModState {
        tp_file,
        tp2_path,
        readme_path,
        ini_path,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary,
        mod_prompt_events,
        name: display_name,
        checked: false,
        hidden_components,
        components: build_component_states(
            deduped_components,
            &tp2_component_blocks,
            &derived_weidu_groups,
            &derived_collapsible_groups,
            &meta_mode_component_ids,
        ),
    }
}

fn read_tp2_text(tp2_path: &str) -> Option<String> {
    if tp2_path.trim().is_empty() {
        None
    } else {
        fs::read_to_string(tp2_path).ok()
    }
}

fn remove_hidden_components(
    components: &mut Vec<ScannedComponent>,
    tp2_path: &str,
    tp2_text: Option<&str>,
) -> Vec<Step2HiddenComponentAudit> {
    let hidden_prompt_like_component_ids =
        hidden::detect_hidden_prompt_like_component_ids(Some(tp2_path), tp2_text, components);
    let hidden_components = components
        .iter()
        .filter_map(|component| {
            let component_id = component.component_id.trim().to_string();
            hidden_prompt_like_component_ids
                .get(component_id.as_str())
                .cloned()
                .or_else(|| {
                    order::display_is_blank_version_only(&component.display)
                        .then(|| "blank_version_only_label".to_string())
                })
                .map(|reason| Step2HiddenComponentAudit {
                    component_id,
                    label: component.display.clone(),
                    raw_line: component.raw_line.clone(),
                    reason,
                })
        })
        .collect::<Vec<_>>();
    components.retain(|component| {
        !hidden_prompt_like_component_ids.contains_key(component.component_id.trim())
            && !order::display_is_blank_version_only(&component.display)
    });
    hidden_components
}

fn mod_prompt_summary(components: &[ScannedComponent]) -> Option<String> {
    components
        .iter()
        .filter_map(|component| component.mod_prompt_summary.as_deref())
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn mod_prompt_events(components: &[ScannedComponent]) -> Vec<crate::parser::PromptSummaryEvent> {
    components
        .iter()
        .find_map(|component| {
            (!component.mod_prompt_events.is_empty()).then_some(component.mod_prompt_events.clone())
        })
        .unwrap_or_default()
}

fn build_component_states(
    components: Vec<ScannedComponent>,
    tp2_component_blocks: &HashMap<String, tp2_blocks::Tp2ComponentBlock>,
    derived_weidu_groups: &HashMap<String, String>,
    derived_collapsible_groups: &HashMap<String, DerivedCollapsibleGroup>,
    meta_mode_component_ids: &HashSet<String>,
) -> Vec<Step2ComponentState> {
    components
        .into_iter()
        .map(|component| {
            let component_key = component.component_id.trim();
            let tp2_component_block = tp2_component_blocks.get(component_key);
            let derived_group = derived_collapsible_groups.get(component_key).cloned();
            Step2ComponentState {
                is_meta_mode_component: meta_mode_component_ids.contains(component_key),
                component_id: component.component_id.clone(),
                label: component.display,
                weidu_group: derived_weidu_groups.get(component_key).cloned(),
                subcomponent_key: tp2_component_block
                    .and_then(|block| block.subcomponent_key.clone()),
                tp2_empty_placeholder_block: tp2_component_block
                    .is_some_and(tp2_block_is_empty_placeholder),
                collapsible_group: derived_group.as_ref().map(|group| group.header.clone()),
                collapsible_group_is_umbrella: derived_group
                    .as_ref()
                    .is_some_and(|group| group.is_umbrella),
                raw_line: component.raw_line,
                prompt_summary: component.prompt_summary,
                prompt_events: component.prompt_events,
                disabled: false,
                compat_kind: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_graph: None,
                compat_evidence: None,
                disabled_reason: None,
                checked: false,
                selected_order: None,
            }
        })
        .collect()
}

fn should_drop_hidden_only_utility_mod(mod_state: &Step2ModState) -> bool {
    mod_state.components.is_empty()
        && !mod_state.hidden_components.is_empty()
        && mod_state
            .hidden_components
            .iter()
            .all(|component| component.reason == NESTED_UTILITY_HIDE_REASON)
}

fn tp2_block_is_empty_placeholder(block: &tp2_blocks::Tp2ComponentBlock) -> bool {
    let Some((first, rest)) = block.body_lines.split_first() else {
        return false;
    };
    if !first
        .trim_start()
        .to_ascii_uppercase()
        .starts_with("BEGIN ")
    {
        return false;
    }
    rest.iter().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || trimmed.starts_with("*/")
    })
}

fn find_best_ini(tp2_path: &str) -> Option<String> {
    if tp2_path.trim().is_empty() {
        return None;
    }
    let tp2 = Path::new(tp2_path);
    let dir = tp2.parent()?;
    if let Some(stem) = tp2
        .file_stem()
        .and_then(|value| value.to_str())
        .map(strip_setup_prefix)
        && let Some(path) = existing_ini_path(dir, &stem)
    {
        return Some(path);
    }
    if let Some(folder) = dir.file_name().and_then(|value| value.to_str())
        && let Some(path) = existing_ini_path(dir, folder)
    {
        return Some(path);
    }

    let mut matches = Vec::new();
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        {
            matches.push(path);
        }
    }
    if matches.len() == 1 {
        matches.pop().map(|path| path.display().to_string())
    } else {
        None
    }
}

fn existing_ini_path(dir: &Path, stem: &str) -> Option<String> {
    let stem = stem.trim();
    if stem.is_empty() {
        return None;
    }
    let path = dir.join(format!("{stem}.ini"));
    path.is_file().then(|| path.display().to_string())
}

fn strip_setup_prefix(value: &str) -> String {
    value
        .strip_prefix("setup-")
        .or_else(|| value.strip_prefix("setup_"))
        .unwrap_or(value)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::should_drop_hidden_only_utility_mod;
    use crate::app::state::{Step2HiddenComponentAudit, Step2ModState};

    #[test]
    fn drops_mod_header_when_only_nested_utility_components_are_hidden() {
        let mod_state = Step2ModState {
            name: "EET_modConverter".to_string(),
            tp_file: "EET_modConverter.tp2".to_string(),
            tp2_path: "/mods/EET/other/EET_modConverter/EET_modConverter/EET_modConverter.tp2"
                .to_string(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: vec![Step2HiddenComponentAudit {
                component_id: "0".to_string(),
                label: "EET_modConverter: beta 0.1".to_string(),
                raw_line: "EET_modConverter".to_string(),
                reason: "nested_other_no_log_record_utility".to_string(),
            }],
            components: Vec::new(),
        };

        assert!(should_drop_hidden_only_utility_mod(&mod_state));
    }

    #[test]
    fn keeps_mod_header_when_non_utility_hidden_reason_is_present() {
        let mod_state = Step2ModState {
            name: "SomeMod".to_string(),
            tp_file: "SomeMod.tp2".to_string(),
            tp2_path: "/mods/SomeMod/SomeMod.tp2".to_string(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: vec![Step2HiddenComponentAudit {
                component_id: "100".to_string(),
                label: "dummy".to_string(),
                raw_line: "dummy".to_string(),
                reason: "deprecated_dummy_placeholder".to_string(),
            }],
            components: Vec::new(),
        };

        assert!(!should_drop_hidden_only_utility_mod(&mod_state));
    }
}
