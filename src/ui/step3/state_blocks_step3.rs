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
                    item.block_id.clone_from(&keep_block);
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

    #[must_use]
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

    #[must_use]
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

    use super::keys::mod_key;

    pub fn repair_orphan_children(items: &mut Vec<Step3ItemState>, clone_seq: &mut usize) {
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
                while j < items.len() {
                    if items[j].is_parent || items[j].block_id != block_id {
                        break;
                    }
                    items[j].block_id.clone_from(&target_block);
                    j += 1;
                }
                idx = j;
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
                    items[j].block_id.clone_from(&new_block);
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

    #[must_use]
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

    #[must_use]
    pub fn count_children_in_block(items: &[Step3ItemState], parent_idx: usize) -> usize {
        let block = items[parent_idx].block_id.as_str();
        items
            .iter()
            .filter(|i| !i.is_parent && i.block_id == block)
            .count()
    }

    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::{
        count_children_in_block, merge_adjacent_same_mod_blocks, prune_empty_parent_blocks,
        repair_orphan_children,
    };
    use crate::app::state::Step3ItemState;

    fn row(mod_name: &str, component_id: &str, is_parent: bool, block_id: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: component_id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: block_id.to_string(),
            is_parent,
            parent_placeholder: is_parent && block_id.contains("::split"),
        }
    }

    fn assert_every_component_sits_under_its_own_header(items: &[Step3ItemState]) {
        for (idx, item) in items.iter().enumerate() {
            if item.is_parent {
                continue;
            }
            let nearest_parent = items[..idx].iter().rev().find(|i| i.is_parent);
            let parent = nearest_parent
                .unwrap_or_else(|| panic!("component at {idx} has no header above it"));
            assert_eq!(parent.block_id, item.block_id);
            assert_eq!(parent.mod_name, item.mod_name);
        }
    }

    fn tidy(items: &mut Vec<Step3ItemState>, clone_seq: &mut usize, selected: &mut Vec<usize>) {
        repair_orphan_children(items, clone_seq);
        merge_adjacent_same_mod_blocks(items, selected);
        prune_empty_parent_blocks(items, selected);
    }

    #[test]
    fn the_stranded_lower_half_gets_its_own_split_header() {
        let mut items = vec![
            row("B", "__PARENT__", true, "B::b0"),
            row("B", "1", false, "B::b0"),
            row("A", "__PARENT__", true, "A::b0"),
            row("A", "1", false, "A::b0"),
            row("B", "2", false, "B::b0"),
        ];
        let mut clone_seq = 1usize;
        let mut selected = Vec::new();

        tidy(&mut items, &mut clone_seq, &mut selected);

        assert_eq!(items.len(), 6);
        assert!(items[4].is_parent);
        assert!(items[4].parent_placeholder);
        assert!(items[4].block_id.contains("::split"));
        assert_eq!(items[5].component_id, "2");
        assert_eq!(items[5].block_id, items[4].block_id);
        assert_eq!(items[1].block_id, "B::b0");
        assert_eq!(count_children_in_block(&items, 0), 1);
        assert_eq!(count_children_in_block(&items, 4), 1);
        assert_every_component_sits_under_its_own_header(&items);
    }

    #[test]
    fn loose_intruders_and_the_stranded_half_each_get_a_header() {
        let mut items = vec![
            row("A", "__PARENT__", true, "A::b0"),
            row("A", "9", false, "A::b0"),
            row("B", "__PARENT__", true, "B::b0"),
            row("B", "1", false, "B::b0"),
            row("A", "1", false, "A::b0"),
            row("B", "2", false, "B::b0"),
        ];
        let mut clone_seq = 1usize;
        let mut selected = Vec::new();

        tidy(&mut items, &mut clone_seq, &mut selected);

        assert_every_component_sits_under_its_own_header(&items);
        let a1_block = items
            .iter()
            .find(|i| i.mod_name == "A" && i.component_id == "1")
            .unwrap()
            .block_id
            .clone();
        let b2_block = items
            .iter()
            .find(|i| i.mod_name == "B" && i.component_id == "2")
            .unwrap()
            .block_id
            .clone();
        assert_ne!(a1_block, "A::b0");
        assert_ne!(b2_block, "B::b0");
        assert_ne!(a1_block, b2_block);
        let a1_header = items
            .iter()
            .find(|i| i.is_parent && i.block_id == a1_block)
            .unwrap();
        let b2_header = items
            .iter()
            .find(|i| i.is_parent && i.block_id == b2_block)
            .unwrap();
        assert!(a1_header.parent_placeholder);
        assert!(b2_header.parent_placeholder);
    }

    #[test]
    fn a_stranded_row_adopts_a_same_mod_header_even_across_a_third_mod() {
        let mut items = vec![
            row("A", "__PARENT__", true, "A::b0"),
            row("A", "1", false, "A::b0"),
            row("C", "__PARENT__", true, "C::b0"),
            row("C", "1", false, "C::b0"),
            row("A", "__PARENT__", true, "A::b0::split7"),
            row("A", "2", false, "A::b0::split7"),
            row("A", "3", false, "A::b0"),
        ];
        let mut clone_seq = 1usize;
        let mut selected = Vec::new();

        tidy(&mut items, &mut clone_seq, &mut selected);

        assert_eq!(items.len(), 7);
        let a3 = items
            .iter()
            .find(|i| i.mod_name == "A" && i.component_id == "3")
            .unwrap();
        assert_eq!(a3.block_id, "A::b0::split7");
        assert_every_component_sits_under_its_own_header(&items);
    }

    #[test]
    fn dragging_the_intruder_out_rejoins_the_halves() {
        let mut items = vec![
            row("B", "__PARENT__", true, "B::b0"),
            row("B", "1", false, "B::b0"),
            row("B", "__PARENT__", true, "B::b0::split1"),
            row("B", "2", false, "B::b0::split1"),
            row("A", "__PARENT__", true, "A::b0"),
            row("A", "1", false, "A::b0"),
        ];
        let mut clone_seq = 1usize;
        let mut selected = Vec::new();

        tidy(&mut items, &mut clone_seq, &mut selected);

        assert_eq!(items.len(), 5);
        assert!(items[0].is_parent);
        assert_eq!(items[0].block_id, "B::b0");
        assert_eq!(items[1].component_id, "1");
        assert_eq!(items[1].block_id, "B::b0");
        assert_eq!(items[2].component_id, "2");
        assert_eq!(items[2].block_id, "B::b0");
        assert!(items[3].is_parent);
        assert_eq!(items[3].mod_name, "A");
        assert_eq!(items[4].component_id, "1");
        assert_every_component_sits_under_its_own_header(&items);
    }

    #[test]
    fn a_tidy_list_is_left_exactly_as_it_was() {
        let mut items = vec![
            row("A", "__PARENT__", true, "A::b0"),
            row("A", "1", false, "A::b0"),
            row("A", "2", false, "A::b0"),
            row("B", "__PARENT__", true, "B::b0"),
            row("B", "1", false, "B::b0"),
        ];
        let before = items.clone();
        let mut clone_seq = 1usize;
        let mut selected = Vec::new();

        tidy(&mut items, &mut clone_seq, &mut selected);

        assert_eq!(items, before);
        assert_every_component_sits_under_its_own_header(&items);
    }
}
