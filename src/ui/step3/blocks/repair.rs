// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

use super::keys::{mod_key, step3_item_key};

pub fn repair_orphan_children(
    items: &mut Vec<Step3ItemState>,
    selected: &[usize],
    clone_seq: &mut usize,
) {
    let selected_keys: std::collections::HashSet<String> = selected
        .iter()
        .filter_map(|idx| items.get(*idx))
        .filter(|item| !item.is_parent)
        .map(step3_item_key)
        .collect();
    let mut idx = 0usize;
    while idx < items.len() {
        if items[idx].is_parent {
            idx += 1;
            continue;
        }
        let block_id = items[idx].block_id.clone();
        let mut nearest_parent_idx: Option<usize> = None;
        for j in (0..idx).rev() {
            if !items[j].is_parent {
                continue;
            }
            nearest_parent_idx = Some(j);
            break;
        }

        if let Some(pidx) = nearest_parent_idx
            && items[pidx].block_id == block_id
        {
            idx += 1;
            continue;
        }

        if let Some(pidx) = nearest_parent_idx
            && mod_key(&items[pidx]) == mod_key(&items[idx])
        {
            let target_block = items[pidx].block_id.clone();
            let mut j = idx;
            let mut moved_any = false;
            while j < items.len() {
                if items[j].is_parent || items[j].block_id != block_id {
                    break;
                }
                if !selected_keys.contains(&step3_item_key(&items[j])) {
                    break;
                }
                items[j].block_id = target_block.clone();
                moved_any = true;
                j += 1;
            }
            idx = if moved_any { j } else { idx + 1 };
            continue;
        }

        let parent_template = items
            .iter()
            .find(|i| i.is_parent && i.mod_name == items[idx].mod_name && i.tp_file == items[idx].tp_file)
            .cloned();
        if let Some(mut parent) = parent_template {
            parent.parent_placeholder = true;
            parent.block_id = format!("{}::split{}", parent.block_id, *clone_seq);
            parent.selected_order = usize::MAX;
            let new_block = parent.block_id.clone();
            *clone_seq += 1;
            items.insert(idx, parent);
            let mut j = idx + 1;
            while j < items.len() {
                if items[j].is_parent || items[j].block_id != block_id {
                    break;
                }
                if !selected_keys.contains(&step3_item_key(&items[j])) {
                    break;
                }
                items[j].block_id = new_block.clone();
                j += 1;
            }
            idx = j;
        } else {
            idx += 1;
        }
    }
}
