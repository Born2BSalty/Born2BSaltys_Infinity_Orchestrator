// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_BUTTON_FONT_SIZE_PX,
    REDESIGN_BUTTON_PADDING_X_PX, REDESIGN_BUTTON_PADDING_Y_PX, REDESIGN_BUTTON_SMALL_FONT_SIZE_PX,
    REDESIGN_BUTTON_SMALL_PADDING_X_PX, REDESIGN_BUTTON_SMALL_PADDING_Y_PX,
    REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_font_medium, redesign_shadow, redesign_shell_bg, redesign_text_faint,
    redesign_text_on_accent, redesign_text_primary,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct BtnOpts {
    pub primary: bool,
    pub small: bool,
    pub disabled: bool,
}

pub fn redesign_btn(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    opts: BtnOpts,
) -> egui::Response {
    let font_size = if opts.small {
        REDESIGN_BUTTON_SMALL_FONT_SIZE_PX
    } else {
        REDESIGN_BUTTON_FONT_SIZE_PX
    };
    let margin = if opts.small {
        egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_SMALL_PADDING_Y_PX),
        )
    } else {
        egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_BUTTON_PADDING_Y_PX),
        )
    };
    let text_color = if opts.disabled {
        redesign_text_faint(palette)
    } else if opts.primary {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_primary(palette)
    };
    let fill = if opts.primary {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let shadow = if opts.primary {
        egui::Shadow {
            offset: [
                crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SHADOW_OFFSET_BTN_PX),
                crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_SHADOW_OFFSET_BTN_PX),
            ],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        }
    } else {
        egui::Shadow::NONE
    };

    let response = egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .shadow(shadow)
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .family(redesign_font_medium())
                    .size(font_size)
                    .color(text_color),
            );
        })
        .response;

    if opts.disabled {
        response
    } else {
        let response = response.interact(egui::Sense::click());
        if response.hovered() {
            ui.painter().rect_stroke(
                response.rect,
                REDESIGN_BORDER_RADIUS_PX,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_text_primary(palette)),
                egui::StrokeKind::Inside,
            );
        }
        if response.is_pointer_button_down_on() {
            ui.painter().rect_stroke(
                response.rect.translate(egui::vec2(
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                    REDESIGN_SHADOW_OFFSET_BTN_PX,
                )),
                REDESIGN_BORDER_RADIUS_PX,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
        }
        response
    }
}
