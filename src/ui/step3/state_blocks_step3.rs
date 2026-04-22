// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod cleanup {
    use crate::app::state::Step3ItemState;

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

    pub fn merge_adjacent_same_mod_blocks(
        items: &mut Vec<Step3ItemState>,
        selected: &mut Vec<usize>,
    ) {
        let mut idx = 0usize;
        while idx < items.len() {
            if !items[idx].is_parent {
                idx += 1;
                continue;
            }
            let mut next_parent = None;
            for (j, next_item) in items.iter().enumerate().skip(idx + 1) {
                if next_item.is_parent {
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
}

mod clone_ops {
    use crate::app::state::Step3ItemState;

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
}

mod keys {
    use crate::app::state::Step3ItemState;

    pub fn step3_item_key(item: &Step3ItemState) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            item.tp_file,
            item.mod_name,
            item.component_id,
            item.component_label,
            item.raw_line,
            item.selected_order
        )
    }

    pub(super) fn mod_key(item: &Step3ItemState) -> String {
        format!(
            "{}::{}",
            item.tp_file.to_ascii_uppercase(),
            item.mod_name.to_ascii_uppercase()
        )
    }
}

mod repair {
    use crate::app::state::Step3ItemState;

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
                .find(|i| {
                    i.is_parent
                        && i.mod_name == items[idx].mod_name
                        && i.tp_file == items[idx].tp_file
                })
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
}

mod visibility {
    use crate::app::state::Step3ItemState;

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
            .filter_map(|(i, item)| {
                if item.block_id == block {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }
}

pub use cleanup::{merge_adjacent_same_mod_blocks, prune_empty_parent_blocks};
pub use clone_ops::clone_parent_empty_block;
pub use keys::step3_item_key;
pub use repair::repair_orphan_children;
pub use visibility::{block_indices, count_children_in_block, visible_indices};
