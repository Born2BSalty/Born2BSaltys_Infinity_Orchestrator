// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_accent, redesign_border_strong, redesign_shadow, redesign_shell_bg,
    redesign_text_primary, redesign_with_alpha,
};

pub type BtnFlag = bool;

#[derive(Debug, Clone, Copy, Default)]
pub struct BtnOpts {
    pub primary: BtnFlag,
    pub small: BtnFlag,
    pub disabled: BtnFlag,
    pub block: BtnFlag,
}

pub fn redesign_btn(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    opts: BtnOpts,
) -> egui::Response {
    let (pad_x, pad_y, font_size): (f32, f32, f32) = if opts.small {
        (10.0, 4.0, 12.0)
    } else {
        (16.0, 8.0, 14.0)
    };
    let fill = if opts.primary {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let text_color = if opts.primary {
        egui::Color32::from_rgb(0x1a, 0x26, 0x38)
    } else {
        redesign_text_primary(palette)
    };

    let font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));
    let text_galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let width = if opts.block {
        ui.available_width()
    } else {
        pad_x.mul_add(2.0, text_galley.size().x)
    };
    let desired_size = egui::vec2(width, pad_y.mul_add(2.0, text_galley.size().y));

    let sense = if opts.disabled {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(desired_size, sense);

    let pressed = !opts.disabled && response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let (alpha_num, alpha_den) = if opts.disabled { (1, 2) } else { (1, 1) };

        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);

        if opts.primary {
            let shadow_rect = rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            ));
            painter.rect_filled(
                shadow_rect,
                radius,
                redesign_with_alpha(redesign_shadow(palette), alpha_num, alpha_den),
            );
        }

        painter.rect_filled(
            rect,
            radius,
            redesign_with_alpha(fill, alpha_num, alpha_den),
        );

        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_with_alpha(redesign_border_strong(palette), alpha_num, alpha_den),
            ),
            egui::StrokeKind::Inside,
        );

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            redesign_with_alpha(text_color, alpha_num, alpha_den),
        );
    }

    response
}
