// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;

use crate::app::state::Step3ItemState;
use crate::ui::step3::blocks;
use eframe::egui;

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
        let start = *anchor.get_or_insert(idx);
        let start_pos = visible_indices.iter().position(|v| *v == start);
        let end_pos = visible_indices.iter().position(|v| *v == idx);
        if let (Some(a), Some(b)) = (start_pos, end_pos) {
            let (from, to) = if a <= b { (a, b) } else { (b, a) };
            *selected = range_selection(items, visible_indices, from, to, [start, idx]);
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

fn range_selection(
    items: &[Step3ItemState],
    visible_indices: &[usize],
    from: usize,
    to: usize,
    endpoints: [usize; 2],
) -> Vec<usize> {
    let range = &visible_indices[from..=to];
    let in_range: HashSet<usize> = range.iter().copied().collect();
    let visible: HashSet<usize> = visible_indices.iter().copied().collect();
    let mut out = Vec::with_capacity(range.len());
    for &row in range {
        let Some(item) = items.get(row) else {
            continue;
        };
        if !item.is_parent {
            out.push(row);
            continue;
        }
        let block = blocks::block_indices(items, row);
        let mut children = block.iter().copied().filter(|member| *member != row);
        let whole_mod_covered = endpoints.contains(&row)
            || children.all(|child| in_range.contains(&child) || !visible.contains(&child));
        if whole_mod_covered {
            out.extend(block);
        } else {
            out.push(row);
        }
    }
    out.extend(headers_of_fully_selected_mods(items, &out));
    out
}

fn headers_of_fully_selected_mods(items: &[Step3ItemState], chosen_rows: &[usize]) -> Vec<usize> {
    let chosen: HashSet<usize> = chosen_rows.iter().copied().collect();
    items
        .iter()
        .enumerate()
        .filter(|(row, item)| item.is_parent && !chosen.contains(row))
        .filter_map(|(row, _)| {
            let block = blocks::block_indices(items, row);
            let mut children = block
                .iter()
                .copied()
                .filter(|member| *member != row)
                .peekable();
            let has_children = children.peek().is_some();
            (has_children && children.all(|child| chosen.contains(&child))).then_some(row)
        })
        .collect()
}

pub mod component_uncheck {
    pub(crate) use crate::ui::step3::service_component_uncheck_step3::*;
}

pub mod prompt_actions {
    pub(crate) use crate::ui::step3::service_prompt_actions_step3::*;
}

pub mod drag_ops {
    pub(crate) use crate::ui::step3::service_drag_ops_step3::*;
}

#[cfg(test)]
mod tests {
    use super::apply_row_selection;
    use crate::app::state::Step3ItemState;
    use eframe::egui;

    const SHIFT: egui::Modifiers = egui::Modifiers {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: false,
        command: false,
    };
    const CTRL: egui::Modifiers = egui::Modifiers {
        alt: false,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    const PLAIN: egui::Modifiers = egui::Modifiers::NONE;

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

    fn three_mods_of_three() -> Vec<Step3ItemState> {
        let mut items = Vec::new();
        for mod_name in ["A", "B", "C"] {
            items.push(row(mod_name, "__PARENT__", true));
            items.push(row(mod_name, "1", false));
            items.push(row(mod_name, "2", false));
            items.push(row(mod_name, "3", false));
        }
        items
    }

    fn every_row_visible() -> Vec<usize> {
        (0..12).collect()
    }

    fn with_mod_b_collapsed() -> Vec<usize> {
        vec![0, 1, 2, 3, 4, 8, 9, 10, 11]
    }

    #[test]
    fn shift_click_across_mods_takes_every_row_between_the_two_clicks() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(anchor, Some(2));
    }

    #[test]
    fn a_header_inside_the_range_is_lit_with_only_its_in_range_components() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 6, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn shift_click_upward_selects_the_same_rows() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![9];
        let mut anchor = Some(9);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 2, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(anchor, Some(9));
    }

    #[test]
    fn part_of_one_mod_is_selected_without_its_header() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![1];
        let mut anchor = Some(1);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 2, SHIFT);

        assert_eq!(selected, vec![1, 2]);
    }

    #[test]
    fn every_component_of_a_mod_takes_its_header_even_outside_the_range() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![1];
        let mut anchor = Some(1);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 3, SHIFT);

        assert_eq!(selected, vec![0, 1, 2, 3]);
    }

    #[test]
    fn a_range_from_a_mods_first_component_takes_that_mods_header_too() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![1];
        let mut anchor = Some(1);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 7, SHIFT);

        assert_eq!(selected, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn a_collapsed_mod_inside_the_range_is_taken_whole_with_its_hidden_components() {
        let items = three_mods_of_three();
        let visible = with_mod_b_collapsed();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn shift_clicking_a_header_takes_that_whole_mod() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 8, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    }

    #[test]
    fn a_header_anchor_takes_its_whole_mod_into_the_range() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![4];
        let mut anchor = Some(4);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);

        assert_eq!(selected, vec![4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn a_second_shift_click_replaces_the_range_from_the_same_anchor() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);
        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 5, SHIFT);

        assert_eq!(selected, vec![2, 3, 4, 5]);
        assert_eq!(anchor, Some(2));
    }

    #[test]
    fn a_first_shift_click_sets_the_anchor_so_the_next_one_makes_a_range() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![];
        let mut anchor = None;

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 5, SHIFT);
        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);

        assert_eq!(selected, vec![4, 5, 6, 7, 8, 9]);
        assert_eq!(anchor, Some(5));
    }

    #[test]
    fn shift_click_from_a_hidden_anchor_selects_the_clicked_row() {
        let items = three_mods_of_three();
        let visible = with_mod_b_collapsed();
        let mut selected: Vec<usize> = vec![5];
        let mut anchor = Some(5);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 9, SHIFT);

        assert_eq!(selected, vec![9]);
    }

    #[test]
    fn ctrl_click_after_a_range_adds_and_removes_single_rows() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![2];
        let mut anchor = Some(2);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 5, SHIFT);
        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 11, CTRL);
        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 5, CTRL);

        assert_eq!(selected, vec![2, 3, 4, 11]);
        assert_eq!(anchor, Some(5));
    }

    #[test]
    fn plain_click_replaces_the_selection_and_moves_the_anchor() {
        let items = three_mods_of_three();
        let visible = every_row_visible();
        let mut selected: Vec<usize> = vec![1, 2, 3];
        let mut anchor = Some(1);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, 6, PLAIN);

        assert_eq!(selected, vec![6]);
        assert_eq!(anchor, Some(6));
    }

    #[test]
    fn grabbing_a_covered_header_moves_everything_that_is_highlighted() {
        let mut items = Vec::new();
        items.push(row("Mod1", "__PARENT__", true));
        for component in ["1", "2", "3", "4"] {
            items.push(row("Mod1", component, false));
        }
        for mod_name in ["Mod2", "Mod3"] {
            items.push(row(mod_name, "__PARENT__", true));
            items.push(row(mod_name, "1", false));
            items.push(row(mod_name, "2", false));
        }
        let visible: Vec<usize> = (0..items.len()).collect();
        let mod1_c3 = 3;
        let mod2_header = 5;
        let mod3_c2 = 10;
        let mut selected: Vec<usize> = vec![mod1_c3];
        let mut anchor = Some(mod1_c3);

        apply_row_selection(&mut selected, &mut anchor, &items, &visible, mod3_c2, SHIFT);

        assert_eq!(selected, vec![3, 4, 5, 6, 7, 8, 9, 10]);
        let dragged =
            crate::ui::step3::move_selection_step3::moving_set(&items, &selected, &[], mod2_header);
        assert_eq!(dragged, vec![3, 4, 5, 6, 7, 8, 9, 10]);
    }
}
