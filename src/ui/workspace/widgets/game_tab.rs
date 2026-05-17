// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `game_tab` — the ONE shared workspace GameTab, used identically by Step 2
// (`step2_tab_row`), Step 4 (`workspace_step4`), and (per the Step-3 C4
// cascade) Step 3. Wireframe `GameTab` (`screens.jsx:1609-1637`; same shape
// as the Settings `tab_strip` `:485-503`): a real **tab**, not a closed
// button — rounded-top corners, a fill, and a border on the **top + left +
// right ONLY**.
//
// ## No bottom bar — in any state (the uniform fix)
//
// History: Step 2 and Step 4 each carried their own copy of this painter,
// and both stroked **all four sides** then tried to hide the bottom — the
// active tab over-painted its bottom in shell-bg, the idle tab relied on its
// border-strong bottom *coinciding* with the pane's top border. That left a
// visible bottom bar under idle tabs (the coincidence is fragile) and was
// non-uniform across steps. This single widget replaces both copies and
// simply **never strokes a bottom edge** — the tab is genuinely open at the
// bottom, so it cannot render a bar in any state, independent of where the
// pane border lands.
//
// The fill still extends `REDESIGN_BORDER_WIDTH_PX` past the bottom into the
// pane seam: the caller pulls the surface below up by that same overlap (the
// Step-2 `TAB_TO_GRID_OVERLAP` negative seam), so the **active** tab's
// shell-bg fill flows over the pane's top border and the tab merges into the
// pane; **idle** tabs (chrome-bg) sit on the pane's own top border with no
// second line of their own.
//
// The top+left+right-only border is achieved by stroking egui's native
// rounded-top `RectShape` through a painter whose clip excludes the bottom
// band — so the two top corners stay correct **by construction** (no
// hand-rolled corner arcs, the prior #1 defect) while the bottom run is
// simply clipped away.
//
// SPEC: §6.4 (Step-2 GameTabs), §7 (Step-3 C4 chrome), §8.1 (Step-4 tab
//       strip) — one wireframe component, one widget.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

/// Tab height — wireframe `GameTab` action sub-row `height: 30`.
pub const TAB_H: f32 = 30.0;
/// Gap between GameTabs in a row — wireframe outer row `gap: 4`.
pub const TAB_GAP: f32 = 4.0;
/// Wireframe `GameTab` horizontal padding (`padding: 5px 14px`).
const TAB_PAD_X: f32 = 14.0;
/// Label font — wireframe `GameTab` text (Poppins-medium 13).
const TAB_FONT_SIZE: f32 = 13.0;

/// Render one GameTab into the current layout. `current` holds the active
/// tab's label; clicking this tab sets `*current = label`. `active` is
/// `current == label`. Lay tabs out in a `ui.horizontal` with
/// `ui.spacing_mut().item_spacing.x = TAB_GAP` (the established caller
/// pattern in Step 2 / Step 4).
pub fn game_tab(ui: &mut egui::Ui, palette: ThemePalette, label: &str, current: &mut String) {
    let active = current == label;
    let font = egui::FontId::new(
        TAB_FONT_SIZE,
        egui::FontFamily::Name("poppins_medium".into()),
    );
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        redesign_text_primary(palette),
    );
    let tab_w = galley.size().x + TAB_PAD_X * 2.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(tab_w, TAB_H), egui::Sense::click());

    // Rounded TOP corners only (wireframe `borderRadius: "4px 4px 0 0"`);
    // the two bottom corners are square — the tab opens into the pane.
    let corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_PX as u8,
        ne: REDESIGN_BORDER_RADIUS_PX as u8,
        sw: 0,
        se: 0,
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let fill = if active {
            redesign_shell_bg(palette)
        } else {
            redesign_chrome_bg(palette)
        };

        // Fill extends one border-width past the bottom into the pane seam
        // (wireframe `marginBottom: -1.5px`). Drawn UNCLIPPED so the active
        // tab's shell-bg flows over the pane's top border (the caller's
        // negative seam pulls the pane up by the same overlap) and the tab
        // merges into the pane. No bottom *border* is ever drawn.
        let box_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, rect.max.y + REDESIGN_BORDER_WIDTH_PX),
        );
        painter.rect_filled(box_rect, corner, fill);
        if !active && response.hovered() {
            painter.rect_filled(box_rect, corner, redesign_hover_overlay(palette));
        }

        // Border on TOP + LEFT + RIGHT only. Stroke egui's native
        // rounded-top rect (top corners correct by construction) through a
        // painter whose clip excludes the bottom band, so the bottom edge is
        // never painted — for active AND idle alike. No hand-rolled arcs.
        let stroke = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
        let border_clip = egui::Rect::from_min_max(
            egui::pos2(box_rect.min.x - 4.0, box_rect.min.y - 4.0),
            egui::pos2(
                box_rect.max.x + 4.0,
                box_rect.max.y - REDESIGN_BORDER_WIDTH_PX * 1.5,
            ),
        );
        painter.with_clip_rect(border_clip).rect_stroke(
            box_rect,
            corner,
            stroke,
            egui::StrokeKind::Inside,
        );

        let text_color = if active {
            redesign_text_primary(palette)
        } else {
            redesign_text_muted(palette)
        };
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }

    if response.clicked() {
        *current = label.to_string();
    }
}
