// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_input_bg, redesign_text_faint, redesign_text_primary,
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
            let margin = egui::Margin::symmetric(8, 4);
            let _response = redesign_text_input(
                ui,
                palette,
                InputOpts {
                    edit: egui::TextEdit::singleline(edit_buffer)
                        .font(egui::FontId::new(
                            14.0,
                            egui::FontFamily::Name("poppins_medium".into()),
                        ))
                        .text_color(redesign_text_primary(palette))
                        .background_color(redesign_input_bg(palette))
                        .margin(margin)
                        .hint_text(
                            egui::RichText::new("@yourhandle")
                                .size(14.0)
                                .family(egui::FontFamily::Name("poppins_medium".into()))
                                .color(redesign_text_faint(palette)),
                        ),
                    margin,
                    size: egui::vec2(200.0, 28.0),
                    border: None,
                },
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
