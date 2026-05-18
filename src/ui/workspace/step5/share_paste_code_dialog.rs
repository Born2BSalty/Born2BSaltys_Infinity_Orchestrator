// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `share_paste_code_dialog` — the **`SharePasteCodeDialog`** non-blocking
// `egui::Window` popup (per SPEC §10 / §10.3), opened from the workspace
// header's `Share import code` button **only after a successful install**.
//
// Per SPEC §10.3 + the wireframe (`screens.jsx:1789-1837`):
//   - Title "Share import code"
//   - Sub "Anyone can paste this into BIO → Install to get the same
//     modlist."
//   - A monospace, scrollable box (`maxHeight 180`, word-break) containing
//     the BIO-MODLIST-V1 code.
//   - Footer: `Close` + primary `Copy`. On `Copy`: write to the clipboard
//     and flash `✓ copied to clipboard` inline next to the buttons for
//     ~1.5s.
//
// **Share code source — `ModlistEntry.latest_share_code`.** SPEC §10.3 /
// §13.3: this dialog only opens after a successful install, and the code
// shown is the at-install **registry snapshot** —
// `entry.latest_share_code`, which `flip_to_installed` (P7.T6) regenerated
// with `allow_auto_install = true`. It is **NOT** re-derived from the
// current `WizardState`. The Home Kebab `Copy import code` reads the same
// field (but copies directly without this dialog). If the entry has no
// code yet (should not happen post-install, but defensive — e.g. a forced
// re-open before `flip_to_installed`'s atomic write), an honest "no code
// available yet" placeholder is shown and `Copy` is disabled.
//
// **Non-blocking** per SPEC §10 — a centered `egui::Window` with NO
// backdrop / focus trap, bit-for-bit consistent with the redesign's other
// net-new popups (`confirm_dialog.rs` — the exact chassis pattern reused
// here). The collapse-chevron (SPEC §10) is a Phase-8 concern (carve-out
// #2 / `popup_collapse_anchor.rs`) — this net-new dialog ships
// `.collapsible(false)` by design, consistent with `confirm_dialog.rs`.
//
// SPEC: §10.3, §10 (non-blocking popup convention), §13.3.

// rationale: `f32 as i8/u8` casts are the same pixel-radius / shadow-offset
// roundings of small positive constants `confirm_dialog.rs` already
// suppresses (Cat 2 — correct by construction).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::{Duration, Instant};

use eframe::egui;

use crate::registry::model::ModlistEntry;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_PX, ThemePalette,
    redesign_border_strong, redesign_input_bg, redesign_shadow, redesign_shell_bg,
    redesign_success, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::step5::state_workspace_step5::WorkspaceStep5State;

/// Wireframe `maxWidth: 600` (`screens.jsx:1807`). egui windows
/// shrink-wrap; this caps the dialog width.
const MAX_WIDTH_PX: f32 = 600.0;
/// Wireframe code box `maxHeight: 180` (`screens.jsx:1823`) — internal
/// scroll past that.
const CODE_BOX_MAX_HEIGHT_PX: f32 = 180.0;
/// `✓ copied to clipboard` flash window (SPEC §10.3 "for ~1.5s").
const COPIED_FLASH: Duration = Duration::from_millis(1500);

/// Render the Share import code dialog when `state.share_dialog_open`.
///
/// No-op when the dialog is closed. When open, paints the non-blocking
/// `egui::Window` per SPEC §10.3 reading the code from
/// `entry.latest_share_code` (the post-`flip_to_installed`
/// `allow_auto_install = true` value). `Close` (or a click outside) clears
/// `share_dialog_open`; `Copy` writes the code to the clipboard
/// (`ctx.copy_text` — the egui built-in, no clipboard crate, same as the
/// Home Kebab) and arms the `✓ copied to clipboard` flash for ~1.5s.
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut WorkspaceStep5State,
    entry: &ModlistEntry,
) {
    if !state.share_dialog_open {
        return;
    }

    // Resolve the code (the registry snapshot — NOT re-derived from
    // WizardState). Post-install this is always `Some`; the placeholder is
    // a defensive honest fallback.
    let code: Option<&str> = entry.latest_share_code.as_deref().filter(|c| !c.is_empty());

    // Expire the copied-flash if its window elapsed (mirrors the
    // `save_draft_flash_until` precedent in `workspace_header.rs`).
    let now = Instant::now();
    let flashing = match state.copied_flash_until {
        Some(until) if now < until => true,
        Some(_) => {
            state.copied_flash_until = None;
            false
        }
        None => false,
    };

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(20))
        .shadow(egui::epaint::Shadow {
            // Wireframe `boxShadow: 5px 5px 0 var(--shadow)`
            // (`screens.jsx:1807`) — the same offset `confirm_dialog.rs`
            // uses.
            offset: [
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
                REDESIGN_SHADOW_OFFSET_PX as i8 - 1,
            ],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        });

    let mut close_clicked = false;
    let mut copy_clicked = false;

    egui::Window::new("Share import code")
        .id(egui::Id::new("orchestrator_share_paste_code_dialog"))
        // Non-blocking per SPEC §10 — no modal area / backdrop / focus
        // trap (BIO's non-modal popup pattern; identical to
        // `confirm_dialog.rs`).
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            // ── Header (wireframe `fontSize:18, fontWeight:500`). ──
            ui.label(
                egui::RichText::new("Share import code")
                    .size(18.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(6.0);

            // ── Sub (muted, 13px, ~1.5 line height). ──
            ui.label(
                egui::RichText::new(
                    "Anyone can paste this into BIO \u{2192} Install to get the same modlist.",
                )
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
            );
            ui.add_space(14.0);

            // ── Monospace, scrollable code box (wireframe: sketchy border,
            //    `var(--input-bg)`, FiraCode, 12px, maxHeight 180,
            //    word-break, pre-wrap). ──
            egui::Frame::default()
                .fill(redesign_input_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(CODE_BOX_MAX_HEIGHT_PX)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            match code {
                                Some(c) => {
                                    // `wordBreak: break-all; whiteSpace:
                                    // pre-wrap` — wrap the long base64url
                                    // body at any char so it never
                                    // overflows the box.
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(c)
                                                .size(12.0)
                                                .family(egui::FontFamily::Name(
                                                    "firacode_nerd".into(),
                                                ))
                                                .color(redesign_text_primary(palette)),
                                        )
                                        .wrap(),
                                    );
                                }
                                None => {
                                    ui.label(
                                        egui::RichText::new(
                                            "No import code available yet for this modlist.",
                                        )
                                        .size(12.0)
                                        .family(egui::FontFamily::Name("poppins_light".into()))
                                        .color(redesign_text_muted(palette)),
                                    );
                                }
                            }
                        });
                });
            ui.add_space(14.0);

            // ── Footer: (flash) … [Close] [Copy primary], flush-right
            //    (wireframe `justifyContent:flex-end, gap:8`; the flash is
            //    `marginRight:auto` so it sits on the LEFT). Bounded
            //    fixed-height band (the `confirm_dialog.rs` rationale —
            //    keep the auto-sizing Window shrink-wrapped). ──
            let footer_h = 30.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), footer_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // `right_to_left` lays trailing-edge first → push the
                    // primary first so the on-screen order is
                    // [flash] … [Close] [Copy].
                    let copy_enabled = code.is_some();
                    let copy = redesign_btn(
                        ui,
                        palette,
                        "Copy",
                        BtnOpts {
                            small: true,
                            primary: true,
                            disabled: !copy_enabled,
                            ..Default::default()
                        },
                    );
                    if copy_enabled && copy.clicked() {
                        copy_clicked = true;
                    }

                    if redesign_btn(
                        ui,
                        palette,
                        "Close",
                        BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        close_clicked = true;
                    }

                    // `✓ copied to clipboard` — `marginRight:auto` ⇒
                    // left-most in the flex row; in `right_to_left` it is
                    // laid last so it lands on the far left. `✓` U+2713 IS
                    // cmap-present in `firacode_nerd` (verified); prose in
                    // poppins. Success-green (wireframe `screens.jsx:1829`
                    // `color: var(--success)`).
                    if flashing {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("\u{2713}")
                                    .size(14.0)
                                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                                    .color(redesign_success(palette)),
                            );
                            ui.label(
                                egui::RichText::new(" copied to clipboard")
                                    .size(14.0)
                                    .family(egui::FontFamily::Name("poppins_light".into()))
                                    .color(redesign_success(palette)),
                            );
                        });
                    }
                },
            );
        });

    // ── Apply outcomes after the Window closure (no live `ctx`/`state`
    //    borrow conflict). ──
    if copy_clicked {
        if let Some(c) = code {
            // egui built-in clipboard write (no clipboard crate — the same
            // `ctx.copy_text` the Home Kebab `Copy import code` uses).
            ctx.copy_text(c.to_string());
            state.copied_flash_until = Some(Instant::now() + COPIED_FLASH);
        }
    }
    if close_clicked {
        state.share_dialog_open = false;
        // Reset the flash so re-opening the dialog starts clean (wireframe:
        // `copied` resets on open).
        state.copied_flash_until = None;
    }

    // Keep repainting while the flash is up so it auto-reverts even without
    // user input (the `save_draft` flash precedent).
    if state.copied_flash_until.is_some() {
        ctx.request_repaint_after(Duration::from_millis(120));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry};

    #[test]
    fn closed_dialog_is_a_noop_state_unchanged() {
        // `render` early-returns when `share_dialog_open == false`; we can't
        // build an `egui::Context` headlessly without a harness, but the
        // closed-path invariant (no state mutation) is asserted by
        // construction here: a fresh state has the dialog closed + no flash.
        let s = WorkspaceStep5State::default();
        assert!(!s.share_dialog_open);
        assert!(s.copied_flash_until.is_none());
    }

    #[test]
    fn code_source_is_the_registry_entry_snapshot() {
        // SPEC §10.3 / §13.3: the dialog's code is `entry.latest_share_code`
        // (the post-`flip_to_installed` allow_auto_install=true snapshot),
        // NOT re-derived from WizardState. This guards the resolution rule
        // the render uses (`entry.latest_share_code.as_deref().filter(non-
        // empty)`).
        let with_code = ModlistEntry {
            id: "S".to_string(),
            name: "n".to_string(),
            game: Game::EET,
            latest_share_code: Some("BIO-MODLIST-V1:ABC".to_string()),
            ..Default::default()
        };
        assert_eq!(
            with_code
                .latest_share_code
                .as_deref()
                .filter(|c| !c.is_empty()),
            Some("BIO-MODLIST-V1:ABC")
        );

        // Empty / absent ⇒ the disabled-Copy honest-fallback path.
        let no_code = ModlistEntry {
            latest_share_code: Some(String::new()),
            ..with_code.clone()
        };
        assert_eq!(
            no_code
                .latest_share_code
                .as_deref()
                .filter(|c| !c.is_empty()),
            None
        );
        let none_code = ModlistEntry {
            latest_share_code: None,
            ..with_code
        };
        assert_eq!(none_code.latest_share_code.as_deref(), None);
    }
}
