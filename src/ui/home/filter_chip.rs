// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_primary,
};

const CHIP_RADIUS_U8: u8 = 14;
const CHIP_PAD_X_PX: f32 = 12.0;
const CHIP_PAD_Y_PX: f32 = 7.0;
const ON_ACCENT_TEXT: egui::Color32 = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    count: usize,
    active: bool,
) -> egui::Response {
    let label_color = if active {
        ON_ACCENT_TEXT
    } else {
        redesign_text_primary(palette)
    };
    let count_color = if active {
        ON_ACCENT_TEXT
    } else {
        redesign_text_faint(palette)
    };

    let label_font = egui::FontId::new(
        13.0,
        if active {
            egui::FontFamily::Name("poppins_medium".into())
        } else {
            egui::FontFamily::Name("poppins_light".into())
        },
    );
    let count_font = egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into()));

    let mut job = egui::text::LayoutJob::default();
    job.append(
        label,
        0.0,
        egui::TextFormat {
            font_id: label_font,
            color: label_color,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    job.append(
        &format!(" ({count})"),
        0.0,
        egui::TextFormat {
            font_id: count_font,
            color: count_color,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    let galley = ui.fonts(|f| f.layout_job(job));

    let desired = egui::vec2(
        CHIP_PAD_X_PX.mul_add(2.0, galley.size().x),
        CHIP_PAD_Y_PX.mul_add(2.0, galley.size().y),
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let rect = if response.is_pointer_button_down_on() {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(CHIP_RADIUS_U8);
        let fill = if active {
            redesign_accent(palette)
        } else {
            redesign_shell_bg(palette)
        };
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );

        let pos = rect.center() - galley.size() * 0.5;
        painter.galley(pos, galley, label_color);
    }

    response
}
