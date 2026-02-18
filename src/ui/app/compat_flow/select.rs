// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::compat::SelectedComponent;
use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{Step2ModState, Step3ItemState};

pub(super) fn build_selected_components(items: &[Step3ItemState]) -> Vec<SelectedComponent> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.is_parent)
        .map(|(idx, item)| SelectedComponent {
            mod_name: item.mod_name.clone(),
            tp_file: effective_tp2_for_row(&item.tp_file, &item.raw_line),
            component_id: item.component_id.parse().unwrap_or(0),
            order: idx,
        })
        .collect()
}

pub(super) fn build_selected_components_from_step2(mods: &[Step2ModState]) -> Vec<SelectedComponent> {
    #[derive(Clone)]
    struct SelectedRow {
        mod_name: String,
        tp_file: String,
        component_id: u32,
        selected_order: usize,
    }

    let mut rows: Vec<SelectedRow> = Vec::new();
    for mod_state in mods {
        for component in mod_state.components.iter().filter(|c| c.checked) {
            rows.push(SelectedRow {
                mod_name: mod_state.name.clone(),
                tp_file: effective_tp2_for_row(&mod_state.tp_file, &component.raw_line),
                component_id: component.component_id.parse().unwrap_or(0),
                selected_order: component.selected_order.unwrap_or(usize::MAX),
            });
        }
    }
    rows.sort_by_key(|r| r.selected_order);
    rows.into_iter()
        .enumerate()
        .map(|(order, row)| SelectedComponent {
            mod_name: row.mod_name,
            tp_file: row.tp_file,
            component_id: row.component_id,
            order,
        })
        .collect()
}

fn effective_tp2_for_row(default_tp_file: &str, raw_line: &str) -> String {
    parse_component_tp2_from_raw(raw_line).unwrap_or_else(|| default_tp_file.to_string())
}
