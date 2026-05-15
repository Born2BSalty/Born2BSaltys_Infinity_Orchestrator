// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{self, BtnOpts};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_ACCOUNT_AVATAR_SIZE_PX, REDESIGN_ACCOUNT_CARD_GAP_PX,
    REDESIGN_ACCOUNT_CARD_PADDING_X_PX, REDESIGN_ACCOUNT_CARD_PADDING_Y_PX,
    REDESIGN_ACCOUNT_PILL_PADDING_X_PX, REDESIGN_ACCOUNT_PILL_PADDING_Y_PX,
    REDESIGN_ACCOUNT_PILL_RADIUS_PX, REDESIGN_AVATAR_FONT_SIZE_PX, REDESIGN_BORDER_RADIUS_PX,
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX,
    REDESIGN_PILL_FONT_SIZE_PX, REDESIGN_SHADOW_OFFSET_BTN_PX, ThemePalette,
    redesign_border_strong, redesign_pill_info, redesign_pill_neutral, redesign_shadow,
    redesign_shell_bg, redesign_text_faint, redesign_text_on_accent, redesign_text_primary,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    service_name: &str,
    initials: &str,
    connected_user: Option<&str>,
    action_label: &str,
) -> egui::Response {
    let connected = connected_user.is_some();
    let mut button_response = None;
    let card_width = ui.available_width();

    let frame_response = egui::Frame::NONE
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_ACCOUNT_CARD_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_ACCOUNT_CARD_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            ui.set_width(
                REDESIGN_ACCOUNT_CARD_PADDING_X_PX
                    .mul_add(-2.0, card_width)
                    .max(0.0),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_ACCOUNT_CARD_GAP_PX;
                render_avatar(ui, palette, initials);

                ui.label(
                    egui::RichText::new(service_name)
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .strong()
                        .color(redesign_text_primary(palette)),
                );

                if let Some(user) = connected_user {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(
                            egui::RichText::new("as")
                                .size(REDESIGN_HINT_FONT_SIZE_PX)
                                .color(redesign_text_faint(palette)),
                        );
                        ui.label(
                            egui::RichText::new(user)
                                .size(REDESIGN_HINT_FONT_SIZE_PX)
                                .strong()
                                .color(redesign_text_primary(palette)),
                        );
                    });
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    button_response = Some(btn::redesign_btn(
                        ui,
                        palette,
                        action_label,
                        BtnOpts {
                            primary: !connected,
                            small: true,
                            disabled: false,
                        },
                    ));
                    render_pill(
                        ui,
                        palette,
                        if connected {
                            "connected"
                        } else {
                            "not connected"
                        },
                        connected,
                    );
                });
            });
        })
        .response;

    button_response.unwrap_or(frame_response)
}

fn render_avatar(ui: &mut egui::Ui, palette: ThemePalette, initials: &str) {
    let size = egui::vec2(
        REDESIGN_ACCOUNT_AVATAR_SIZE_PX,
        REDESIGN_ACCOUNT_AVATAR_SIZE_PX,
    );
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let shadow_rect = rect.translate(egui::vec2(
        REDESIGN_SHADOW_OFFSET_BTN_PX,
        REDESIGN_SHADOW_OFFSET_BTN_PX,
    ));

    ui.painter().rect_filled(
        shadow_rect,
        REDESIGN_BORDER_RADIUS_PX,
        redesign_shadow(palette),
    );
    ui.painter().rect(
        rect,
        REDESIGN_BORDER_RADIUS_PX,
        redesign_shell_bg(palette),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        initials,
        egui::FontId::proportional(REDESIGN_AVATAR_FONT_SIZE_PX),
        redesign_text_primary(palette),
    );
}

fn render_pill(ui: &mut egui::Ui, palette: ThemePalette, text: &str, connected: bool) {
    let fill = if connected {
        redesign_pill_info(palette)
    } else {
        redesign_pill_neutral(palette)
    };

    egui::Frame::NONE
        .fill(fill)
        .corner_radius(REDESIGN_ACCOUNT_PILL_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_ACCOUNT_PILL_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_ACCOUNT_PILL_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(REDESIGN_PILL_FONT_SIZE_PX)
                    .color(redesign_text_on_accent(palette)),
            );
        });
}
