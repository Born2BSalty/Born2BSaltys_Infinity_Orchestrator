// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Active-tab open-seam cover: paints the fill color over the panel's top
//! border segment under the selected tab so the tab visually merges into its
//! content panel.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_shell_bg,
};

/// Margin above the panel's top edge, catching both the tab border's inside-stroke
/// height and its AA fringe. Must be at least `REDESIGN_BORDER_WIDTH_PX + AA_fringe`
/// so the active tab's own bottom stroke is fully overwritten.
const COVER_TOP_MARGIN: f32 = 2.5;

/// Margin below the border's bottom edge so the cover's solid (non-AA) region
/// fully brackets the inside-stroke border at every pixel density. Extending
/// into the panel interior is invisible: the cover color equals the panel fill.
const COVER_BOTTOM_MARGIN: f32 = 1.0;

/// Paints a fill-colored cover over the panel border where the active tab overlaps it.
///
/// The caller must supply a `painter` from the panel's own `Ui` and call this
/// after the panel border is stroked, so the cover appears later in the same
/// paint list and reliably overwrites the border segment.
///
/// `tab_rect` is the allocated rect of the active tab. `panel_top_y` is the y
/// coordinate of the top edge of the content panel.
pub fn paint_active_tab_seam_cover(
    painter: &egui::Painter,
    palette: ThemePalette,
    tab_rect: egui::Rect,
    panel_top_y: f32,
) {
    // Snap the cover edges to the device-pixel grid so they abut the active
    // tab's side strokes crisply, instead of leaving a hairline of panel border
    // on one edge or overrunning the stroke on the other when the tab lands on a
    // fractional position.
    let ppp = painter.ctx().pixels_per_point();
    let snap = |v: f32| (v * ppp).round() / ppp;
    let cover = egui::Rect::from_min_max(
        egui::pos2(
            snap(tab_rect.left() + REDESIGN_BORDER_WIDTH_PX),
            snap(panel_top_y - COVER_TOP_MARGIN),
        ),
        egui::pos2(
            snap(tab_rect.right() - REDESIGN_BORDER_WIDTH_PX),
            snap(panel_top_y + REDESIGN_BORDER_WIDTH_PX + COVER_BOTTOM_MARGIN),
        ),
    );
    painter.rect_filled(cover, egui::CornerRadius::ZERO, redesign_shell_bg(palette));
}
