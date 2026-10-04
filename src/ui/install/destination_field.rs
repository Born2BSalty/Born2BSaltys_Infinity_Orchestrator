// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{InputOpts, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong, redesign_input_bg,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

const BROWSE_W_PX: f32 = 96.0;
const LABEL: &str = "destination folder";
const PLACEHOLDER: &str = "D:\\BG2EE_install_test";

const FORM_INPUT_MARGIN: egui::Margin = egui::Margin {
    left: 12,
    right: 12,
    top: 8,
    bottom: 8,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    destination: &mut String,
    error: bool,
) -> bool {
    let mut changed = false;

    let border = if error {
        Some(crate::ui::shared::redesign_tokens::redesign_error(palette))
    } else {
        None
    };

    ui.label(
        egui::RichText::new(LABEL)
            .size(14.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);

    let box_h = ui.fonts(|f| {
        f.row_height(&egui::FontId::new(
            14.0,
            egui::FontFamily::Name("poppins_light".into()),
        ))
    }) + f32::from(FORM_INPUT_MARGIN.top)
        + f32::from(FORM_INPUT_MARGIN.bottom);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        let reserved = BROWSE_W_PX + 8.0;
        let edit_width = (ui.available_width() - reserved).max(120.0);

        let pre = destination.clone();
        let response = redesign_text_input(
            ui,
            palette,
            InputOpts {
                edit: egui::TextEdit::singleline(destination)
                    .font(egui::FontId::new(
                        12.0,
                        egui::FontFamily::Name("firacode_nerd".into()),
                    ))
                    .hint_text(
                        egui::RichText::new(PLACEHOLDER)
                            .family(egui::FontFamily::Name("firacode_nerd".into()))
                            .color(redesign_text_faint(palette)),
                    )
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .vertical_align(egui::Align::Center)
                    .margin(FORM_INPUT_MARGIN),
                margin: FORM_INPUT_MARGIN,
                size: egui::vec2(edit_width, box_h),
                border,
            },
        );
        if response.changed() || *destination != pre {
            changed = true;
        }

        if ui
            .add_sized(
                egui::vec2(BROWSE_W_PX, box_h),
                egui::Button::new(
                    egui::RichText::new("browse\u{2026}")
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                )
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                )),
            )
            .clicked()
            && let Some(path) = rfd::FileDialog::new().pick_folder()
        {
            let picked = path.to_string_lossy().to_string();
            if picked != *destination {
                *destination = picked;
                changed = true;
            }
        }
    });

    changed
}
