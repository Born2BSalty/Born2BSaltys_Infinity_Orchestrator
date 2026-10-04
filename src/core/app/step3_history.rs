// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::app::state::Step3ItemState;

pub(crate) const STEP3_HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Step3TouchedRows {
    pub component_keys: HashSet<String>,
    pub header_blocks: HashSet<String>,
}

impl Step3TouchedRows {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.component_keys.is_empty() && self.header_blocks.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step3HistoryEntry {
    pub items: Vec<Step3ItemState>,
    pub touched: Step3TouchedRows,
}

pub(crate) fn push_undo_snapshot(
    items: &[Step3ItemState],
    touched: Step3TouchedRows,
    undo_stack: &mut Vec<Step3HistoryEntry>,
    redo_stack: &mut Vec<Step3HistoryEntry>,
) {
    undo_stack.push(Step3HistoryEntry {
        items: items.to_vec(),
        touched,
    });
    if undo_stack.len() > STEP3_HISTORY_LIMIT {
        undo_stack.remove(0);
    }
    redo_stack.clear();
}

pub(crate) fn expand_all(collapsed_blocks: &mut Vec<String>) {
    collapsed_blocks.clear();
}

pub(crate) fn collapse_all(items: &[Step3ItemState], collapsed_blocks: &mut Vec<String>) {
    collapsed_blocks.clear();
    for item in items.iter().filter(|item| item.is_parent) {
        if !collapsed_blocks.contains(&item.block_id) {
            collapsed_blocks.push(item.block_id.clone());
        }
    }
}

pub(crate) fn redo(
    items: &mut Vec<Step3ItemState>,
    undo_stack: &mut Vec<Step3HistoryEntry>,
    redo_stack: &mut Vec<Step3HistoryEntry>,
) -> Option<Step3TouchedRows> {
    let entry = redo_stack.pop()?;
    undo_stack.push(Step3HistoryEntry {
        items: std::mem::replace(items, entry.items),
        touched: entry.touched.clone(),
    });
    Some(entry.touched)
}

pub(crate) fn undo(
    items: &mut Vec<Step3ItemState>,
    undo_stack: &mut Vec<Step3HistoryEntry>,
    redo_stack: &mut Vec<Step3HistoryEntry>,
) -> Option<Step3TouchedRows> {
    let entry = undo_stack.pop()?;
    redo_stack.push(Step3HistoryEntry {
        items: std::mem::replace(items, entry.items),
        touched: entry.touched.clone(),
    });
    Some(entry.touched)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(mod_name: &str, component_id: &str, is_parent: bool) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: component_id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: format!("{mod_name}::block0"),
            is_parent,
            parent_placeholder: false,
        }
    }

    fn touched(component_keys: &[&str]) -> Step3TouchedRows {
        Step3TouchedRows {
            component_keys: component_keys
                .iter()
                .map(|key| (*key).to_string())
                .collect(),
            header_blocks: HashSet::new(),
        }
    }

    #[test]
    fn undo_returns_the_rows_its_step_touched_and_redo_returns_them_again() {
        let mut items = vec![row("A", "1", false), row("B", "1", false)];
        let original = items.clone();
        let mut undo_stack = Vec::new();
        let mut redo_stack = Vec::new();

        push_undo_snapshot(&items, touched(&["k1"]), &mut undo_stack, &mut redo_stack);
        items[0].component_id = "mutated".to_string();
        let mutated = items.clone();

        let undo_result = undo(&mut items, &mut undo_stack, &mut redo_stack);
        assert_eq!(undo_result, Some(touched(&["k1"])));
        assert_eq!(items, original);

        let redo_result = redo(&mut items, &mut undo_stack, &mut redo_stack);
        assert_eq!(redo_result, Some(touched(&["k1"])));
        assert_eq!(items, mutated);
    }

    #[test]
    fn two_steps_undo_and_redo_each_with_their_own_rows() {
        let mut items = vec![
            row("A", "1", false),
            row("B", "1", false),
            row("C", "1", false),
            row("D", "1", false),
        ];
        let original = items.clone();
        let mut undo_stack = Vec::new();
        let mut redo_stack = Vec::new();

        push_undo_snapshot(
            &items,
            touched(&["a", "b", "c"]),
            &mut undo_stack,
            &mut redo_stack,
        );
        items.swap(0, 3);
        let after_step_1 = items.clone();

        push_undo_snapshot(
            &items,
            touched(&["c", "d"]),
            &mut undo_stack,
            &mut redo_stack,
        );
        items.swap(1, 2);
        let after_step_2 = items.clone();

        assert_eq!(
            undo(&mut items, &mut undo_stack, &mut redo_stack),
            Some(touched(&["c", "d"]))
        );
        assert_eq!(items, after_step_1);

        assert_eq!(
            undo(&mut items, &mut undo_stack, &mut redo_stack),
            Some(touched(&["a", "b", "c"]))
        );
        assert_eq!(items, original);

        assert_eq!(
            redo(&mut items, &mut undo_stack, &mut redo_stack),
            Some(touched(&["a", "b", "c"]))
        );
        assert_eq!(items, after_step_1);

        assert_eq!(
            redo(&mut items, &mut undo_stack, &mut redo_stack),
            Some(touched(&["c", "d"]))
        );
        assert_eq!(items, after_step_2);
    }
}
