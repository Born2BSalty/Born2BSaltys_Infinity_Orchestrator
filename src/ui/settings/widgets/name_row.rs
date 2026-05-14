// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `name_row` — edit-in-place name field used by the General sub-tab.
//
// Per Phase 4 P4.T8 file inventory:
//   - Display mode: large Poppins label `<user_name>` (or `<click to set>`
//     placeholder) with a small `edit` button on the right.
//   - Edit mode: single-line TextEdit + Done/Cancel buttons.
//   - Caller passes an `&mut String` for the persisted value and an `&mut bool`
//     for the editing toggle.

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent_deep,
    redesign_border_strong, redesign_input_bg, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label_caption: &str,
    user_name: &mut String,
    editing: &mut bool,
    edit_buffer: &mut String,
    mut on_commit: impl FnMut(),
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label_caption)
                .size(11.0)
                .family(egui::FontFamily::Proportional)
                .color(redesign_text_muted(palette)),
        );
    });
    ui.add_space(2.0);

    if *editing {
        ui.horizontal(|ui| {
            let response = ui.add_sized(
                egui::vec2(220.0, 28.0),
                egui::TextEdit::singleline(edit_buffer)
                    .font(egui::FontId::new(
                        16.0,
                        egui::FontFamily::Name("poppins_medium".into()),
                    ))
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .margin(egui::Margin::symmetric(8, 4)),
            );
            ui.painter().rect_stroke(
                response.rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Outside,
            );
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("done")
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
                *user_name = edit_buffer.trim().to_string();
                *editing = false;
                on_commit();
            }
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("cancel")
                            .size(12.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_muted(palette)),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(
                        REDESIGN_BORDER_WIDTH_PX,
                        redesign_border_strong(palette),
                    )),
                )
                .clicked()
            {
                *editing = false;
                edit_buffer.clear();
                edit_buffer.push_str(user_name);
            }
        });
    } else {
        ui.horizontal(|ui| {
            let (display, color) = if user_name.trim().is_empty() {
                (
                    String::from("click to set your name"),
                    redesign_text_faint(palette),
                )
            } else {
                (user_name.clone(), redesign_text_primary(palette))
            };
            ui.label(
                egui::RichText::new(display)
                    .size(20.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(color),
            );
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("edit")
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
                edit_buffer.clear();
                edit_buffer.push_str(user_name);
                *editing = true;
            }
        });
    }
}
