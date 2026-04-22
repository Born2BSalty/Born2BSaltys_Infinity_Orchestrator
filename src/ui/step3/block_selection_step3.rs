// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::app::state::Step3ItemState;
use crate::ui::step3::blocks;

pub(crate) fn single_child_main_parent_block_indices(
    items: &[Step3ItemState],
    idx: usize,
) -> Option<Vec<usize>> {
    let item = items.get(idx)?;
    if item.is_parent {
        return None;
    }
    let parent_idx = items
        .iter()
        .position(|it| it.is_parent && it.block_id == item.block_id)?;
    let parent = items.get(parent_idx)?;
    (!parent.parent_placeholder && blocks::count_children_in_block(items, parent_idx) == 1)
        .then(|| blocks::block_indices(items, parent_idx))
}

pub(crate) fn selected_full_main_parent_block_indices(
    items: &[Step3ItemState],
    selected: &[usize],
    idx: usize,
) -> Option<Vec<usize>> {
    let item = items.get(idx)?;
    if !selected.contains(&idx) {
        return None;
    }
    if item.is_parent {
        let selected_blocks: HashSet<&str> = selected
            .iter()
            .filter_map(|selected_idx| items.get(*selected_idx))
            .map(|selected_item| selected_item.block_id.as_str())
            .collect();
        return (selected_blocks.len() > 1).then(|| {
            items
                .iter()
                .enumerate()
                .filter_map(|(row_idx, row_item)| {
                    selected_blocks
                        .contains(row_item.block_id.as_str())
                        .then_some(row_idx)
                })
                .collect()
        });
    }
    let parent_idx = items
        .iter()
        .position(|it| it.is_parent && it.block_id == item.block_id)?;
    let parent = items.get(parent_idx)?;
    if parent.parent_placeholder {
        return None;
    }
    let child_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(row_idx, row_item)| {
            (!row_item.is_parent && row_item.block_id == item.block_id).then_some(row_idx)
        })
        .collect();
    if child_indices.is_empty() {
        return None;
    }
    let selected_non_parent: HashSet<usize> = selected
        .iter()
        .copied()
        .filter(|selected_idx| {
            items
                .get(*selected_idx)
                .is_some_and(|selected_item| !selected_item.is_parent)
        })
        .collect();
    if selected_non_parent.len() != child_indices.len() {
        return None;
    }
    if !child_indices
        .iter()
        .all(|child_idx| selected_non_parent.contains(child_idx))
    {
        return None;
    }
    Some(blocks::block_indices(items, parent_idx))
}
