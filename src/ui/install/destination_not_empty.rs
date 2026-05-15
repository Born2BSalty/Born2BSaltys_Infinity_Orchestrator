// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `destination_not_empty` — the yellow-bordered warning Box shown in the
// Install Modlist paste stage when the chosen destination folder already has
// content (SPEC §4.1 / §13.12 #6).
//
// **Verbatim from `wireframe-preview/screens.jsx::DestinationNotEmptyWarning`
// (line 123-154).** Reproduced faithfully:
//   - container: `border:1.5px solid #edc547; borderRadius:3px;
//     background:rgba(237,197,71,0.18); boxShadow:2px 2px 0 var(--shadow);
//     padding:10px 14px; marginTop:12`
//   - header row: `⚠` (fontSize:13) + Label weight 500 / 13px
//     "Target directory not empty"
//   - sub: Label hand / 14px / text-muted "How would you like to proceed?"
//   - options row: `flex gap:8 wrap` of `Btn small primary={choice===id}`
//     — the wireframe renders the choices as toggle-style `Btn`s (the one
//     matching the current choice is `primary`), NOT native radio widgets.
//     Per the fidelity rule (wireframe wins over SPEC prose on UI), these are
//     `redesign_btn`s, not egui radio buttons, even though SPEC §4.1 calls
//     them "radio choices".
//
// Option set (wireframe-verbatim labels, `screens.jsx:124-128`):
//   - `clear`    → "Clear contents"
//   - `backup`   → "Backup contents then proceed"
//   - `continue` → "Continue partial installation"  (only when
//     `allow_partial == true`)
//
// The `⚠` U+26A0 glyph is rendered in `firacode_nerd`, NOT Poppins: the
// shipped Poppins TTFs are a Latin-only subset and would tofu it (HANDOFF
// "Non-Latin symbol glyphs" caveat). The amber `#edc547` border + the
// `rgba(237,197,71,0.18)` fill are wireframe-literal hex values local to this
// warning (they are not in the redesign palette table — the wireframe hard-
// codes them here, so we mirror them as local constants per the fidelity
// rule).
//
// SPEC: §4.1, §13.12 #6. Wireframe: screens.jsx:123-154 (verbatim).

use eframe::egui;

use crate::ui::install::state_install::DestChoice;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_shadow, redesign_text_muted, redesign_text_primary,
};

/// Wireframe `border: 1.5px solid #edc547` — amber warning border (literal
/// hex, not a palette token: the wireframe hard-codes it inside this
/// component).
const WARN_BORDER: egui::Color32 = egui::Color32::from_rgb(0xed, 0xc5, 0x47);
/// Wireframe `background: rgba(237, 197, 71, 0.18)` — same amber at 18%.
/// Premultiplied: alpha = round(0.18·255) = 46; each channel = round(c·0.18)
/// → R 237·0.18≈43 (0x2B), G 197·0.18≈35 (0x23), B 71·0.18≈13 (0x0D).
const WARN_FILL: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(0x2B, 0x23, 0x0D, 46);

/// Render the warning Box. `choice` is the currently selected option (if
/// any); returns `Some(new_choice)` when the user clicks one this frame.
/// `allow_partial` gates the third (`Continue partial installation`) option
/// per the wireframe's `allowPartial` prop (default `true`).
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    choice: Option<DestChoice>,
    allow_partial: bool,
) -> Option<DestChoice> {
    // Wireframe `marginTop: 12`.
    ui.add_space(12.0);

    let mut picked: Option<DestChoice> = None;

    let frame = egui::Frame::default()
        .fill(WARN_FILL)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, WARN_BORDER))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        // Wireframe `padding: 10px 14px`.
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .shadow(egui::epaint::Shadow {
            // Wireframe `boxShadow: 2px 2px 0 var(--shadow)`.
            offset: [
                REDESIGN_SHADOW_OFFSET_BTN_PX as i8,
                REDESIGN_SHADOW_OFFSET_BTN_PX as i8,
            ],
            blur: 0,
            spread: 0,
            color: redesign_shadow(palette),
        });

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());

        // ── Header row: ⚠ + "Target directory not empty" (gap 10). ──
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            // ⚠ in FiraCode Nerd (Poppins is Latin-subset — HANDOFF caveat).
            ui.label(
                egui::RichText::new("\u{26A0}")
                    .size(13.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.label(
                egui::RichText::new("Target directory not empty")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
        });

        // Wireframe header `marginBottom: 4`.
        ui.add_space(4.0);

        // ── Sub (hand-style, muted): "How would you like to proceed?" ──
        ui.label(
            egui::RichText::new("How would you like to proceed?")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_muted(palette)),
        );

        // Wireframe sub `marginBottom: 10`.
        ui.add_space(10.0);

        // ── Options row: toggle-style Btns, flex gap:8 wrap. ──
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            for (opt, label) in option_set(allow_partial) {
                let is_active = choice == Some(opt);
                if redesign_btn(
                    ui,
                    palette,
                    label,
                    BtnOpts {
                        small: true,
                        primary: is_active,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    picked = Some(opt);
                }
            }
        });
    });

    picked
}

/// The wireframe-verbatim option set (`screens.jsx:124-128`). `Continue
/// partial installation` is appended only when `allow_partial` is `true`
/// (wireframe `...(allowPartial ? [...] : [])`).
fn option_set(allow_partial: bool) -> Vec<(DestChoice, &'static str)> {
    let mut opts = vec![
        (DestChoice::Clear, "Clear contents"),
        (DestChoice::Backup, "Backup contents then proceed"),
    ];
    if allow_partial {
        opts.push((DestChoice::Continue, "Continue partial installation"));
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_labels_are_wireframe_verbatim() {
        // screens.jsx:124-128 — exact label strings + ids.
        let opts = option_set(true);
        assert_eq!(
            opts,
            vec![
                (DestChoice::Clear, "Clear contents"),
                (DestChoice::Backup, "Backup contents then proceed"),
                (DestChoice::Continue, "Continue partial installation"),
            ]
        );
    }

    #[test]
    fn continue_option_hidden_when_partial_disallowed() {
        let opts = option_set(false);
        assert_eq!(
            opts,
            vec![
                (DestChoice::Clear, "Clear contents"),
                (DestChoice::Backup, "Backup contents then proceed"),
            ]
        );
        assert!(!opts.iter().any(|(c, _)| *c == DestChoice::Continue));
    }
}
