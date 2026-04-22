// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step3ItemState;

pub(crate) const STEP3_HISTORY_LIMIT: usize = 100;

pub(crate) fn push_undo_snapshot(
    items: &[Step3ItemState],
    undo_stack: &mut Vec<Vec<Step3ItemState>>,
    redo_stack: &mut Vec<Vec<Step3ItemState>>,
) {
    undo_stack.push(items.to_vec());
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
    undo_stack: &mut Vec<Vec<Step3ItemState>>,
    redo_stack: &mut Vec<Vec<Step3ItemState>>,
) {
    if let Some(next) = redo_stack.pop() {
        undo_stack.push(items.clone());
        *items = next;
    }
}

pub(crate) fn undo(
    items: &mut Vec<Step3ItemState>,
    undo_stack: &mut Vec<Vec<Step3ItemState>>,
    redo_stack: &mut Vec<Vec<Step3ItemState>>,
) {
    if let Some(previous) = undo_stack.pop() {
        redo_stack.push(items.clone());
        *items = previous;
    }
}
