// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SectionDrop {
    pub(super) before: Option<usize>,
    pub(super) line_y: f32,
}

pub(super) fn is_section_drag(
    items: &[Step3ItemState],
    drag_from: Option<usize>,
    drag_indices: &[usize],
) -> bool {
    drag_indices.len() > 1
        && drag_from
            .and_then(|idx| items.get(idx))
            .is_some_and(Step3ItemState::is_separator)
}

pub(super) fn section_drop_target(
    items: &[Step3ItemState],
    visible_rows: &[(usize, egui::Rect)],
    ghost_center_y: f32,
    gap: f32,
) -> Option<SectionDrop> {
    let (_, last) = visible_rows.last()?;
    visible_rows
        .iter()
        .filter(|(idx, _)| items.get(*idx).is_some_and(Step3ItemState::is_separator))
        .map(|(idx, rect)| (Some(*idx), rect.top(), rect.top() - gap))
        .chain(std::iter::once((None, last.bottom(), last.bottom() + gap)))
        .min_by(|a, b| {
            (a.1 - ghost_center_y)
                .abs()
                .total_cmp(&(b.1 - ghost_center_y).abs())
        })
        .map(|(before, _, line_y)| SectionDrop { before, line_y })
}

#[cfg(test)]
mod tests {
    use eframe::egui;

    use super::{SectionDrop, section_drop_target};
    use crate::app::state::Step3ItemState;
    use crate::app::step3_separators::separator_row;

    fn header(mod_name: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: format!("{mod_name}.tp2"),
            component_id: "__PARENT__".to_string(),
            mod_name: mod_name.to_string(),
            component_label: String::new(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: format!("{mod_name}::block0"),
            is_parent: true,
            parent_placeholder: false,
        }
    }

    fn list() -> Vec<Step3ItemState> {
        vec![
            header("Fixpack"),
            separator_row("Utilities", "SEPARATOR::1".to_string()),
            header("Eeex"),
            separator_row("Quests", "SEPARATOR::2".to_string()),
            header("Heart"),
            header("Ruins"),
            separator_row("Tweaks", "SEPARATOR::3".to_string()),
            header("Klatu"),
        ]
    }

    #[test]
    fn the_drop_goes_to_the_band_edge_nearest_the_floating_band() {
        let items = list();
        let rows: Vec<(usize, egui::Rect)> = [0usize, 1, 2, 3, 4, 5, 6, 7]
            .iter()
            .zip(0u8..)
            .map(|(idx, slot)| {
                let top = f32::from(slot) * 20.0;
                (
                    *idx,
                    egui::Rect::from_min_size(egui::pos2(0.0, top), egui::vec2(100.0, 20.0)),
                )
            })
            .collect();

        let near_tweaks = section_drop_target(&items, &rows, 118.0, 4.0);
        assert_eq!(
            near_tweaks,
            Some(SectionDrop {
                before: Some(6),
                line_y: 116.0
            })
        );
        let past_the_end = section_drop_target(&items, &rows, 400.0, 4.0);
        assert_eq!(past_the_end.map(|drop| drop.before), Some(None));
        let near_the_top = section_drop_target(&items, &rows, 0.0, 4.0);
        assert_eq!(near_the_top.map(|drop| drop.before), Some(Some(1)));
    }
}
