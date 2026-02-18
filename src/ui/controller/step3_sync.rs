// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{Step1State, Step3ItemState};

pub fn build_step3_items(mods: &[crate::ui::state::Step2ModState]) -> Vec<Step3ItemState> {
    #[derive(Clone)]
    struct SelectedComponent {
        tp_file: String,
        mod_name: String,
        component_id: String,
        component_label: String,
        raw_line: String,
        selected_order: usize,
    }

    let mut ordered: Vec<SelectedComponent> = Vec::new();
    for mod_state in mods {
        for component in mod_state.components.iter().filter(|c| c.checked) {
            ordered.push(SelectedComponent {
                tp_file: mod_state.tp_file.clone(),
                mod_name: mod_state.name.clone(),
                component_id: component.component_id.clone(),
                component_label: component.label.clone(),
                raw_line: component.raw_line.clone(),
                selected_order: component.selected_order.unwrap_or(usize::MAX),
            });
        }
    }
    ordered.sort_by_key(|c| c.selected_order);

    let mut out = Vec::new();
    let mut block_seq = 0usize;
    let mut last_parent_key = String::new();
    let mut current_block_id = String::new();
    for component in ordered {
        let parent_key = format!(
            "{}::{}",
            component.tp_file.to_ascii_uppercase(),
            component.mod_name.to_ascii_uppercase()
        );
        let needs_new_parent = parent_key != last_parent_key;
        if needs_new_parent {
            block_seq += 1;
            let block_id = format!("{parent_key}::segment{block_seq}");
            out.push(Step3ItemState {
                tp_file: component.tp_file.clone(),
                component_id: "__PARENT__".to_string(),
                mod_name: component.mod_name.clone(),
                component_label: component.mod_name.clone(),
                raw_line: String::new(),
                selected_order: component.selected_order,
                block_id: block_id.clone(),
                is_parent: true,
                parent_placeholder: false,
            });
            last_parent_key = parent_key;
            current_block_id = block_id;
        }
        out.push(Step3ItemState {
            tp_file: component.tp_file,
            component_id: component.component_id,
            mod_name: component.mod_name,
            component_label: component.component_label,
            raw_line: component.raw_line,
            selected_order: component.selected_order,
            block_id: current_block_id.clone(),
            is_parent: false,
            parent_placeholder: false,
        });
    }
    out
}

pub fn collect_parent_block_ids(items: &[Step3ItemState]) -> Vec<String> {
    let mut out = Vec::new();
    for item in items.iter().filter(|i| i.is_parent) {
        if !out.contains(&item.block_id) {
            out.push(item.block_id.clone());
        }
    }
    out
}

pub fn scrub_dev_settings(step1: &mut Step1State) {
    step1.bio_full_debug = false;
    step1.tick_dev_enabled = false;
    step1.log_raw_output_dev = false;
}
