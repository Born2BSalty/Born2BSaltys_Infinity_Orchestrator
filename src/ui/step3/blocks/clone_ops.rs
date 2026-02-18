// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

use super::visibility::block_indices;

pub fn clone_parent_empty_block(
    items: &mut Vec<Step3ItemState>,
    parent_idx: usize,
    clone_seq: &mut usize,
) {
    let parent = match items.get(parent_idx) {
        Some(p) => p.clone(),
        None => return,
    };
    if !parent.is_parent {
        return;
    }
    let mut clone = parent.clone();
    clone.parent_placeholder = true;
    clone.selected_order = usize::MAX;
    clone.block_id = format!("{}::clone{}", parent.block_id, *clone_seq);
    *clone_seq += 1;
    let insert_at = parent_idx + block_indices(items, parent_idx).len();
    items.insert(insert_at, clone);
}
