// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `toast` — the bottom-center transient notification (P5.T16, SPEC §10.8 /
// §3.1).
//
// Mirrors `wireframe-preview/screens.jsx::HomeScreen` toast (line 367-382):
//   position: fixed; bottom: 38; left: 50%; transform: translateX(-50%);
//   sketchyBorder; background: var(--shell-bg); boxShadow: 3px 3px 0
//   var(--shadow); padding: 8px 16px; fontSize: 13; color: var(--success);
//   content: `✓ {toast}`
//
// egui implementation: an `egui::Area` anchored bottom-center with
// `Order::Tooltip` so it floats above the destination content (the wireframe
// uses `zIndex: 200`). It is **non-interactive** (no dismiss button — SPEC
// §10.8) and auto-dismisses ~1.8s after `shown_at` (the caller clears
// `HomeScreenState.toast` once `is_expired` returns true).
//
// The leading `✓` marker is rendered in `firacode_nerd`, NOT Poppins: the
// shipped Poppins TTFs are a Latin-only subset that lacks U+2713 (✓), so a
// Poppins `✓` tofus to `?` (HANDOFF "Non-Latin symbol glyphs" caveat — this
// bit the project twice). The body text stays Poppins. Error toasts (SPEC
// §3.2 "Open install folder" failure surface) render in the danger tone with
// no marker.
//
// SPEC: §10.8 (Toast), §3.1 (bottom-center, ~1.8s auto-dismiss), §3.2.

use std::time::Duration;

use eframe::egui;

use crate::ui::home::state_home::{ToastMessage, ToastTone};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_border_strong, redesign_pill_danger, redesign_shadow,
    redesign_shell_bg, redesign_success,
};

/// Auto-dismiss window. SPEC §3.1 / §10.8: "auto-dismisses ~1.8s"; wireframe
/// `setTimeout(..., 1800)` (`screens.jsx:237/244`).
pub const TOAST_TTL: Duration = Duration::from_millis(1800);

/// Distance from the bottom edge to the toast — wireframe `bottom: 38`.
const BOTTOM_OFFSET_PX: f32 = 38.0;

impl ToastMessage {
    /// True once the toast has been visible longer than `TOAST_TTL`. The
    /// caller clears `HomeScreenState.toast` when this returns true.
    pub fn is_expired(&self) -> bool {
        self.shown_at.elapsed() >= TOAST_TTL
    }
}

/// Render the toast (if any) bottom-center. Returns `true` while a toast is
/// still live this frame so the caller can `ctx.request_repaint_after(...)`
/// to drive the auto-dismiss without waiting for a user event (egui paints
/// lazily).
pub fn render(ctx: &egui::Context, palette: ThemePalette, toast: Option<&ToastMessage>) -> bool {
    let Some(toast) = toast else {
        return false;
    };
    if toast.is_expired() {
        // Don't paint a stale toast on the frame the caller is about to
        // clear it — but still report "live" so the caller's repaint loop
        // ticks one more time to actually drop it.
        return true;
    }

    let (marker, body_color): (Option<&str>, egui::Color32) = match toast.tone {
        ToastTone::Success => (Some("\u{2713}"), redesign_success(palette)), // ✓
        ToastTone::Error => (None, redesign_pill_danger(palette)),
    };

    egui::Area::new(egui::Id::new("orchestrator_home_toast"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -BOTTOM_OFFSET_PX))
        .interactable(false)
        .show(ctx, |ui| {
            let chassis = egui::Frame::default()
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
                .inner_margin(egui::Margin {
                    left: 16,
                    right: 16,
                    top: 8,
                    bottom: 8,
                })
                .shadow(egui::epaint::Shadow {
                    offset: [
                        REDESIGN_SHADOW_OFFSET_BTN_PX as i8 + 1,
                        REDESIGN_SHADOW_OFFSET_BTN_PX as i8 + 1,
                    ],
                    blur: 0,
                    spread: 0,
                    color: redesign_shadow(palette),
                });

            chassis.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if let Some(marker) = marker {
                        // ✓ in FiraCode Nerd (Poppins is Latin-subset and
                        // would tofu it — HANDOFF caveat).
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
            shown_at: Instant::now() - (TOAST_TTL + Duration::from_millis(50)),
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
