// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Duration;

use eframe::egui;

use crate::ui::home::state_home::{ToastMessage, ToastTone};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_pill_danger, redesign_shell_bg, redesign_success,
};
use crate::ui::shared::redesign_visuals::redesign_overlay_shadow;

pub const TOAST_TTL: Duration = Duration::from_millis(1800);

const BOTTOM_OFFSET_PX: f32 = 38.0;

impl ToastMessage {
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.shown_at.elapsed() >= TOAST_TTL
    }
}

#[must_use]
pub fn render(ctx: &egui::Context, palette: ThemePalette, toast: Option<&ToastMessage>) -> bool {
    let Some(toast) = toast else {
        return false;
    };
    if toast.is_expired() {
        return true;
    }

    let (marker, body_color): (Option<&str>, egui::Color32) = match toast.tone {
        ToastTone::Success => (Some("\u{2713}"), redesign_success(palette)),
        ToastTone::Error => (None, redesign_pill_danger(palette)),
    };

    egui::Area::new(egui::Id::new("orchestrator_home_toast"))
        .order(egui::Order::Tooltip)
        .anchor(
            egui::Align2::CENTER_BOTTOM,
            egui::vec2(0.0, -BOTTOM_OFFSET_PX),
        )
        .interactable(false)
        .show(ctx, |ui| {
            let chassis = egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
                .shadow(redesign_overlay_shadow(palette))
                .inner_margin(egui::Margin {
                    left: 16,
                    right: 16,
                    top: 8,
                    bottom: 8,
                });

            chassis.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if let Some(marker) = marker {
                        ui.label(
                            egui::RichText::new(marker)
                                .size(13.0)
                                .family(egui::FontFamily::Name("firacode_nerd".into()))
                                .color(body_color),
                        );
                    }
                    ui.label(
                        egui::RichText::new(&toast.text)
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_medium".into()))
                            .color(body_color),
                    );
                });
            });
        });

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn fresh_toast_is_not_expired() {
        let t = ToastMessage::success("Copied import code");
        assert!(!t.is_expired());
    }

    #[test]
    fn old_toast_is_expired() {
        let t = ToastMessage {
            text: "stale".to_string(),
            shown_at: Instant::now()
                .checked_sub(TOAST_TTL + Duration::from_millis(50))
                .unwrap(),
            tone: ToastTone::Success,
        };
        assert!(t.is_expired());
    }

    #[test]
    fn constructors_set_tone() {
        assert_eq!(ToastMessage::success("x").tone, ToastTone::Success);
        assert_eq!(ToastMessage::error("y").tone, ToastTone::Error);
    }
}
