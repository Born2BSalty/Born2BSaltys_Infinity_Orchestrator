// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(crate) fn compute_desired_block_start(
    pointer_y: f32,
    list_top_y: f32,
    row_pitch: f32,
    grab_offset: f32,
    grab_pos_in_block: usize,
    n: usize,
    k: usize,
) -> usize {
    let row_pitch = row_pitch.max(1.0);
    let desired_grabbed_row = ((pointer_y - list_top_y - grab_offset) / row_pitch).floor() as isize;
    let max_start = (n.saturating_sub(k)) as isize;
    (desired_grabbed_row - grab_pos_in_block as isize).clamp(0, max_start) as usize
}
