// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use crate::ui::state::Step3ItemState;

pub fn apply_row_selection(
    selected: &mut Vec<usize>,
    anchor: &mut Option<usize>,
    items: &[Step3ItemState],
    visible_indices: &[usize],
    idx: usize,
    modifiers: egui::Modifiers,
) {
    if modifiers.shift {
        selected.clear();
        let start = anchor.unwrap_or(idx);
        let start_pos = visible_indices.iter().position(|v| *v == start);
        let end_pos = visible_indices.iter().position(|v| *v == idx);
        if let (Some(a), Some(b)) = (start_pos, end_pos) {
            let (from, to) = if a <= b { (a, b) } else { (b, a) };
            let start_item = &items[start];
            let end_item = &items[idx];
            if !start_item.is_parent && !end_item.is_parent {
                if start_item.block_id == end_item.block_id {
                    for &v in &visible_indices[from..=to] {
                        if !items[v].is_parent && items[v].block_id == start_item.block_id {
                            selected.push(v);
                        }
                    }
                } else {
                    selected.push(idx);
                    *anchor = Some(idx);
                }
            } else {
                for &v in &visible_indices[from..=to] {
                    selected.push(v);
                }
            }
        } else {
            selected.push(idx);
        }
        selected.sort_unstable();
        selected.dedup();
    } else if modifiers.ctrl {
        if let Some(pos) = selected.iter().position(|v| *v == idx) {
            selected.remove(pos);
        } else {
            selected.push(idx);
            selected.sort_unstable();
            selected.dedup();
        }
        *anchor = Some(idx);
    } else {
        selected.clear();
        selected.push(idx);
        *anchor = Some(idx);
    }
}

pub(crate) mod component_uncheck {
    pub(crate) use crate::ui::step3::service_component_uncheck_step3::*;
}

pub(crate) mod prompt_actions {
    pub(crate) use crate::ui::step3::service_prompt_actions_step3::*;
}

pub(crate) mod drag_ops {
    pub(crate) use crate::ui::step3::service_drag_ops_step3::*;
}
