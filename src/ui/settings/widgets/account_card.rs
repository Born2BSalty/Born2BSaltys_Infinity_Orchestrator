// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `account_card` — service card chassis used by `tab_accounts`.
//
// Per SPEC §11.4 + wireframe `screens.jsx::AccountCard` (line 3884–3917):
// single horizontal row inside a redesign Box, ~10px vertical / 16px
// horizontal padding, `align-items: center`:
//
//   [36×36 avatar] [service name] [optional "as @user" hand-style label
//                                  when connected]
//                                                   …push to right…
//                                                   [pill] [Btn]
//
// Avatar: shell-bg fill, sketchy 1.5px border, 2×2 drop shadow, initials in
// poppins_bold. (Not accent-filled — that conflated the avatar with the
// active rail item; the wireframe uses the neutral notebook-card treatment.)
//
// Pill: small chip — no border, just the tone-matched fill with rounded
// ends. Text is `connected` (info tone) or `not connected` (neutral tone).
//
// Btn: primary fill (accent + drop shadow) when the action is the call to
// action — i.e. when NOT connected (the "connect" affordance is what we
// want the user to notice). Non-primary when connected ("disconnect" is a
// secondary action). The `disabled` flag greys the button at 50% alpha and
// suppresses the click event, for stub services like Nexus / Mega that we
// haven't wired yet.

use eframe::egui;

use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_border_strong, redesign_pill_info, redesign_pill_neutral,
    redesign_pill_text, redesign_shadow, redesign_shell_bg, redesign_text_faint,
    redesign_text_primary,
};

/// Connection state of an account card.
#[derive(Debug, Clone)]
pub enum CardState<'a> {
    Connected { user_label: &'a str },
    NotConnected,
}

/// Render one account card. Returns `true` if the primary action button
/// was clicked this frame. Returns `false` when `disabled` is true (clicks
/// are suppressed alongside the visual grey-out).
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    initials: &str,
    service_name: &str,
    state: CardState<'_>,
    connect_label: &str,
    disconnect_label: &str,
    disabled: bool,
) -> bool {
    let mut clicked = false;
    redesign_box(ui, palette, None, |ui| {
        ui.horizontal(|ui| {
            // Avatar — 36×36, shell-bg fill, sketchy border, 2×2 drop shadow.
            let avatar_size = 36.0;
            let (avatar_rect, _) =
                ui.allocate_exact_size(egui::vec2(avatar_size, avatar_size), egui::Sense::hover());
            let painter = ui.painter();
            let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
            let shadow_rect = avatar_rect.translate(egui::vec2(
                REDESIGN_SHADOW_OFFSET_BTN_PX,
                REDESIGN_SHADOW_OFFSET_BTN_PX,
            ));
            painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));
            painter.rect_filled(avatar_rect, radius, redesign_shell_bg(palette));
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
                egui::FontId::new(14.0, egui::FontFamily::Name("poppins_bold".into())),
                redesign_text_primary(palette),
            );

            ui.add_space(12.0);

            // Service name (always shown).
            ui.label(
                egui::RichText::new(service_name)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );

            // "as @user" hand-style label, only when connected. We always
            // render with a leading `@` regardless of whether the caller's
            // user_label already has one — strips a duplicate if present so
            // we never emit `@@b2bs`.
            if let CardState::Connected { user_label } = &state {
                ui.add_space(8.0);
                let handle = user_label.trim_start_matches('@');
                ui.label(
                    egui::RichText::new(format!("as @{handle}"))
                        .size(13.0)
                        .family(egui::FontFamily::Proportional)
                        .color(redesign_text_faint(palette)),
                );
            }

            // Right-anchored cluster: pill + button.
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    let (button_label, button_primary) = match &state {
                        CardState::Connected { .. } => (disconnect_label, false),
                        CardState::NotConnected => (connect_label, true),
                    };
                    let response = redesign_btn(
                        ui,
                        palette,
                        button_label,
                        BtnOpts {
                            primary: button_primary,
                            small: true,
                            disabled,
                            ..Default::default()
                        },
                    );
                    if disabled {
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
                        }
                        let _ = response.on_hover_text("coming soon");
                    } else if response.clicked() {
                        clicked = true;
                    }

                    ui.add_space(10.0);

                    let (pill_text, pill_fill) = match &state {
                        CardState::Connected { .. } => {
                            ("connected", redesign_pill_info(palette))
                        }
                        CardState::NotConnected => {
                            ("not connected", redesign_pill_neutral(palette))
                        }
                    };
                    draw_pill(ui, palette, pill_text, pill_fill);
                },
            );
        });
    });
    ui.add_space(8.0);
    clicked
}

/// Small filled chip — no border per wireframe `screens.jsx::Pill` (line
/// 745). 10px font, 7px corner radius, 1px×6px padding, dark text on the
/// tone-matched fill.
fn draw_pill(ui: &mut egui::Ui, palette: ThemePalette, label: &str, fill: egui::Color32) {
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), redesign_pill_text(palette));
    let pad = egui::vec2(6.0, 1.5);
    let size = egui::vec2(galley.size().x + pad.x * 2.0, galley.size().y + pad.y * 2.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(7);
    painter.rect_filled(rect, radius, fill);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font,
        redesign_pill_text(palette),
    );
}
