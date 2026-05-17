// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `step4_review_list` — the line-numbered monospace review list for the
// normal install modes (P6.T2b, SPEC §8.1). Net-new redesign chrome.
//
// Mirrors the wireframe `OrderPanel` Box (`screens.jsx:3199-3223`):
//
//   <Box padding:12 flex:1 minHeight:0 display:flex flexDirection:column>
//     <div flex:1 minHeight:0 overflow:auto>
//       {selected.length === 0
//         ? <Label color:text-faint>No components selected on {upperTab}.</Label>
//         : selected.map((c,i) => (
//             <div flex alignItems:baseline gap:10>
//               <span ...lineNumber/> <WeiduLine c={c}/>
//             </div>))}
//     </div>
//   </Box>
//
// Per SPEC §8.1:
//   - Right-aligned line number (1, 2, 3, …; column auto-sizes to total
//     count; no leading zeros).
//   - WeiDU-style line with the §6.7 three-colour encoding (delegated to
//     `widgets/weidu_line::render_weidu_line` — the redesign-token surface,
//     NOT BIO's `content_step4::render_weidu_colored_line`).
//   - Empty → `No components selected on <TAB>.` in faint type.
//
// Only the installable **leaves** are listed (`!is_parent`) — the exact
// filter BIO's `content_step4::render_order_list` applies
// (`items.iter().filter(|i| !i.is_parent)`); synthetic Step-3 parent-header
// rows are not WeiDU lines.
//
// SPEC: §8.1 (Step-4 review list), §6.7 (line three-colour syntax).

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint,
};
use crate::ui::workspace::widgets::weidu_line;

/// Wireframe `Box` inner padding (`padding: 12`).
const BOX_PADDING: f32 = 12.0;

/// Render the review list for `items` (the active tab's Step-3 ordered
/// items) into the remaining vertical space. `active_tab` is the upper-case
/// tab label for the empty-state copy.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    items: &[Step3ItemState],
    active_tab: &str,
) {
    let leaves: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();

    // The bordered Box (wireframe `Box` = sketchy 1.5px border, shell-bg
    // fill, rounded). It fills the remaining vertical space; the list
    // scrolls inside it.
    let avail = ui.available_size();
    let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    if ui.is_rect_visible(box_rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
    }

    let inner = box_rect.shrink(BOX_PADDING);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(inner.intersect(ui.clip_rect()));

    if leaves.is_empty() {
        // SPEC §8.1: `No components selected on <TAB>.` in faint type.
        child.label(
            egui::RichText::new(format!("No components selected on {active_tab}."))
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
        );
    } else {
        // SPEC §8.1: line-number column auto-sizes to the total count
        // (no leading zeros). Width via the wireframe formula in
        // `weidu_line` (`String(n).length * 9 + 4`), based on the largest
        // line number = `leaves.len()`.
        let max_digits = leaves.len().to_string().len();
        let lineno_w = weidu_line::lineno_column_width(max_digits);

        egui::ScrollArea::vertical()
            .id_salt(("workspace_step4_review_scroll", active_tab.to_string()))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                for (i, item) in leaves.iter().enumerate() {
                    weidu_line::render_weidu_line(ui, palette, item, Some(i + 1), lineno_w);
                }
            });
    }

    // Advance the parent placer by exactly the bounded Box rect (the child
    // UI does not advance it) so the workspace nav bar below stays pinned.
    ui.allocate_rect(box_rect, egui::Sense::hover());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(tp: &str, id: &str, label: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp.to_string(),
            component_id: id.to_string(),
            mod_name: tp.to_string(),
            component_label: label.to_string(),
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }
    fn parent(tp: &str) -> Step3ItemState {
        let mut p = leaf(tp, "__PARENT__", "");
        p.is_parent = true;
        p
    }

    /// SPEC §8.1: the line-number column width grows with the total leaf
    /// count's digit length (1→9..→ wider for ≥10, ≥100). The filter
    /// excludes synthetic parent rows so the count is the installable-leaf
    /// count (matches BIO `render_order_list`).
    #[test]
    fn lineno_width_keys_off_leaf_count_not_total() {
        let mut items = vec![parent("A.TP2")];
        for n in 0..12 {
            items.push(leaf("A.TP2", &n.to_string(), "x"));
        }
        // 12 leaves → max line number 12 → 2 digits.
        let leaves: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();
        assert_eq!(leaves.len(), 12);
        let w2 = weidu_line::lineno_column_width(leaves.len().to_string().len());
        let w1 = weidu_line::lineno_column_width(1);
        assert!(w2 > w1, "2-digit column wider than 1-digit");
    }
}
