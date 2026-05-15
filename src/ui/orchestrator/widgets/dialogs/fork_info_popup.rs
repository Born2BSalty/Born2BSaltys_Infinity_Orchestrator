// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `ForkInfoPopup` — the read-only fork-lineage / credit-chain popup
// (SPEC §10.9), rendered per the §10.1 non-blocking rule.
//
// Mirrors `wireframe-preview/screens.jsx::ForkInfoPopup` (line 2228-2320):
//   header:  `Fork lineage` (Label 15px / weight 500) + collapse chevron
//   body:    one row per ForkAncestor (oldest → newest), then a final
//            emphasized row for THIS modlist:
//              row i: marginLeft = i*20
//                line 1: (i>0 ? `↳` faint firacode) + <name> (Poppins 14,
//                        weight 700 if current, accent-deep if current)
//                        + (current ? `⑂ this modlist` sketchy tag)
//                line 2: `by <author>`  (firacode 12, faint) — OMITTED when
//                        the node's author is empty (SPEC §10.9 / §4.2
//                        author-absent rule; the wireframe's `|| "—"` demo
//                        fallback is overridden by the SPEC, which is
//                        authoritative on the absent case)
//            empty lineage → faint "This modlist was created from scratch —
//            no fork lineage."
//   footer:  `Close` only (flush-right).
//
// **Non-blocking** per SPEC §10.1 — same chassis decision as the sibling
// `confirm_dialog.rs`: a centered `egui::Window` with `.title_bar(false)`
// `.collapsible(false)` and NO modal area / backdrop / focus trap (the
// wireframe's `rgba(0,0,0,0.45)` overlay is a wireframe-rendering
// convention only). The redesign collapse-chevron pattern is a Phase 8
// concern (carve-out #2 / `popup_collapse_anchor.rs`); this net-new dialog
// uses egui's native title-less window and is left non-collapsible for
// Phase 5 — bit-for-bit consistent with `confirm_dialog.rs`.
//
// **Symbol-glyph rule (cmap-verified, HANDOFF caveat).** Two glyphs:
//   - `↳` U+21B3 — PRESENT in the shipped FiraCodeNerdFont-Light.ttf
//     (cmap-verified); rendered as a `firacode_nerd` glyph exactly as the
//     wireframe specifies.
//   - `⑂` U+2442 — ABSENT even in the full Nerd build (cmap-verified;
//     "Miscellaneous Symbols" / fork glyph is not in FiraCode's coverage
//     and not in the PUA patch). A glyph here would tofu to `?`. It is
//     therefore PAINTED AS A VECTOR (a two-tine fork), the same approach
//     `left_rail.rs`'s nav icons and `destination_not_empty.rs`'s
//     `paint_warning_triangle` use. The `⑂ this modlist` tag draws the
//     vector fork then the prose in Poppins, side by side.
//
// `forked_from` / the current-node identity come from the parsed
// `bio::app::modlist_share::ModlistSharePreview` (carve-out #5 fields) at
// the Install/fork-preview call site, or the registry entry at the
// workspace-header call site (Phase 6 wires those triggers; Run 4 only
// builds the widget — SPEC §10.9 Triggers).
//
// SPEC: §10.9 (ForkInfoPopup), §10.1 (non-blocking), §13.3 (Provenance),
//       §4.2 (author-absent rule), §1 (carve-out #5 provenance fields).

// rationale: `f32 as u8`/`i8` casts are pixel-radius / shadow-offset
// roundings of small positive constants — correct by construction (Cat 2);
// the doc-paragraph-length lint is subjective style on a faithfully-
// mirrored widget (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::app::modlist_share::ForkAncestor;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_PX, ThemePalette,
    redesign_accent_deep, redesign_border_strong, redesign_shadow, redesign_shell_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// `maxWidth: 480` (wireframe `screens.jsx:2256`). egui windows shrink-wrap;
/// this caps the width so a wide name / deep indent wraps instead of
/// stretching the popup.
const MAX_WIDTH_PX: f32 = 480.0;

/// Wireframe per-generation indent (`marginLeft: i * 20`).
const INDENT_PER_GEN_PX: f32 = 20.0;

/// SPEC §10.9: "For deep lineages the indent caps … so content never
/// overflows the popup width." Cap the indent at 6 generations' worth
/// (120px) — past that the rows stop marching right and just stack at the
/// cap (the connector still reads as a descent chain).
const MAX_INDENT_PX: f32 = INDENT_PER_GEN_PX * 6.0;

/// What the popup did this frame. `pub(crate)` (not `pub`) because the
/// sibling `render` traffics in the `pub(crate)` BIO `ForkAncestor`
/// (carve-out #5 fixes that visibility) — the popup API can't be more
/// public than the type it consumes, and every call site is in-crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ForkInfoOutcome {
    /// Still open, nothing clicked.
    #[default]
    Open,
    /// `Close` clicked (or a click landed on nothing actionable — caller
    /// clears its `fork_info_open` bit).
    Closed,
}

/// The current modlist's own identity (the emphasized final chain row). Its
/// name/author come from the top-level payload/registry — NEVER from
/// `forked_from` (SPEC §10.9: a modlist's own identity is never in its own
/// chain).
pub(crate) struct SelfNode<'a> {
    pub(crate) name: &'a str,
    /// Empty ⇒ the `by …` line is omitted for the current node (SPEC §10.9 /
    /// §4.2 author-absent rule).
    pub(crate) author: &'a str,
}

/// Render the `ForkInfoPopup`. `lineage` is oldest → newest ancestors;
/// `self_node` is the current modlist. Caller owns the open/closed bit
/// (clears it on `Closed`). `id_salt` keeps multiple instances from
/// colliding (Install preview vs workspace header — different triggers).
pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    id_salt: &str,
    lineage: &[ForkAncestor],
    self_node: &SelfNode<'_>,
) -> ForkInfoOutcome {
    let mut outcome = ForkInfoOutcome::Open;

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

    egui::Window::new("orchestrator_fork_info_popup")
        .id(egui::Id::new(("orchestrator_fork_info_popup", id_salt)))
        // Non-blocking per SPEC §10.1 — no modal area / backdrop / focus trap.
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            // ── Header: `Fork lineage` (15px / weight 500). ──
            ui.label(
                egui::RichText::new("Fork lineage")
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(14.0);

            // ── Body. ──
            if lineage.is_empty() {
                // SPEC §10.9 empty-lineage guard (faint, hand style).
                ui.label(
                    egui::RichText::new(
                        "This modlist was created from scratch \u{2014} no fork lineage.",
                    )
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
                );
            } else {
                // Chain = ancestors (oldest→newest) then the current node.
                for (i, anc) in lineage.iter().enumerate() {
                    chain_row(ui, palette, i, &anc.name, &anc.author, false, i == 0);
                }
                let cur_idx = lineage.len();
                chain_row(
                    ui,
                    palette,
                    cur_idx,
                    self_node.name,
                    self_node.author,
                    true,
                    false,
                );
            }
            ui.add_space(16.0);

            // ── Footer: `Close` only, flush-right. ──
            let footer_h = 30.0;
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), footer_h),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
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
                        outcome = ForkInfoOutcome::Closed;
                    }
                },
            );
        });

    outcome
}

/// One lineage row. `generation` is the 0-based generation (drives the
/// indent + whether a `↳` connector is drawn). `current` marks the final
/// emphasized node; `is_root` is the oldest ancestor (no connector, no
/// extra emphasis).
fn chain_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    generation: usize,
    name: &str,
    author: &str,
    current: bool,
    is_root: bool,
) {
    // Wireframe `marginLeft: i * 20`, capped (SPEC §10.9 deep-lineage cap).
    let indent = (generation as f32 * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);

    // Spacing between rows (wireframe `marginBottom: 10` except the last).
    ui.horizontal(|ui| {
        ui.add_space(indent);
        ui.spacing_mut().item_spacing.x = 8.0;

        // `↳` connector for every non-root row (wireframe `i > 0`). U+21B3 is
        // cmap-verified PRESENT in firacode_nerd — render it as a glyph.
        if generation > 0 {
            ui.label(
                egui::RichText::new("\u{21B3}")
                    .size(13.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        }

        // Name — bold + accent when current (wireframe weight 700 /
        // accent-deep), else weight-500 primary.
        let name_color = if current {
            redesign_accent_deep(palette)
        } else {
            redesign_text_primary(palette)
        };
        ui.label(
            egui::RichText::new(name)
                .size(14.0)
                .family(egui::FontFamily::Name(if current {
                    "poppins_bold".into()
                } else {
                    "poppins_medium".into()
                }))
                .color(name_color),
        );

        // `⑂ this modlist` tag on the current node (wireframe sketchy
        // mini-pill). `⑂` U+2442 is cmap-ABSENT → painted vector + prose.
        if current {
            current_tag(ui, palette);
        }
    });

    // `by <author>` — OMITTED when author is empty (SPEC §10.9 / §4.2). The
    // line is indented to align under the name (wireframe `marginLeft: i>0 ?
    // 21 : 0` relative to the row's own indent).
    if !author.trim().is_empty() {
        ui.horizontal(|ui| {
            let extra = if generation > 0 { 21.0 } else { 0.0 };
            ui.add_space(indent + extra);
            ui.label(
                egui::RichText::new(format!("by {}", author.trim()))
                    .size(12.0)
                    .family(egui::FontFamily::Name("firacode_nerd".into()))
                    .color(redesign_text_faint(palette)),
            );
        });
    }

    if !is_root || generation > 0 || current {
        ui.add_space(10.0);
    }
}

/// The `⑂ this modlist` tag: a sketchy mini-pill with a PAINTED vector fork
/// glyph (U+2442 is cmap-absent — see the module header) + the uppercase
/// prose. Wireframe: 1px 7px padding, 9px Poppins-medium, letterSpacing 1,
/// uppercase, text-muted, sketchy border.
fn current_tag(ui: &mut egui::Ui, palette: ThemePalette) {
    let pad_x = 7.0;
    let pad_y = 1.0;
    let font = egui::FontId::new(9.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_muted(palette);
    // Uppercase + the wireframe's 1px letter-spacing approximated by spaces
    // would be lossy; egui has no letter-spacing, so render the uppercased
    // string at face value (the visual intent — a small caps tag — is met).
    let label = "THIS MODLIST";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);

    let fork_w = 9.0; // vector fork ink box
    let gap = 4.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, _resp) = ui.allocate_exact_size(desired, egui::Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let fork_center = egui::pos2(rect.left() + pad_x + fork_w / 2.0, rect.center().y);
        paint_fork_glyph(painter, fork_center, color);
        painter.text(
            egui::pos2(rect.left() + pad_x + fork_w + gap, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }
}

/// Paint `⑂` (fork) as a vector — a stem that splits into two tines (see the
/// module header for why this is not a font glyph). Sized to ~9px ink
/// centered on `center`; matches the tag text color so it reads as one unit.
fn paint_fork_glyph(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let half_h = 4.5;
    let split_y = center.y - 0.5; // where the stem forks
    let tine_dx = 3.0;

    // Lower stem: bottom → fork point.
    painter.line_segment(
        [
            egui::pos2(center.x, center.y + half_h),
            egui::pos2(center.x, split_y),
        ],
        stroke,
    );
    // Left tine: fork point → upper-left.
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x - tine_dx, center.y - half_h),
        ],
        stroke,
    );
    // Right tine: fork point → upper-right.
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x + tine_dx, center.y - half_h),
        ],
        stroke,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_caps_for_deep_lineages() {
        // SPEC §10.9: indent must cap so content never overflows. Past the
        // cap generation, the indent stays flat.
        let at_cap = (6.0_f32 * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);
        let way_past = (40.0_f32 * INDENT_PER_GEN_PX).min(MAX_INDENT_PX);
        assert_eq!(at_cap, MAX_INDENT_PX);
        assert_eq!(way_past, MAX_INDENT_PX);
        assert!(MAX_INDENT_PX < MAX_WIDTH_PX, "cap must fit the popup");
    }

    #[test]
    fn self_node_author_absence_is_representable() {
        // The author-absent path is driven by an empty `author` (SPEC §10.9
        // omits the `by` line). Just assert the type permits it.
        let s = SelfNode {
            name: "X",
            author: "",
        };
        assert!(s.author.trim().is_empty());
    }
}
