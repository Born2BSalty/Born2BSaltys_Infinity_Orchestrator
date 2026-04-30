// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use crate::app::state::{Step2ModState, Step3ItemState};

#[derive(Debug, Clone)]
pub(crate) struct CompatActiveItem {
    pub(crate) tp_file: String,
    pub(crate) mod_name: String,
    pub(crate) tp2_path: String,
    pub(crate) component_id: String,
    pub(crate) order: Option<usize>,
}

pub(crate) fn collect_step2_active_items(mods: &[Step2ModState]) -> Vec<CompatActiveItem> {
    let mut out = Vec::<(usize, CompatActiveItem)>::new();
    let mut discovery_index = 0usize;
    for mod_state in mods {
        for component in mod_state
            .components
            .iter()
            .filter(|component| component.checked && !component.disabled)
        {
            out.push((
                discovery_index,
                CompatActiveItem {
                    tp_file: mod_state.tp_file.clone(),
                    mod_name: mod_state.name.clone(),
                    tp2_path: mod_state.tp2_path.clone(),
                    component_id: component.component_id.clone(),
                    order: component.selected_order,
                },
            ));
            discovery_index += 1;
        }
    }
    out.sort_by_key(|(idx, item)| (item.order.unwrap_or(usize::MAX), *idx));
    out.into_iter()
        .enumerate()
        .map(|(idx, (_discovery_index, mut item))| {
            item.order = Some(idx + 1);
            item
        })
        .collect()
}

pub(crate) fn collect_step3_active_items(
    items: &[Step3ItemState],
    tp2_paths: &HashMap<String, String>,
) -> Vec<CompatActiveItem> {
    let mut out = Vec::<CompatActiveItem>::new();
    for (order, item) in (1usize..).zip(items.iter().filter(|item| !item.is_parent)) {
        let tp2_path = tp2_paths
            .get(&format!(
                "{}|{}",
                item.tp_file.to_ascii_uppercase(),
                item.mod_name.to_ascii_uppercase()
            ))
            .cloned()
            .unwrap_or_default();
        out.push(CompatActiveItem {
            tp_file: item.tp_file.clone(),
            mod_name: item.mod_name.clone(),
            tp2_path,
            component_id: item.component_id.clone(),
            order: Some(order),
        });
    }
    out
}

pub(crate) fn active_item_order(
    active_items: &[CompatActiveItem],
    tp_file: &str,
    component_id: &str,
) -> Option<usize> {
    active_items.iter().find_map(|item| {
        (item.tp_file.trim().eq_ignore_ascii_case(tp_file.trim())
            && item
                .component_id
                .trim()
                .eq_ignore_ascii_case(component_id.trim()))
        .then_some(item.order)
        .flatten()
    })
}
