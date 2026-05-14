// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `account_card` — service card chassis used by `tab_accounts`.
//
// Per Phase 4 P4.T5 + P4.T8 file inventory: 32×32 avatar square with
// initials, service name, connection-state Pill, primary action button on
// the right. Rendered inside a redesign Box.
//
// Connection state:
//   - `Connected { user_label }` → info-tone Pill `as <user_label>`,
//     button label is the supplied `disconnect_label` (e.g. "disconnect"
//     for GitHub, "view" for Nexus when wired in v2).
//   - `NotConnected`             → neutral Pill `not connected`, button
//     label is the supplied `connect_label`.

use eframe::egui;

use crate::ui::orchestrator::widgets::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_accent_deep, redesign_border_strong, redesign_pill_info, redesign_pill_neutral,
    redesign_pill_text, redesign_text_muted, redesign_text_primary,
};

/// Connection state of an account card.
#[derive(Debug, Clone)]
pub enum CardState<'a> {
    Connected { user_label: &'a str },
    NotConnected,
}

/// Render one account card. Returns `true` if the primary action button
/// was clicked this frame.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    initials: &str,
    service_name: &str,
    state: CardState<'_>,
    connect_label: &str,
    disconnect_label: &str,
) -> bool {
    let mut clicked = false;
    redesign_box(ui, palette, None, |ui| {
        ui.horizontal(|ui| {
            // Avatar square (32×32 accent fill, initials text).
            let avatar_size = 32.0;
            let (avatar_rect, _) = ui.allocate_exact_size(
                egui::vec2(avatar_size, avatar_size),
                egui::Sense::hover(),
            );
            let painter = ui.painter();
            let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
            painter.rect_filled(avatar_rect, radius, redesign_accent(palette));
            painter.rect_stroke(
                avatar_rect,
                radius,
                egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
                egui::StrokeKind::Inside,
            );
            painter.text(
                avatar_rect.center(),
                egui::Align2::CENTER_CENTER,
                initials,
                egui::FontId::new(
                    12.0,
                    egui::FontFamily::Name("poppins_bold".into()),
                ),
                egui::Color32::from_rgb(0x1a, 0x26, 0x38),
            );

            ui.add_space(10.0);

            // Service name + state pill stacked vertically.
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(service_name)
                        .size(14.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
                ui.add_space(2.0);

                let (pill_label, pill_fill) = match &state {
                    CardState::Connected { user_label } => (
                        format!("as {user_label}"),
                        redesign_pill_info(palette),
                    ),
                    CardState::NotConnected => (
                        String::from("not connected"),
                        redesign_pill_neutral(palette),
                    ),
                };
                draw_pill(ui, palette, &pill_label, pill_fill);
            });

            // Push action button to the right edge.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = match state {
                    CardState::Connected { .. } => disconnect_label,
                    CardState::NotConnected => connect_label,
                };
                let button = egui::Button::new(
                    egui::RichText::new(label)
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_accent_deep(palette)),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ));
                if ui.add(button).clicked() {
                    clicked = true;
                }
            });
        });
    });
    ui.add_space(8.0);
    clicked
}

fn draw_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    fill: egui::Color32,
) {
    let font = egui::FontId::new(
        11.0,
        egui::FontFamily::Name("poppins_medium".into()),
    );
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), redesign_pill_text(palette));
    let pad = egui::vec2(10.0, 4.0);
    let size = egui::vec2(
        galley.size().x + pad.x * 2.0,
        galley.size().y + pad.y * 2.0,
    );
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same((REDESIGN_BORDER_RADIUS_PX + 2.0) as u8);
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        redesign_text_muted(palette),
    );
}
