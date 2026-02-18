// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

const STEP3_HISTORY_LIMIT: usize = 100;

pub(super) fn push_undo_snapshot(
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
