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
// The wireframe's `⚠` (U+26A0, "Miscellaneous Symbols" block) is painted as
// a VECTOR triangle, not a font glyph: neither the Latin-subset Poppins nor
// the shipped FiraCode Nerd covers U+2600–26FF (base FiraCode has the
// math/arrow/✓ ranges but not Misc Symbols; the Nerd patch adds PUA icons,
// not that backfill), so a glyph here tofus to `?`. Vectors decouple it from
// font coverage — the same approach `left_rail.rs` uses for the nav icons.
// See `paint_warning_triangle`. The amber `#edc547` border + the
// `rgba(237,197,71,0.18)` fill are wireframe-literal hex values local to this
// warning (they are not in the redesign palette table — the wireframe hard-
// codes them here, so we mirror them as local constants per the fidelity
// rule).
//
// SPEC: §4.1, §13.12 #6. Wireframe: screens.jsx:123-154 (verbatim).

// rationale: `f32 as u8`/`i8` casts are pixel-radius / shadow-offset
// roundings of small positive constants — correct by construction (Cat 2);
// the doc-paragraph-length lint is subjective style (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph
)]

use eframe::egui;

use crate::ui::install::state_install::DestChoice;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_shadow,
};

/// Wireframe `border: 1.5px solid #edc547` — amber warning border (literal
/// hex, not a palette token: the wireframe hard-codes it inside this
/// component).
const WARN_BORDER: egui::Color32 = egui::Color32::from_rgb(0xed, 0xc5, 0x47);
/// Ink for text on this warning surface — **white, theme-invariant**. The
/// amber fill composites over the app's dark-ish backdrop to a fairly **dark
/// olive in BOTH themes**, so the surface is *dark*, not a light pill tone:
/// §12.2's "dark text on toned surfaces" rule is for the *light* pill tones
/// (coral / amber / teal / grey); inverted here because this composited
/// surface is dark, so it takes light/white ink. Deliberate, recorded
/// deviation from the wireframe's `var(--text-muted)` (and from the earlier
/// dark-ink attempt, which looked muddy on the olive — QA 2026-05-16). Header
/// solid white; the secondary "how would you like to proceed?" uses white at
/// reduced alpha — secondary but crisp on the dark olive.
const WARN_INK: egui::Color32 = egui::Color32::from_rgb(0xff, 0xff, 0xff);
/// Wireframe `background: rgba(237, 197, 71, 0.18)` — amber at ~18% alpha.
/// **Un-premultiplied** so egui composites it source-over the *current theme
/// background* (pale amber wash on Light, subtle amber glow on Dark). The
/// earlier premultiplied form baked a dark fill that rendered as an opaque
/// dark box on the Light parchment (QA: "not clear in light mode"). A `fn`
/// (not a `const`) because `from_rgba_unmultiplied` is not `const fn` in this
/// egui version. alpha = round(0.18·255) = 46.
fn warn_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(0xED, 0xC5, 0x47, 46)
}

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
        .fill(warn_fill())
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
            // The warning mark is a PAINTED VECTOR, not a glyph. `⚠` U+26A0
            // is in the "Miscellaneous Symbols" block, which neither the
            // Latin-subset Poppins NOR the shipped FiraCode Nerd covers
            // (base FiraCode has math/arrows/✓ but not U+2600–26FF; the Nerd
            // patch adds PUA icons, not that backfill) — a glyph here tofus
            // to `?`. Vectors decouple it from font coverage entirely, the
            // same approach `left_rail.rs` uses for the nav icons.
            let (icon_rect, _) =
                ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
            paint_warning_triangle(ui.painter(), icon_rect.center(), WARN_INK);
            ui.label(
                egui::RichText::new("Target directory not empty")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(WARN_INK),
            );
        });

        // Wireframe header `marginBottom: 4`.
        ui.add_space(4.0);

        // ── Sub (hand-style, muted): "How would you like to proceed?" ──
        ui.label(
            egui::RichText::new("How would you like to proceed?")
                .size(14.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                // White at ~80% over the dark olive surface — secondary vs.
                // the solid-white header, but crisp in both themes (neither
                // the grey `text-muted` token nor dark ink reads on it).
                .color(egui::Color32::from_rgba_unmultiplied(
                    0xff, 0xff, 0xff, 0xCC,
                )),
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

/// Paint the warning mark as a vector triangle + `!` (see the module header
/// for why this is not a font glyph). Sized to ~13px ink centered on
/// `center`; stroke + color match the header text so it reads as one unit.
fn paint_warning_triangle(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.6, color);
    let hw = 6.5; // half-width of the triangle base
    let top = center.y - 6.0; // apex
    let base_y = center.y + 5.0; // base line

    // Triangle outline (closed): apex → base-right → base-left.
    painter.add(egui::Shape::closed_line(
        vec![
            egui::pos2(center.x, top),
            egui::pos2(center.x + hw, base_y),
            egui::pos2(center.x - hw, base_y),
        ],
        stroke,
    ));

    // Exclamation: short vertical stem + a dot near the base.
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - 2.0),
            egui::pos2(center.x, center.y + 1.5),
        ],
        stroke,
    );
    painter.circle_filled(egui::pos2(center.x, center.y + 3.6), 0.95, color);
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
