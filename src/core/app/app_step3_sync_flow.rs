// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use crate::ui::controller::step3_sync::{build_step3_items, collect_parent_block_ids};
use crate::ui::state::Step3ItemState;

use super::WizardApp;

pub(super) fn sync_step3_from_step2(app: &mut WizardApp) {
    let bgee_fresh = build_step3_items(&app.state.step2.bgee_mods);
    let bg2ee_fresh = build_step3_items(&app.state.step2.bg2ee_mods);

    app.state.step3.bgee_items = reconcile_step3_items(&app.state.step3.bgee_items, bgee_fresh);
    app.state.step3.bg2ee_items = reconcile_step3_items(&app.state.step3.bg2ee_items, bg2ee_fresh);
    app.state.step3.bgee_collapsed_blocks = collect_parent_block_ids(&app.state.step3.bgee_items);
    app.state.step3.bg2ee_collapsed_blocks = collect_parent_block_ids(&app.state.step3.bg2ee_items);
    app.state.step3.bgee_clone_seq = 1;
    app.state.step3.bg2ee_clone_seq = 1;
    app.state.step3.bgee_selected.clear();
    app.state.step3.bg2ee_selected.clear();
    app.state.step3.bgee_drag_from = None;
    app.state.step3.bg2ee_drag_from = None;
    app.state.step3.bgee_drag_over = None;
    app.state.step3.bg2ee_drag_over = None;
    app.state.step3.bgee_drag_indices.clear();
    app.state.step3.bg2ee_drag_indices.clear();
    app.state.step3.bgee_anchor = None;
    app.state.step3.bg2ee_anchor = None;
    app.state.step3.jump_to_selected_requested = false;
    app.state.step3.compat_modal_open = false;

    super::tp2_metadata::refresh_validator_tp2_metadata(app);
    app.revalidate_compat();
}

fn reconcile_step3_items(
    current: &[Step3ItemState],
    fresh: Vec<Step3ItemState>,
) -> Vec<Step3ItemState> {
    let mut fresh_by_key = HashMap::<String, Step3ItemState>::new();
    let mut fresh_order = Vec::<String>::new();
    for item in fresh.into_iter().filter(|item| !item.is_parent) {
        let key = child_key(&item);
        fresh_order.push(key.clone());
        fresh_by_key.insert(key, item);
    }

    let mut ordered_children = Vec::<Step3ItemState>::new();
    for item in current.iter().filter(|item| !item.is_parent) {
        let key = child_key(item);
        if let Some(fresh_item) = fresh_by_key.remove(&key) {
            ordered_children.push(fresh_item);
        }
    }
    for key in fresh_order {
        if let Some(fresh_item) = fresh_by_key.remove(&key) {
            ordered_children.push(fresh_item);
        }
    }

    rebuild_parent_blocks(ordered_children)
}

fn rebuild_parent_blocks(children: Vec<Step3ItemState>) -> Vec<Step3ItemState> {
    let mut out = Vec::<Step3ItemState>::new();
    let mut block_seq = 0usize;
    let mut last_parent_key = String::new();
    let mut current_block_id = String::new();

    for mut child in children {
        let next_parent_key = parent_key(&child);
        if next_parent_key != last_parent_key {
            block_seq += 1;
            current_block_id = format!("{next_parent_key}::segment{block_seq}");
            out.push(Step3ItemState {
                tp_file: child.tp_file.clone(),
                component_id: "__PARENT__".to_string(),
                mod_name: child.mod_name.clone(),
                component_label: child.mod_name.clone(),
                raw_line: String::new(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                selected_order: child.selected_order,
                block_id: current_block_id.clone(),
                is_parent: true,
                parent_placeholder: false,
            });
            last_parent_key = next_parent_key;
        }

        child.block_id = current_block_id.clone();
        child.is_parent = false;
        child.parent_placeholder = false;
        out.push(child);
    }

    out
}

fn child_key(item: &Step3ItemState) -> String {
    format!(
        "{}|{}|{}|{}",
        item.tp_file.to_ascii_uppercase(),
        item.component_id,
        item.mod_name.to_ascii_uppercase(),
        item.raw_line
    )
}

fn parent_key(item: &Step3ItemState) -> String {
    format!(
        "{}::{}",
        item.tp_file.to_ascii_uppercase(),
        item.mod_name.to_ascii_uppercase()
    )
}
