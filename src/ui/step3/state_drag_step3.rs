// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod constraints {
    use crate::app::state::Step3ItemState;

    #[must_use]
    pub fn snap_to_parent_boundary(remaining: &[Step3ItemState], target: usize) -> usize {
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

    #[must_use]
    pub fn enforce_child_parent_constraint(
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

    #[must_use]
    pub fn hard_clamp_insert_at(
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
}

mod math {
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "drag row positions are bounded UI coordinates converted to row indices"
    )]
    const fn floor_isize(value: f32) -> isize {
        value.floor() as isize
    }

    #[must_use]
    pub fn compute_desired_block_start(
        pointer_y: f32,
        list_top_y: f32,
        row_pitch: f32,
        grab_offset: f32,
        grab_pos_in_block: usize,
        n: usize,
        k: usize,
    ) -> usize {
        let row_pitch = row_pitch.max(1.0);
        let desired_grabbed_row = floor_isize((pointer_y - list_top_y - grab_offset) / row_pitch);
        let max_start = isize::try_from(n.saturating_sub(k)).unwrap_or(isize::MAX);
        let grab_pos_in_block = isize::try_from(grab_pos_in_block).unwrap_or(isize::MAX);
        usize::try_from((desired_grabbed_row - grab_pos_in_block).clamp(0, max_start)).unwrap_or(0)
    }
}

mod slots {
    use eframe::egui;

    use crate::app::state::Step3ItemState;

    #[must_use]
    pub fn visible_slot_to_insert_at(
        items: &[Step3ItemState],
        block: &[usize],
        visible_rows: &[(usize, egui::Rect)],
        target_visible_slot: usize,
        remaining_len: usize,
    ) -> usize {
        let remaining_full_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| {
                if block.contains(&idx) {
                    None
                } else {
                    Some(idx)
                }
            })
            .collect();
        let visible_remaining_full: Vec<usize> = visible_rows
            .iter()
            .filter_map(|(idx, _)| {
                if block.contains(idx) {
                    None
                } else {
                    Some(*idx)
                }
            })
            .collect();
        if target_visible_slot >= visible_remaining_full.len() {
            return remaining_len;
        }
        let target_full = visible_remaining_full[target_visible_slot];
        remaining_full_indices
            .iter()
            .position(|idx| *idx == target_full)
            .unwrap_or(remaining_len)
    }
}

pub use constraints::enforce_child_parent_constraint;
pub use constraints::hard_clamp_insert_at;
pub use constraints::snap_to_parent_boundary;
pub use math::compute_desired_block_start;
pub use slots::visible_slot_to_insert_at;

#[cfg(test)]
mod tests {
    use super::compute_desired_block_start;

    #[test]
    fn start_is_clamped_to_valid_range() {
        let n = 10;
        let k = 4;
        let s1 = compute_desired_block_start(-1000.0, 100.0, 24.0, 5.0, 1, n, k);
        let s2 = compute_desired_block_start(99999.0, 100.0, 24.0, 5.0, 1, n, k);
        assert_eq!(s1, 0);
        assert_eq!(s2, n - k);
    }
}
