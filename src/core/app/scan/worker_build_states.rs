// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::ui::scan::discovery::display_name_from_group_key;
use crate::ui::scan::parse::dedup_components;
use crate::ui::scan::readme::find_best_readme;
use crate::ui::scan::ScannedComponent;
use crate::ui::state::{Step2ComponentState, Step2HiddenComponentAudit, Step2ModState};

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
    tp2_map: BTreeMap<String, String>,
    mods_root: &Path,
) -> Vec<Step2ModState> {
    let mut mods: Vec<Step2ModState> = map
        .into_iter()
        .map(|(group_key, comps)| {
            let tp2_path = tp2_map.get(&group_key).cloned().unwrap_or_default();
            let display_name = display_name_from_group_key(&group_key);
            let readme_path = find_best_readme(mods_root, &tp2_path, &display_name);
            let tp_file = Path::new(&tp2_path)
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| display_name.clone());
            let tp2_text = if tp2_path.trim().is_empty() {
                None
            } else {
                fs::read_to_string(&tp2_path).ok()
            };
            let mut deduped_components = dedup_components(comps);
            if let Some(tp2_text) = tp2_text.as_deref() {
                order::reorder_components_by_tp2_order(
                    &mut deduped_components,
                    &tp2_path,
                    tp2_text,
                );
            }
            let derived_weidu_groups = tp2_text
                .as_deref()
                .map(|tp2_text| {
                    groups::detect_weidu_groups(&tp2_path, tp2_text, &deduped_components)
                })
                .unwrap_or_default();
            let derived_collapsible_groups = tp2_text
                .as_deref()
                .map(|tp2_text| {
                    groups::detect_derived_collapsible_groups(
                        &tp_file,
                        tp2_text,
                        &deduped_components,
                    )
                })
                .unwrap_or_default();
            let hidden_prompt_like_component_ids =
                hidden::detect_hidden_prompt_like_component_ids(
                    Some(&tp2_path),
                    tp2_text.as_deref(),
                    &deduped_components,
                );
            let hidden_components = deduped_components
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
            deduped_components.retain(|component| {
                !hidden_prompt_like_component_ids.contains_key(component.component_id.trim())
                    && !order::display_is_blank_version_only(&component.display)
            });
            let mod_prompt_summary = deduped_components
                .iter()
                .filter_map(|component| component.mod_prompt_summary.as_deref())
                .map(str::trim)
                .find(|value| !value.is_empty())
                .map(ToString::to_string);
            let mod_prompt_events = deduped_components
                .iter()
                .find_map(|component| {
                    (!component.mod_prompt_events.is_empty())
                        .then_some(component.mod_prompt_events.clone())
                })
                .unwrap_or_default();
            let meta_mode_component_ids =
                meta::detect_meta_mode_component_ids(&tp2_path, mods_root, tp2_text.as_deref());
            Step2ModState {
                tp_file,
                tp2_path,
                readme_path,
                web_url: None,
                mod_prompt_summary,
                mod_prompt_events,
                name: display_name,
                checked: false,
                hidden_components,
                components: deduped_components
                    .into_iter()
                    .map(|component| {
                        let derived_group = derived_collapsible_groups
                            .get(component.component_id.trim())
                            .cloned();
                        Step2ComponentState {
                            is_meta_mode_component: meta_mode_component_ids
                                .contains(component.component_id.trim()),
                            component_id: component.component_id.clone(),
                            label: component.display,
                            weidu_group: derived_weidu_groups
                                .get(component.component_id.trim())
                                .cloned(),
                            collapsible_group: derived_group
                                .as_ref()
                                .map(|group| group.header.clone()),
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
                    .collect(),
            }
        })
        .collect();

    mods.retain(|mod_state| !should_drop_hidden_only_utility_mod(mod_state));

    let mut counts: HashMap<String, usize> = HashMap::new();
    for mod_state in &mods {
        *counts.entry(mod_state.name.to_ascii_lowercase()).or_insert(0) += 1;
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

fn should_drop_hidden_only_utility_mod(mod_state: &Step2ModState) -> bool {
    mod_state.components.is_empty()
        && !mod_state.hidden_components.is_empty()
        && mod_state
            .hidden_components
            .iter()
            .all(|component| component.reason == NESTED_UTILITY_HIDE_REASON)
}

#[cfg(test)]
mod tests {
    use super::should_drop_hidden_only_utility_mod;
    use crate::ui::state::{Step2HiddenComponentAudit, Step2ModState};

    #[test]
    fn drops_mod_header_when_only_nested_utility_components_are_hidden() {
        let mod_state = Step2ModState {
            name: "EET_modConverter".to_string(),
            tp_file: "EET_modConverter.tp2".to_string(),
            tp2_path: "/mods/EET/other/EET_modConverter/EET_modConverter/EET_modConverter.tp2"
                .to_string(),
            readme_path: None,
            web_url: None,
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
            web_url: None,
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
