// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::ui::scan::ScannedComponent;
use crate::ui::scan::discovery::display_name_from_group_key;
use crate::ui::scan::parse::dedup_components;
use crate::ui::scan::readme::find_best_readme;
use crate::ui::state::{Step2ComponentState, Step2ModState};

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
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| display_name.clone());
            let deduped_components = dedup_components(comps);
            Step2ModState {
                tp_file,
                tp2_path,
                readme_path,
                web_url: None,
                name: display_name,
                checked: false,
                components: deduped_components
                    .into_iter()
                    .map(|component| Step2ComponentState {
                        component_id: component.component_id,
                        label: component.display,
                        raw_line: component.raw_line,
                        disabled: false,
                        compat_kind: None,
                        compat_source: None,
                        compat_related_mod: None,
                        compat_related_component: None,
                        disabled_reason: None,
                        checked: false,
                        selected_order: None,
                    })
                    .collect(),
            }
        })
        .collect();

    // If duplicate names still exist (same mod name in different folders),
    // append relative folder path to disambiguate in UI.
    let mut counts: HashMap<String, usize> = HashMap::new();
    for m in &mods {
        *counts.entry(m.name.to_ascii_lowercase()).or_insert(0) += 1;
    }
    for m in &mut mods {
        if counts.get(&m.name.to_ascii_lowercase()).copied().unwrap_or(0) > 1
            && let Ok(rel) = Path::new(&m.tp2_path).strip_prefix(mods_root)
            && let Some(parent) = rel.parent()
        {
            let rel_parent = parent.to_string_lossy().replace('\\', "/");
            m.name = format!("{} ({})", m.name, rel_parent);
        }
    }

    mods
}
