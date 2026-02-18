// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

pub fn visible_indices(items: &[Step3ItemState], collapsed_blocks: &[String]) -> Vec<usize> {
    let mut out = Vec::with_capacity(items.len());
    for (idx, item) in items.iter().enumerate() {
        if item.is_parent {
            out.push(idx);
            continue;
        }
        if collapsed_blocks.contains(&item.block_id) {
            continue;
        }
        out.push(idx);
    }
    out
}

pub fn count_children_in_block(items: &[Step3ItemState], parent_idx: usize) -> usize {
    let block = items[parent_idx].block_id.as_str();
    items
        .iter()
        .filter(|i| !i.is_parent && i.block_id == block)
        .count()
}

pub fn block_indices(items: &[Step3ItemState], parent_idx: usize) -> Vec<usize> {
    let block = items[parent_idx].block_id.as_str();
    items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| if item.block_id == block { Some(i) } else { None })
        .collect()
}
