// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Active-tab open-seam cover: paints the fill color over the panel's top
//! border segment under the selected tab so the tab visually merges into its
//! content panel.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_shell_bg,
};

/// Half-height of the cover rect on each side of the panel top edge, in
/// addition to the border half-width. Provides AA margin to eliminate hairlines.
const COVER_HALF_MARGIN: f32 = 0.5;

/// Paints a fill-colored cover over the panel's top border where the active tab
/// overlaps it, on `egui::Order::Foreground` so it composites above the panel
/// regardless of paint order.
///
/// `tab_rect` is the allocated rect of the active tab. `panel_top_y` is the y
/// coordinate of the top edge of the content panel. `layer_id_salt` is a stable
/// string that identifies this cover site (one per tab surface).
pub fn paint_active_tab_seam_cover(
    ctx: &egui::Context,
    palette: ThemePalette,
    tab_rect: egui::Rect,
    panel_top_y: f32,
    layer_id_salt: &str,
) {
    let half = REDESIGN_BORDER_WIDTH_PX / 2.0 + COVER_HALF_MARGIN;
    let cover = egui::Rect::from_min_max(
        egui::pos2(
            tab_rect.left() + REDESIGN_BORDER_WIDTH_PX,
            panel_top_y - half,
        ),
        egui::pos2(
            tab_rect.right() - REDESIGN_BORDER_WIDTH_PX,
            panel_top_y + half,
        ),
    );

    let layer_id = egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new(("tab_seam_cover", layer_id_salt)),
    );
    ctx.layer_painter(layer_id).rect_filled(
        cover,
        egui::CornerRadius::ZERO,
        redesign_shell_bg(palette),
    );
}
