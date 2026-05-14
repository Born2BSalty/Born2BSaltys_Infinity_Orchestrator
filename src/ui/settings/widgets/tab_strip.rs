// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_SETTINGS_TAB_GAP_PX, REDESIGN_SETTINGS_TAB_PADDING_X_PX,
    REDESIGN_SETTINGS_TAB_PADDING_Y_PX, REDESIGN_SETTINGS_TAB_RADIUS_PX, REDESIGN_TAB_FONT_SIZE_PX,
    ThemePalette, redesign_border_strong, redesign_chrome_bg, redesign_shell_bg,
    redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, active_tab: &mut SettingsTab) {
    let mut active_rect = None;
    let row_response = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_SETTINGS_TAB_GAP_PX;

        for tab in SettingsTab::ORDERED {
            let active = *active_tab == tab;
            let fill = if active {
                redesign_shell_bg(palette)
            } else {
                redesign_chrome_bg(palette)
            };
            let stroke =
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette));
            let text = egui::RichText::new(tab.label())
                .size(REDESIGN_TAB_FONT_SIZE_PX)
                .color(redesign_text_primary(palette));

            let response = egui::Frame::NONE
                .fill(fill)
                .stroke(stroke)
                .corner_radius(egui::CornerRadius {
                    nw: REDESIGN_SETTINGS_TAB_RADIUS_PX as u8,
                    ne: REDESIGN_SETTINGS_TAB_RADIUS_PX as u8,
                    sw: 0,
                    se: 0,
                })
                .inner_margin(egui::Margin::symmetric(
                    REDESIGN_SETTINGS_TAB_PADDING_X_PX as i8,
                    REDESIGN_SETTINGS_TAB_PADDING_Y_PX as i8,
                ))
                .show(ui, |ui| ui.label(if active { text.strong() } else { text }))
                .response
                .interact(egui::Sense::click());

            if active {
                active_rect = Some(response.rect);
            }

            if response.clicked() {
                *active_tab = tab;
            }
        }
    });

    let line_y = row_response.response.rect.bottom();
    let rect = ui.available_rect_before_wrap();
    ui.painter().line_segment(
        [
            egui::pos2(row_response.response.rect.left(), line_y),
            egui::pos2(rect.right(), line_y),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );
    if let Some(active_rect) = active_rect {
        ui.painter().line_segment(
            [active_rect.left_bottom(), active_rect.right_bottom()],
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_shell_bg(palette)),
        );
    }
}
