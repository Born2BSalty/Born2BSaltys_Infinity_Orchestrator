// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_strip` — file-folder tab pattern used at the top of the Settings page
// (and reused by Phase 6's workspace progress bar with different content).
//
// Per Phase 4 P4.T1 (file-inventory entry):
//   - Renders one tab per `T` (where `T: Copy + Eq + TabLabel`).
//   - Active tab visually merges with the body Box below — same fill, no
//     bottom border for the active tab, 1.5px sketchy border everywhere else.
//   - The widget body then renders the active tab's content inside a single
//     Box that fills the remaining vertical space.
//
// Mirrors the wireframe file-folder treatment (`screens.jsx::TabStrip`).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_chrome_bg, redesign_hover_overlay, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

/// Trait satisfied by tab enums — supplies the visible label.
pub trait TabLabel: Copy + Eq {
    fn label(self) -> &'static str;
}

/// Render the tab strip + the active tab's body inside a single merged Box.
///
/// - `tabs`     — iterable of all available tabs in render order.
/// - `current`  — currently-active tab (mutated on click).
/// - `body`     — closure that paints the active tab's content; called with
///                the inner `Ui` already padded inside the merged Box.
pub fn render<T: TabLabel>(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    tabs: &[T],
    current: &mut T,
    body: impl FnOnce(&mut egui::Ui),
) {
    let tab_height = 30.0;
    let body_padding = 14.0;

    // Tabs have rounded TOP corners only — the bottom is flat so the active
    // tab can merge into the body's flat top edge without a corner clash.
    let tab_corner = egui::CornerRadius {
        nw: REDESIGN_BORDER_RADIUS_PX as u8,
        ne: REDESIGN_BORDER_RADIUS_PX as u8,
        sw: 0,
        se: 0,
    };
    // Body has SQUARE top corners (the tabs sit on top so any top curvature
    // would clash with them) and rounded BOTTOM corners (aesthetic).
    let body_corner = egui::CornerRadius {
        nw: 0,
        ne: 0,
        sw: REDESIGN_BORDER_RADIUS_PX as u8,
        se: REDESIGN_BORDER_RADIUS_PX as u8,
    };
    let border = egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
    let active_fill = redesign_shell_bg(palette);
    let idle_fill = redesign_chrome_bg(palette);

    // Track the active tab's x-range so we can mask the seam between tab
    // bottom and body top after both are painted — this is what makes the
    // active tab visually flow into the body (the wireframe's
    // `marginBottom: -1.5px` file-folder pattern).
    let mut active_x_range: Option<(f32, f32)> = None;

    // Tab row. No leading add_space — tabs share the parent's left edge with
    // the body Box, so the active tab's left border continues the body's
    // left border (wireframe `screens.jsx:3941` — tab row has no left pad).
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for &tab in tabs {
            let active = tab == *current;
            let label = tab.label();
            let font = egui::FontId::new(
                13.0,
                egui::FontFamily::Name("poppins_medium".into()),
            );
            let galley = ui.painter().layout_no_wrap(
                label.to_string(),
                font.clone(),
                redesign_text_primary(palette),
            );
            let tab_w = galley.size().x + 26.0;
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(tab_w, tab_height),
                egui::Sense::click(),
            );
            let painter = ui.painter();

            let fill = if active {
                active_fill
            } else {
                idle_fill
            };
            painter.rect_filled(rect, tab_corner, fill);
            if !active && response.hovered() {
                painter.rect_filled(rect, tab_corner, redesign_hover_overlay(palette));
            }
            // Full 4-sided stroke with StrokeKind::Inside so the border sits
            // inside the rect (same alignment as the body's stroke). The
            // active tab's bottom stroke gets masked in the seam pass below.
            painter.rect_stroke(rect, tab_corner, border, egui::StrokeKind::Inside);

            if active {
                active_x_range = Some((rect.left(), rect.right()));
            }

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

            if response.clicked() {
                *current = tab;
            }
        }
    });

    // Remove the vertical gap egui inserts after the horizontal row so the
    // body Box's top edge sits exactly where the tabs ended.
    let item_gap_y = ui.spacing().item_spacing.y;
    ui.add_space(-item_gap_y);

    // Body Box — same fill as the active tab; square top corners.
    let avail = ui.available_size();
    let (body_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(body_rect, body_corner, active_fill);
    painter.rect_stroke(body_rect, body_corner, border, egui::StrokeKind::Inside);

    // Mask the seam at tab.bottom() == body.top() within the active tab's
    // x-range, inset by the border width on both sides so the active tab's
    // left and right strokes flow cleanly into the body's left/right strokes.
    if let Some((x0, x1)) = active_x_range {
        let seam_y = body_rect.top();
        let mask = egui::Rect::from_min_max(
            egui::pos2(x0 + REDESIGN_BORDER_WIDTH_PX, seam_y - REDESIGN_BORDER_WIDTH_PX),
            egui::pos2(x1 - REDESIGN_BORDER_WIDTH_PX, seam_y + REDESIGN_BORDER_WIDTH_PX),
        );
        painter.rect_filled(mask, 0.0, active_fill);
    }

    let inner_rect = body_rect.shrink(body_padding);
    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(inner_rect),
        |ui| {
            ui.set_clip_rect(inner_rect);
            ui.vertical(|ui| body(ui));
        },
    );
}
