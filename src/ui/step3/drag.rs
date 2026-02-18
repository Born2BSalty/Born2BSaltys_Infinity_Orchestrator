// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod constraints;
mod math;
mod slots;

pub(crate) use constraints::enforce_child_parent_constraint;
pub(crate) use constraints::hard_clamp_insert_at;
pub(crate) use constraints::snap_to_parent_boundary;
pub(crate) use math::compute_desired_block_start;
pub(crate) use slots::visible_slot_to_insert_at;

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
