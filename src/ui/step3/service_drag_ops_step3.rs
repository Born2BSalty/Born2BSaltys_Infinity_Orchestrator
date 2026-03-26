// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::Step3ItemState;
use crate::ui::step3::blocks;
use crate::ui::step3::drag;

pub(crate) struct DragFinalizeContext<'a> {
    pub items: &'a mut Vec<Step3ItemState>,
    pub selected: &'a mut Vec<usize>,
    pub drag_from: &'a mut Option<usize>,
    pub drag_over: &'a mut Option<usize>,
    pub drag_indices: &'a mut Vec<usize>,
    pub drag_grab_offset: &'a mut f32,
    pub drag_grab_pos_in_block: &'a mut usize,
    pub drag_row_h: &'a mut f32,
    pub last_insert_at: &'a mut Option<usize>,
    pub clone_seq: &'a mut usize,
}

pub(crate) struct DragPointerContext<'a> {
    pub items: &'a [Step3ItemState],
    pub drag_from: &'a Option<usize>,
    pub drag_over: &'a mut Option<usize>,
    pub drag_indices: &'a [usize],
    pub drag_grab_offset: &'a f32,
    pub drag_grab_pos_in_block: &'a usize,
    pub drag_row_h: &'a f32,
    pub visible_rows: &'a [(usize, egui::Rect)],
}

pub(crate) struct LiveReorderContext<'a> {
    pub items: &'a mut Vec<Step3ItemState>,
    pub selected: &'a mut Vec<usize>,
    pub drag_from: &'a mut Option<usize>,
    pub drag_over: &'a Option<usize>,
    pub drag_indices: &'a mut Vec<usize>,
    pub drag_grab_pos_in_block: &'a usize,
    pub last_insert_at: &'a mut Option<usize>,
    pub locked_blocks: &'a [String],
    pub visible_rows: &'a [(usize, egui::Rect)],
}

pub(crate) fn finalize_on_release(ui: &egui::Ui, ctx: &mut DragFinalizeContext<'_>) -> bool {
    let items = &mut *ctx.items;
    let selected = &mut *ctx.selected;
    let drag_from = &mut *ctx.drag_from;
    let drag_over = &mut *ctx.drag_over;
    let drag_indices = &mut *ctx.drag_indices;
    let drag_grab_offset = &mut *ctx.drag_grab_offset;
    let drag_grab_pos_in_block = &mut *ctx.drag_grab_pos_in_block;
    let drag_row_h = &mut *ctx.drag_row_h;
    let last_insert_at = &mut *ctx.last_insert_at;
    let clone_seq = &mut *ctx.clone_seq;
    if !ui.input(|i| i.pointer.any_released()) {
        return false;
    }
    let had_drag = drag_from.is_some() || !drag_indices.is_empty();
    let selected_keys: std::collections::HashSet<String> = selected
        .iter()
        .filter_map(|idx| items.get(*idx))
        .filter(|item| !item.is_parent)
        .map(blocks::step3_item_key)
        .collect();
    *drag_from = None;
    *drag_over = None;
    drag_indices.clear();
    *drag_grab_offset = 0.0;
    *drag_grab_pos_in_block = 0;
    *drag_row_h = 0.0;
    *last_insert_at = None;
    blocks::repair_orphan_children(items, selected, clone_seq);
    blocks::merge_adjacent_same_mod_blocks(items, selected);
    blocks::prune_empty_parent_blocks(items, selected);
    if !selected_keys.is_empty() {
        selected.clear();
        for (idx, item) in items.iter().enumerate() {
            if item.is_parent {
                continue;
            }
            if selected_keys.contains(&blocks::step3_item_key(item)) {
                selected.push(idx);
            }
        }
        selected.sort_unstable();
        selected.dedup();
    }
    had_drag
}

pub(crate) fn draw_insert_marker(
    ui: &egui::Ui,
    items: &[Step3ItemState],
    drag_from: &Option<usize>,
    drag_over: Option<usize>,
    visible_rows: &[(usize, egui::Rect)],
) {
    if drag_from.is_none() {
        return;
    }
    if let Some(insert_at) = drag_over {
        let row_rects: Vec<egui::Rect> = visible_rows.iter().map(|(_, r)| *r).collect();
        if !row_rects.is_empty() {
            let clamped = insert_at.min(items.len());
            let (x0, x1, y) = if clamped == 0 {
                let r = row_rects[0];
                (r.left(), r.right(), r.top() - 1.0)
            } else if clamped >= row_rects.len() {
                let r = row_rects[row_rects.len() - 1];
                (r.left(), r.right(), r.bottom() + 1.0)
            } else {
                let r = row_rects[clamped];
                (r.left(), r.right(), r.top() - 1.0)
            };
            ui.painter().line_segment(
                [egui::pos2(x0, y), egui::pos2(x1, y)],
                egui::Stroke::new(1.5, ui.visuals().selection.stroke.color),
            );
        }
    }
}

pub(crate) fn update_drag_target_from_pointer(
    ui: &egui::Ui,
    ctx: &mut DragPointerContext<'_>,
) {
    let items = ctx.items;
    let drag_from = ctx.drag_from;
    let drag_over = &mut *ctx.drag_over;
    let drag_indices = ctx.drag_indices;
    let drag_grab_offset = ctx.drag_grab_offset;
    let drag_grab_pos_in_block = ctx.drag_grab_pos_in_block;
    let drag_row_h = ctx.drag_row_h;
    let visible_rows = ctx.visible_rows;
    if drag_from.is_none() {
        return;
    }
    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        let n = visible_rows.len();
        let k = visible_rows
            .iter()
            .filter(|(idx, _)| drag_indices.contains(idx))
            .count()
            .max(1);
        if n > 0 && k > 0 {
            let list_top_y = visible_rows.first().map(|(_, r)| r.top()).unwrap_or(pointer.y);
            let row_h = if *drag_row_h > 0.0 {
                *drag_row_h
            } else {
                (visible_rows.first().map(|(_, r)| r.height()).unwrap_or(20.0)
                    + ui.spacing().item_spacing.y.max(0.0))
                .max(1.0)
            };
            let desired_block_start = drag::compute_desired_block_start(
                pointer.y,
                list_top_y,
                row_h,
                *drag_grab_offset,
                *drag_grab_pos_in_block,
                n,
                k,
            );
            *drag_over = Some(desired_block_start.min(items.len()));
        }
    }
}

pub(crate) fn apply_live_reorder(ui: &egui::Ui, ctx: &mut LiveReorderContext<'_>) {
    let items = &mut *ctx.items;
    let selected = &mut *ctx.selected;
    let drag_from = &mut *ctx.drag_from;
    let drag_over = ctx.drag_over;
    let drag_indices = &mut *ctx.drag_indices;
    let drag_grab_pos_in_block = ctx.drag_grab_pos_in_block;
    let last_insert_at = &mut *ctx.last_insert_at;
    let locked_blocks = ctx.locked_blocks;
    let visible_rows = ctx.visible_rows;
    if !ui.input(|i| i.pointer.primary_down()) || drag_from.is_none() || drag_indices.is_empty() {
        return;
    }
    let Some(target_slot) = *drag_over else {
        return;
    };
    if *last_insert_at == Some(target_slot) {
        return;
    }

    let mut block = drag_indices.clone();
    block.sort_unstable();
    block.dedup();
    if !block.iter().all(|idx| *idx < items.len()) {
        return;
    }
    let moving: Vec<_> = block.iter().map(|idx| items[*idx].clone()).collect();
    if moving.iter().any(|m| locked_blocks.contains(&m.block_id)) {
        return;
    }
    let mut remaining = Vec::with_capacity(items.len() - moving.len());
    for (idx, item) in items.iter().cloned().enumerate() {
        if !block.contains(&idx) {
            remaining.push(item);
        }
    }
    let mut insert_at = target_slot;
    insert_at = drag::visible_slot_to_insert_at(items, &block, visible_rows, insert_at, remaining.len());
    if block.first().is_some_and(|first| items[*first].is_parent) {
        insert_at = drag::snap_to_parent_boundary(&remaining, insert_at);
    } else {
        insert_at = drag::enforce_child_parent_constraint(&remaining, insert_at, &moving);
    }
    insert_at = drag::hard_clamp_insert_at(&remaining, insert_at, &moving);
    let mut reordered = remaining;
    reordered.splice(insert_at..insert_at, moving);
    if *items != reordered {
        *items = reordered;
        selected.clear();
        drag_indices.clear();
        for idx in insert_at..insert_at + block.len() {
            selected.push(idx);
            drag_indices.push(idx);
        }
        let grabbed = insert_at + (*drag_grab_pos_in_block).min(block.len() - 1);
        *drag_from = Some(grabbed);
    }
    *last_insert_at = Some(insert_at);
}
