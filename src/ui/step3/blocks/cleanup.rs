// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

use super::keys::mod_key;

pub fn prune_empty_parent_blocks(items: &mut Vec<Step3ItemState>, selected: &mut Vec<usize>) {
    let mut remove_indices: Vec<usize> = Vec::new();
    for (idx, item) in items.iter().enumerate() {
        if !item.is_parent || !item.parent_placeholder {
            continue;
        }
        let has_child = items
            .iter()
            .enumerate()
            .any(|(j, c)| j != idx && !c.is_parent && c.block_id == item.block_id);
        if !has_child {
            remove_indices.push(idx);
        }
    }
    if remove_indices.is_empty() {
        return;
    }
    remove_indices.sort_unstable_by(|a, b| b.cmp(a));
    for idx in remove_indices {
        remove_row_and_fix_selection(items, selected, idx);
    }
}

pub fn merge_adjacent_same_mod_blocks(items: &mut Vec<Step3ItemState>, selected: &mut Vec<usize>) {
    let mut idx = 0usize;
    while idx < items.len() {
        if !items[idx].is_parent {
            idx += 1;
            continue;
        }
        let mut next_parent = None;
        for j in idx + 1..items.len() {
            if items[j].is_parent {
                next_parent = Some(j);
                break;
            }
        }
        let Some(next_idx) = next_parent else {
            break;
        };
        if mod_key(&items[idx]) != mod_key(&items[next_idx]) {
            idx = next_idx;
            continue;
        }

        if items[idx].parent_placeholder && !items[next_idx].parent_placeholder {
            items[idx].parent_placeholder = false;
        }
        let keep_block = items[idx].block_id.clone();
        let drop_block = items[next_idx].block_id.clone();
        for item in items.iter_mut() {
            if !item.is_parent && item.block_id == drop_block {
                item.block_id = keep_block.clone();
            }
        }
        remove_row_and_fix_selection(items, selected, next_idx);
    }
}

pub(super) fn remove_row_and_fix_selection(
    items: &mut Vec<Step3ItemState>,
    selected: &mut Vec<usize>,
    idx: usize,
) {
    if idx >= items.len() {
        return;
    }
    items.remove(idx);
    selected.retain(|s| *s != idx);
    for s in selected.iter_mut() {
        if *s > idx {
            *s -= 1;
        }
    }
}
