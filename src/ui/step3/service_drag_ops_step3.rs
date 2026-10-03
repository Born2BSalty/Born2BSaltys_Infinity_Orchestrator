// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;
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
    pub drag_from: Option<&'a usize>,
    pub drag_over: &'a mut Option<usize>,
    pub drag_indices: &'a [usize],
    pub drag_grab_offset: &'a f32,
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
    let selected_header_blocks: std::collections::HashSet<String> = selected
        .iter()
        .filter_map(|idx| items.get(*idx))
        .filter(|item| item.is_parent)
        .map(|item| item.block_id.clone())
        .collect();
    *drag_from = None;
    *drag_over = None;
    drag_indices.clear();
    *drag_grab_offset = 0.0;
    *drag_grab_pos_in_block = 0;
    *drag_row_h = 0.0;
    *last_insert_at = None;
    blocks::repair_orphan_children(items, clone_seq);
    blocks::merge_adjacent_same_mod_blocks(items, selected);
    blocks::prune_empty_parent_blocks(items, selected);
    if !selected_keys.is_empty() || !selected_header_blocks.is_empty() {
        *selected = reselect_after_cleanup(items, &selected_keys, &selected_header_blocks);
    }
    had_drag
}

fn reselect_after_cleanup(
    items: &[Step3ItemState],
    selected_keys: &std::collections::HashSet<String>,
    selected_header_blocks: &std::collections::HashSet<String>,
) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if item.is_parent {
                selected_header_blocks.contains(&item.block_id)
            } else {
                selected_keys.contains(&blocks::step3_item_key(item))
            }
        })
        .map(|(idx, _)| idx)
        .collect()
}

pub(crate) fn draw_insert_marker(
    ui: &egui::Ui,
    items: &[Step3ItemState],
    drag_active: bool,
    drag_over: Option<usize>,
    visible_rows: &[(usize, egui::Rect)],
) {
    if !drag_active {
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
                egui::Stroke::new(1.5_f32, ui.visuals().selection.stroke.color),
            );
        }
    }
}

pub(crate) fn update_drag_target_from_pointer(ui: &egui::Ui, ctx: &mut DragPointerContext<'_>) {
    let items = ctx.items;
    let Some(grabbed) = ctx.drag_from else {
        return;
    };
    let drag_over = &mut *ctx.drag_over;
    let drag_indices = ctx.drag_indices;
    let drag_grab_offset = ctx.drag_grab_offset;
    let _ = ctx.drag_row_h;
    let visible_rows = ctx.visible_rows;
    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        let slot = drag::pointer_to_visible_slot(
            visible_rows,
            drag_indices,
            *grabbed,
            pointer.y - *drag_grab_offset,
        );
        *drag_over = Some(slot.min(items.len()));
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
    let insert_at =
        drag::visible_slot_to_insert_at(items, &block, visible_rows, target_slot, remaining.len());
    let insert_at = drag::resolve_insert_at(&remaining, insert_at, &moving, locked_blocks);
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

#[cfg(test)]
mod tests {
    use super::{DragFinalizeContext, LiveReorderContext, apply_live_reorder, finalize_on_release};
    use crate::app::state::Step3ItemState;
    use eframe::egui;

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

    fn two_mods() -> Vec<Step3ItemState> {
        vec![
            row("A", "__PARENT__", true),
            row("A", "1", false),
            row("A", "2", false),
            row("B", "__PARENT__", true),
            row("B", "1", false),
        ]
    }

    fn pointer_button(pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: egui::pos2(20.0, 20.0),
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn selection_after_a_click_release(items: &mut Vec<Step3ItemState>, selected: &mut Vec<usize>) {
        let ctx = egui::Context::default();
        let press = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(egui::pos2(20.0, 20.0)),
                pointer_button(true),
            ],
            ..Default::default()
        };
        let _ = ctx.run(press, |_| {});
        let release = egui::RawInput {
            events: vec![pointer_button(false)],
            ..Default::default()
        };
        let mut finalized = false;
        let _ = ctx.run(release, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut drag_from = None;
                let mut drag_over = None;
                let mut drag_indices = Vec::new();
                let mut drag_grab_offset = 0.0;
                let mut drag_grab_pos_in_block = 0;
                let mut drag_row_h = 0.0;
                let mut last_insert_at = None;
                let mut clone_seq = 0;
                let mut finalize_ctx = DragFinalizeContext {
                    items: &mut *items,
                    selected: &mut *selected,
                    drag_from: &mut drag_from,
                    drag_over: &mut drag_over,
                    drag_indices: &mut drag_indices,
                    drag_grab_offset: &mut drag_grab_offset,
                    drag_grab_pos_in_block: &mut drag_grab_pos_in_block,
                    drag_row_h: &mut drag_row_h,
                    last_insert_at: &mut last_insert_at,
                    clone_seq: &mut clone_seq,
                };
                assert!(ui.input(|i| i.pointer.any_released()));
                finalize_on_release(ui, &mut finalize_ctx);
                finalized = true;
            });
        });
        assert!(finalized);
    }

    #[test]
    fn a_click_release_keeps_selected_headers_selected() {
        let mut items = two_mods();
        let mut selected = vec![0, 1, 2, 3, 4];

        selection_after_a_click_release(&mut items, &mut selected);

        assert_eq!(selected, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_click_release_keeps_a_components_only_selection_as_it_was() {
        let mut items = two_mods();
        let mut selected = vec![2, 4];

        selection_after_a_click_release(&mut items, &mut selected);

        assert_eq!(selected, vec![2, 4]);
    }

    #[test]
    fn a_click_release_keeps_a_header_only_selection() {
        let mut items = two_mods();
        let mut selected = vec![3];

        selection_after_a_click_release(&mut items, &mut selected);

        assert_eq!(selected, vec![3]);
    }

    fn assert_every_component_sits_under_its_own_header(items: &[Step3ItemState]) {
        for (idx, item) in items.iter().enumerate() {
            if item.is_parent {
                continue;
            }
            let parent = items[..idx]
                .iter()
                .rev()
                .find(|i| i.is_parent)
                .unwrap_or_else(|| panic!("component at {idx} has no header above it"));
            assert_eq!(parent.block_id, item.block_id);
            assert_eq!(parent.mod_name, item.mod_name);
        }
    }

    #[test]
    fn a_release_gives_the_stranded_half_its_own_split_header() {
        let mut items = vec![
            row("B", "__PARENT__", true),
            row("B", "1", false),
            row("A", "__PARENT__", true),
            row("A", "1", false),
            row("B", "2", false),
        ];
        let mut selected = vec![2, 3];

        selection_after_a_click_release(&mut items, &mut selected);

        assert_eq!(items.len(), 6);
        assert!(items[4].is_parent);
        assert!(items[4].parent_placeholder);
        assert_eq!(items[4].mod_name, "B");
        assert_eq!(selected, vec![2, 3]);
        assert_every_component_sits_under_its_own_header(&items);
    }

    #[test]
    fn a_header_released_with_some_components_leaves_the_rest_a_split_header() {
        let mut items = vec![
            row("B", "__PARENT__", true),
            row("B", "1", false),
            row("A", "__PARENT__", true),
            row("A", "1", false),
            row("B", "2", false),
            row("B", "3", false),
        ];
        let mut selected = vec![0, 1];

        selection_after_a_click_release(&mut items, &mut selected);

        assert_every_component_sits_under_its_own_header(&items);
        assert_eq!(selected, vec![0, 1]);
        assert!(items[0].is_parent);
        assert!(!items[0].parent_placeholder);
        let placeholder = items
            .iter()
            .find(|item| item.is_parent && item.parent_placeholder)
            .expect("the left-behind components get a split header");
        assert_eq!(placeholder.mod_name, "B");
    }

    fn stacked_visible_rows(n: usize) -> Vec<(usize, egui::Rect)> {
        (0..n)
            .map(|idx| {
                let top = f32::from(u16::try_from(idx).unwrap_or(u16::MAX)) * 20.0;
                (
                    idx,
                    egui::Rect::from_min_size(egui::pos2(0.0, top), egui::vec2(200.0, 20.0)),
                )
            })
            .collect()
    }

    fn reorder_after_two_frames(
        items: &mut Vec<Step3ItemState>,
        selected: &mut Vec<usize>,
        drag_indices: &mut Vec<usize>,
        mut drag_from: Option<usize>,
        drag_over: Option<usize>,
        locked_blocks: &[String],
        visible_rows: &[(usize, egui::Rect)],
    ) {
        let ctx = egui::Context::default();
        let press = egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(egui::pos2(20.0, 20.0)),
                pointer_button(true),
            ],
            ..Default::default()
        };
        let _ = ctx.run(press, |_| {});
        let mut ran = false;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(ui.input(|i| i.pointer.primary_down()));
                let mut last_insert_at = None;
                let grab_pos_in_block = 0usize;
                let mut reorder_ctx = LiveReorderContext {
                    items,
                    selected,
                    drag_from: &mut drag_from,
                    drag_over: &drag_over,
                    drag_indices,
                    drag_grab_pos_in_block: &grab_pos_in_block,
                    last_insert_at: &mut last_insert_at,
                    locked_blocks,
                    visible_rows,
                };
                apply_live_reorder(ui, &mut reorder_ctx);
                ran = true;
            });
        });
        assert!(ran);
    }

    #[test]
    fn a_live_reorder_lands_a_foreign_mod_between_two_components() {
        let base_items = || {
            vec![
                row("A", "__PARENT__", true),
                row("A", "1", false),
                row("B", "__PARENT__", true),
                row("B", "1", false),
                row("B", "2", false),
            ]
        };
        let visible_rows = stacked_visible_rows(5);

        let mut items = base_items();
        let mut selected = Vec::new();
        let mut drag_indices = vec![0, 1];
        reorder_after_two_frames(
            &mut items,
            &mut selected,
            &mut drag_indices,
            Some(0),
            Some(2),
            &[],
            &visible_rows,
        );
        assert_eq!(
            items.iter().map(|i| i.mod_name.clone()).collect::<Vec<_>>(),
            vec!["B", "B", "A", "A", "B"]
        );
        assert_eq!(selected, vec![2, 3]);

        let mut items = base_items();
        let mut selected = Vec::new();
        let mut drag_indices = vec![0, 1];
        let locked_blocks = vec!["B::block0".to_string()];
        reorder_after_two_frames(
            &mut items,
            &mut selected,
            &mut drag_indices,
            Some(0),
            Some(2),
            &locked_blocks,
            &visible_rows,
        );
        assert_eq!(
            items.iter().map(|i| i.mod_name.clone()).collect::<Vec<_>>(),
            vec!["B", "B", "B", "A", "A"]
        );
    }
}
