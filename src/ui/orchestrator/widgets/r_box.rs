// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `Box` primitive — sketchy-bordered chassis with optional top-left corner
// label.
//
// Mirrors `wireframe-preview/screens.jsx::Box` (line 11-20):
//   sketchyBorder { border: 1.5px solid var(--border-strong); borderRadius: 3px }
//   padding: 10px 12px
//   position: relative
//   {label && <span className="sk-corner-label">{label}</span>}
//   {children}
//
// SPEC: §1.2 (sketchy aesthetic).

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_muted,
};

/// Render a `Box` chassis around a body callback.
///
/// - `label`  → optional Poppins 10px top-left corner label (per
///   wireframe `.sk-corner-label`).
/// - `body`   → closure called inside the padded chassis with the inner `Ui`.
///
/// The chassis paints `shell_bg` fill, 1.5px `border_strong` stroke, 3px
/// rounded corners, and 10×12 internal padding.
pub fn redesign_box<R>(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: Option<&str>,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 });

    // Boxes are block-level containers (wireframe `Box` is `display:block`):
    // fill the available width so the chassis is flush with its column
    // rather than shrink-wrapping to its content. Every caller wants this;
    // doing it here keeps them from each repeating `ui.set_width(...)`.
    let response = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        body(ui)
    });

    if let Some(label_text) = label {
        // Paint the corner label over the top-left of the box's stroke. The
        // wireframe puts it slightly outside the border (negative offsets),
        // but to keep it inside our allocated rect we anchor it just inside
        // the top-left corner.
        let rect = response.response.rect;
        let painter = ui.painter();
        let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
        let bg = redesign_shell_bg(palette);
        let muted = redesign_text_muted(palette);

        let galley = painter.layout_no_wrap(label_text.to_string(), font.clone(), muted);
        let label_size = galley.size();
        let label_pos = egui::pos2(rect.left() + 8.0, rect.top() - label_size.y * 0.5);
        let label_rect = egui::Rect::from_min_size(
            label_pos - egui::vec2(2.0, 0.0),
            egui::vec2(label_size.x + 4.0, label_size.y),
        );
        // Background patch so the label "breaks" the border line cleanly.
        painter.rect_filled(label_rect, 0.0, bg);
        painter.galley(label_pos, galley, muted);
    }

    response
}
