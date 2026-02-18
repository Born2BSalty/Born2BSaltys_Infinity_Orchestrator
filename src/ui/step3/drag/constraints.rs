// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

pub(crate) fn snap_to_parent_boundary(remaining: &[Step3ItemState], target: usize) -> usize {
    let mut candidates: Vec<usize> = remaining
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| if item.is_parent { Some(idx) } else { None })
        .collect();
    candidates.push(remaining.len());
    if !candidates.contains(&0) {
        candidates.push(0);
    }
    let mut best = candidates[0];
    let mut best_dist = best.abs_diff(target);
    for c in candidates.into_iter().skip(1) {
        let d = c.abs_diff(target);
        if d < best_dist {
            best = c;
            best_dist = d;
        }
    }
    best
}

pub(crate) fn enforce_child_parent_constraint(
    remaining: &[Step3ItemState],
    insert_at: usize,
    moving: &[Step3ItemState],
) -> usize {
    if moving.is_empty() || moving.iter().any(|i| i.is_parent) {
        return insert_at;
    }
    let first_key = mod_key(&moving[0]);
    if moving.iter().any(|i| mod_key(i) != first_key) {
        return insert_at;
    }
    if remaining.is_empty() || insert_at == 0 {
        return insert_at;
    }
    if insert_at < remaining.len() && remaining[insert_at].is_parent {
        return insert_at;
    }

    let owner_idx = insert_at.saturating_sub(1);
    let owner_block = remaining[owner_idx].block_id.clone();
    let owner_key = mod_key(&remaining[owner_idx]);
    if owner_key == first_key {
        return insert_at;
    }

    let mut start = owner_idx;
    while start > 0 && remaining[start - 1].block_id == owner_block {
        start -= 1;
    }
    let mut end = owner_idx + 1;
    while end < remaining.len() && remaining[end].block_id == owner_block {
        end += 1;
    }
    let d_start = insert_at.abs_diff(start);
    let d_end = insert_at.abs_diff(end);
    if d_start <= d_end { start } else { end }
}

pub(crate) fn hard_clamp_insert_at(
    remaining: &[Step3ItemState],
    insert_at: usize,
    moving: &[Step3ItemState],
) -> usize {
    let mut clamped = insert_at.min(remaining.len());
    if moving.is_empty() {
        return clamped;
    }

    if moving.iter().any(|i| i.is_parent) {
        return snap_to_parent_boundary(remaining, clamped);
    }

    clamped = enforce_child_parent_constraint(remaining, clamped, moving);
    if clamped == 0 || clamped >= remaining.len() {
        return clamped;
    }

    let left = &remaining[clamped - 1];
    let right = &remaining[clamped];
    if left.block_id == right.block_id {
        let moving_key = mod_key(&moving[0]);
        let owner_key = mod_key(left);
        if owner_key != moving_key {
            let mut start = clamped - 1;
            while start > 0 && remaining[start - 1].block_id == left.block_id {
                start -= 1;
            }
            let mut end = clamped;
            while end < remaining.len() && remaining[end].block_id == left.block_id {
                end += 1;
            }
            let d_start = clamped.abs_diff(start);
            let d_end = clamped.abs_diff(end);
            clamped = if d_start <= d_end { start } else { end };
        }
    }
    clamped
}

fn mod_key(item: &Step3ItemState) -> String {
    format!(
        "{}::{}",
        item.tp_file.to_ascii_uppercase(),
        item.mod_name.to_ascii_uppercase()
    )
}
