// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX,
    REDESIGN_TOGGLE_HEIGHT_PX, REDESIGN_TOGGLE_KNOB_INSET_PX, REDESIGN_TOGGLE_KNOB_RADIUS_PX,
    REDESIGN_TOGGLE_RADIUS_PX, REDESIGN_TOGGLE_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_hover_overlay, redesign_input_bg, redesign_shell_bg,
    redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, label: &str, value: &mut bool, hint: &str) {
    if label.is_empty() && hint.is_empty() {
        render_toggle(ui, palette, value);
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(hint)
                    .size(REDESIGN_HINT_FONT_SIZE_PX)
                    .color(redesign_text_muted(palette)),
            );
            render_toggle(ui, palette, value);
        });
    });
}

fn render_toggle(ui: &mut egui::Ui, palette: ThemePalette, value: &mut bool) {
    let size = egui::vec2(REDESIGN_TOGGLE_WIDTH_PX, REDESIGN_TOGGLE_HEIGHT_PX);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *value = !*value;
    }

    let fill = if *value {
        redesign_accent(palette)
    } else {
        redesign_input_bg(palette)
    };
    let track_fill = if response.hovered() {
        redesign_hover_overlay(palette)
    } else {
        fill
    };
    ui.painter().rect(
        rect,
        REDESIGN_TOGGLE_RADIUS_PX,
        track_fill,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );

    let knob_x = if *value {
        rect.right() - REDESIGN_TOGGLE_KNOB_INSET_PX - REDESIGN_TOGGLE_KNOB_RADIUS_PX
    } else {
        rect.left() + REDESIGN_TOGGLE_KNOB_INSET_PX + REDESIGN_TOGGLE_KNOB_RADIUS_PX
    };
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        REDESIGN_TOGGLE_KNOB_RADIUS_PX,
        redesign_shell_bg(palette),
    );
}
