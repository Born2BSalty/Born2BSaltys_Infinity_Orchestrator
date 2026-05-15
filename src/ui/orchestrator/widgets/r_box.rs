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

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// layout constants — correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    redesign_border_strong, redesign_shell_bg, redesign_text_muted, ThemePalette,
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX,
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
        .inner_margin(egui::Margin {
            left: 12,
            right: 12,
            top: 10,
            bottom: 10,
        });

    let muted = redesign_text_muted(palette);

    // The wireframe `.sk-corner-label` has no CSS — it's just plain text as
    // the first child *inside* the box's padding (`Box` is
    // `position:relative; padding:10px 12px`). So the label renders inside
    // the box as the first content line, at normal size — NOT a tiny
    // fieldset-legend straddling the border. Rendering it inside (rather
    // than as a painted overlay above `rect.top()`) also means labeled and
    // unlabeled boxes start at the same Y, so a labeled box stays aligned
    // with an unlabeled sibling in the same row.
    //
    // Boxes are block-level containers (wireframe `Box` is `display:block`):
    // fill the available width so the chassis is flush with its column
    // rather than shrink-wrapping to its content. Every caller wants this;
    // doing it here keeps them from each repeating `ui.set_width(...)`.
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        if let Some(text) = label {
            ui.label(
                egui::RichText::new(text)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(muted),
            );
            ui.add_space(8.0);
        }
        body(ui)
    })
}
