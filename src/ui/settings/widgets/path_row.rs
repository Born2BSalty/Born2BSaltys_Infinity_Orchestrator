// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `path_row` — single row in the Paths / Tools sub-tabs.
//
// Per SPEC §11.2 (updated):
//   - Top line: label (Poppins medium) | input (FiraCode Nerd, tinted border)
//     | `browse...` button. The input fills the remaining width — no hint slot
//     to the right.
//   - Bottom line: per-row status message (warning / error reason), always
//     reserved at a fixed height so row spacing doesn't jitter when validation
//     flips a hint on/off. Aligned under the input column so the eye tracks
//     it as belonging to the input above.
//
// Border tint on the input + color on the status text both come from
// `PathStatusTone` — Errors red, Warnings yellow, Ok subtle green, Neutral
// default. Reason text is painted directly to avoid the
// `allocate_ui_with_layout` pitfall where the inner content's `min_rect`
// expands the parent allocation beyond the slot.

// rationale: `f32 as u8` casts are pixel roundings of small positive
// constants — correct by construction (Cat 2); the row renderer's argument
// count mirrors its inputs and a struct wrapper would not aid clarity (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]

use eframe::egui;

use crate::ui::orchestrator::widgets::{InputOpts, redesign_text_input};
use crate::ui::settings::state_settings::PathStatusTone;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent_deep, redesign_border_strong,
    redesign_input_bg, redesign_pill_danger, redesign_success_soft, redesign_text_faint,
    redesign_text_primary, redesign_warning_soft,
};

/// Picker style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRowMode {
    Folder,
    File,
}

const LABEL_WIDTH_PX: f32 = 160.0;
const BROWSE_W_PX: f32 = 80.0;
const ROW_GAP_PX: f32 = 6.0;
const WARNING_LINE_HEIGHT_PX: f32 = 14.0;
const ROW_VERTICAL_GAP_PX: f32 = 6.0;

/// One row in the Paths / Tools sections.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    mono_value: &mut String,
    hint: Option<&str>,
    tone: PathStatusTone,
    mode: PathRowMode,
    mut on_change: impl FnMut(),
) {
    ui.vertical(|ui| {
        // Capture the input rect's x-range so the warning line below aligns
        // under it.
        let mut input_x_range: Option<(f32, f32)> = None;

        ui.horizontal(|ui| {
            // Label (fixed width column for alignment across rows).
            let (label_rect, _) =
                ui.allocate_exact_size(egui::vec2(LABEL_WIDTH_PX, 26.0), egui::Sense::hover());
            ui.painter().text(
                egui::pos2(label_rect.left(), label_rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
                redesign_text_primary(palette),
            );

            // Input takes whatever space is left after the trailing browse
            // button. No hint slot on the right — the hint lives on the line
            // below now.
            let reserved = BROWSE_W_PX + ROW_GAP_PX;
            let edit_width = (ui.available_width() - reserved).max(120.0);

            let border_color = match tone {
                PathStatusTone::Neutral => redesign_border_strong(palette),
                PathStatusTone::Success => redesign_success_soft(palette),
                PathStatusTone::Warning => redesign_warning_soft(palette),
                PathStatusTone::Error => redesign_pill_danger(palette),
            };
            let pre = mono_value.clone();
            let margin = egui::Margin::symmetric(6, 4);
            // Shared input primitive — sketchy border on the OUTER
            // (allocated) box, not egui's margin-inset inner rect (the
            // app-wide indented-input fix); the per-row status tone is
            // passed through as the border color.
            let response = redesign_text_input(
                ui,
                palette,
                InputOpts {
                    edit: egui::TextEdit::singleline(mono_value)
                        .font(egui::FontId::new(
                            12.0,
                            egui::FontFamily::Name("firacode_nerd".into()),
                        ))
                        .text_color(redesign_text_primary(palette))
                        .background_color(redesign_input_bg(palette))
                        .margin(margin),
                    margin,
                    size: egui::vec2(edit_width, 24.0),
                    border: Some(border_color),
                },
            );
            input_x_range = Some((response.rect.left(), response.rect.right()));

            if response.changed() || *mono_value != pre {
                on_change();
            }

            // Browse button (opens rfd folder/file picker inline on click).
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("browse\u{2026}")
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_accent_deep(palette)),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    )),
                )
                .clicked()
            {
                let picked = match mode {
                    PathRowMode::Folder => rfd::FileDialog::new().pick_folder(),
                    PathRowMode::File => rfd::FileDialog::new().pick_file(),
                };
                if let Some(path) = picked {
                    let s = path.to_string_lossy().to_string();
                    if s != *mono_value {
                        *mono_value = s;
                        on_change();
                    }
                }
            }
        });

        // Warning line — always allocate the same vertical space so rows
        // don't shift when a status appears/disappears. When there's a hint,
        // paint it aligned under the input column; when not, the slot stays
        // blank.
        let (warning_rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), WARNING_LINE_HEIGHT_PX),
            egui::Sense::hover(),
        );
        if let (Some(hint_text), Some((x0, x1))) = (hint, input_x_range)
            && !hint_text.is_empty()
        {
            let color = hint_color(palette, tone);
            let painter = ui.painter();
            // Clip text to the input's x-range so a long reason never
            // bleeds into the browse button area.
            let clip = egui::Rect::from_min_max(
                egui::pos2(x0, warning_rect.top()),
                egui::pos2(x1, warning_rect.bottom()),
            );
            let prior_clip = painter.clip_rect();
            let painter = painter.with_clip_rect(clip);
            painter.text(
                egui::pos2(x0 + 2.0, warning_rect.center().y),
                egui::Align2::LEFT_CENTER,
                hint_text,
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
                color,
            );
            let _ = prior_clip;
        }
    });
    ui.add_space(ROW_VERTICAL_GAP_PX);
}

fn hint_color(palette: ThemePalette, tone: PathStatusTone) -> egui::Color32 {
    match tone {
        PathStatusTone::Neutral => redesign_text_faint(palette),
        PathStatusTone::Success => redesign_success_soft(palette),
        PathStatusTone::Warning => redesign_warning_soft(palette),
        PathStatusTone::Error => redesign_pill_danger(palette),
    }
}
