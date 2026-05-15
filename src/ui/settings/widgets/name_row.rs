// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `name_row` — edit-in-place name *control* used by the General sub-tab.
//
// The label + hint + dashed-rule chassis is owned by the caller's
// `settings_row` wrapper (the wireframe wraps `NameRow` in the same
// `SettingsRow` chassis as every other General setting — see
// `wireframe-preview/screens.jsx::NameRow`). This widget renders only the
// right-anchored control:
//   - Display mode: the name (or the `click to set your name` placeholder)
//     + an `edit` button.
//   - Edit mode: a single-line TextEdit (placeholder `@yourhandle`) + a
//     primary `save` button + a `cancel` button.
//
// Caller passes an `&mut String` for the persisted value, an `&mut bool` for
// the editing toggle, and an `&mut String` scratch buffer for the in-progress
// edit text.

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_faint, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    user_name: &mut String,
    editing: &mut bool,
    edit_buffer: &mut String,
    mut on_commit: impl FnMut(),
) {
    ui.horizontal(|ui| {
        if *editing {
            let response = ui.add_sized(
                egui::vec2(200.0, 28.0),
                egui::TextEdit::singleline(edit_buffer)
                    .font(egui::FontId::new(
                        14.0,
                        egui::FontFamily::Name("poppins_medium".into()),
                    ))
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .margin(egui::Margin::symmetric(8, 4))
                    .hint_text(
                        egui::RichText::new("@yourhandle")
                            .size(14.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(redesign_text_faint(palette)),
                    ),
            );
            ui.painter().rect_stroke(
                response.rect,
                egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Outside,
            );
            if redesign_btn(
                ui,
                palette,
                "save",
                BtnOpts {
                    primary: true,
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                *user_name = edit_buffer.trim().to_string();
                *editing = false;
                on_commit();
            }
            if redesign_btn(
                ui,
                palette,
                "cancel",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                *editing = false;
                edit_buffer.clear();
                edit_buffer.push_str(user_name);
            }
        } else {
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
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(color),
            );
            if redesign_btn(
                ui,
                palette,
                "edit",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                edit_buffer.clear();
                edit_buffer.push_str(user_name);
                *editing = true;
            }
        }
    });
}
