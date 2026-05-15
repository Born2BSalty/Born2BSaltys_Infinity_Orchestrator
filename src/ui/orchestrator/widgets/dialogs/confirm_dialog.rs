// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ConfirmDialog` — the shared generic confirm/cancel popup (SPEC §10.7,
// rendered per the §10.1 non-blocking rule). Used by Home Delete (P5.T7) and
// Home Reinstall (P5.T18); later phases reuse it (e.g. Step 2 select-via-
// WeiDU-log).
//
// Mirrors `wireframe-preview/screens.jsx::ConfirmDialog` (line 1721-1771):
//   sketchyBorder; background var(--shell-bg); boxShadow 5px 5px 0
//   var(--shadow); padding 18; maxWidth 460; width 92%
//   header row: <Label fontSize:15 fontWeight:500>{title}</Label>
//   body:  <Label color:var(--text-muted) fontSize:13 lineHeight:1.45>
//   footer: <Btn small>Cancel</Btn>  <Btn small primary
//            style={danger ? {background:#e69a96} : undefined}>{confirmLabel}</Btn>
//   (footer is flex-end, gap 8)
//
// **Non-blocking** per SPEC §10.1: "All dialogs listed here are non-blocking
// `egui::Window` popups … they do not block interaction with the rest of the
// app behind them. … No backdrop, no focus trap." The wireframe's
// `rgba(0,0,0,0.45)` full-screen overlay is a wireframe-rendering convention
// only — we follow BIO's non-modal `egui::Window` pattern instead.
//
// The redesign collapse-chevron pattern (SPEC §10.1) is a Phase 8 concern
// (carve-out #2 flips BIO popups to `.collapsible(true)`); this net-new
// dialog simply uses egui's native title-less window and is left
// non-collapsible for Phase 5 — consistent with the other net-new redesign
// surfaces shipped so far.
//
// Danger styling: the confirm button is the primary `redesign_btn` recolored
// to the danger tone (`redesign_pill_danger` == wireframe `#e69a96`).
//
// SPEC: §10.1 (non-blocking), §10.7 (ConfirmDialog), §3.1.

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_PX, ThemePalette,
    redesign_border_strong, redesign_pill_danger, redesign_shadow, redesign_shell_bg,
    redesign_text_muted, redesign_text_primary,
};

/// What the user did on the dialog this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmOutcome {
    /// Still open, nothing clicked.
    #[default]
    Pending,
    /// Primary `Confirm` clicked.
    Confirmed,
    /// `Cancel` clicked.
    Cancelled,
}

/// Static description of a confirm dialog. `body` is the multi-line message
/// (already assembled by the caller — e.g. the Delete confirm's
/// wireframe-verbatim text incl. the destination path).
pub struct ConfirmDialog<'a> {
    /// Unique window id-salt so multiple dialogs don't collide.
    pub id_salt: &'a str,
    /// Header line (wireframe 15px / weight 500).
    pub title: &'a str,
    /// Body message (muted, 13px, ~1.45 line height).
    pub body: &'a str,
    /// Primary button label (e.g. `Delete`, `Reinstall`).
    pub confirm_label: &'a str,
    /// Coral-tint the primary button (destructive actions).
    pub danger: bool,
}

/// `maxWidth: 460` (wireframe). egui windows shrink-wrap, so this caps the
/// text-wrap width.
const MAX_WIDTH_PX: f32 = 460.0;

/// Render the dialog as a centered, non-blocking `egui::Window`. Returns the
/// outcome this frame; the caller owns the open/closed state (clears its
/// `*_target` on `Confirmed` / `Cancelled`).
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    dialog: &ConfirmDialog<'_>,
) -> ConfirmOutcome {
    let mut outcome = ConfirmOutcome::Pending;

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(18))
        .shadow(egui::epaint::Shadow {
            // Wireframe `boxShadow: 5px 5px 0 var(--shadow)`.
            offset: [
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
            ],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        });

    egui::Window::new(dialog.title)
        .id(egui::Id::new(("orchestrator_confirm_dialog", dialog.id_salt)))
        // Non-blocking per SPEC §10.1 — no modal area / backdrop / focus trap.
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            // ── Header (15px / weight 500). ──
            ui.label(
                egui::RichText::new(dialog.title)
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(8.0);

            // ── Body (muted, 13px, wrapped). ──
            ui.label(
                egui::RichText::new(dialog.body)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.add_space(16.0);

            // ── Footer: Cancel + (danger) primary Confirm, flush-right. ──
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // `right_to_left` lays trailing-edge first → push the primary
                // first so the on-screen order reads [Cancel] [Confirm].
                let confirm = if dialog.danger {
                    danger_primary_btn(ui, palette, dialog.confirm_label)
                } else {
                    redesign_btn(
                        ui,
                        palette,
                        dialog.confirm_label,
                        BtnOpts {
                            small: true,
                            primary: true,
                            ..Default::default()
                        },
                    )
                };
                if confirm.clicked() {
                    outcome = ConfirmOutcome::Confirmed;
                }

                if redesign_btn(
                    ui,
                    palette,
                    "Cancel",
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    outcome = ConfirmOutcome::Cancelled;
                }
            });
        });

    outcome
}

/// The danger-tinted primary button. The wireframe overrides only the primary
/// button's `background` to `#e69a96` (`redesign_pill_danger`); everything
/// else (sketchy border, 2×2 shadow, fixed dark `#1a2638` label, small
/// padding) matches the normal primary `redesign_btn`. We reproduce the
/// primary chassis with the fill swapped so the override stays faithful
/// without forking `redesign_btn`'s options.
fn danger_primary_btn(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let fill = redesign_pill_danger(palette);
    // Wireframe primary text is the theme-invariant `#1a2638` (same as
    // `redesign_btn`'s primary branch) — high contrast on the coral fill.
    let text_color = egui::Color32::from_rgb(0x1a, 0x26, 0x38);

    let font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    // Match `redesign_btn`'s active-press transform.
    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        // 2×2 drop shadow (primary).
        let shadow_rect = rect.translate(egui::vec2(2.0, 2.0));
        painter.rect_filled(shadow_rect, radius, redesign_shadow(palette));
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
            text_color,
        );
    }

    response
}
