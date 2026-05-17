// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Create → Import-and-modify — Fork-preview stage (`ForkPreviewScreen`).
// SPEC §5.3 + §13.3 + §10.9, P6.T8.
//
// **SPEC §5.3: "identical to Install's preview stage" — the same chassis,
// only the CTA + the no-gate differ.** This stage is the structural mirror
// of `install::stage_preview` (which is module-private, like `stage_paste`
// — so per the Run-3 precedent the *structure* is mirrored, not its
// internals copied; grep-prove: no `stage_preview` import). It reuses the
// truly-shared Phase-5 chassis **verbatim**:
//   - `preview_tabs::render_tab_strip` / `render_tab_body` — the exact
//     file-folder 6-tab strip + per-tab monospace body Install's preview
//     uses (SPEC §4.2 tab contents).
//   - `preview_counts::distinct_mod_count` — the same net-new distinct-TP2
//     mod count (BIO's public weidu-line parser, read-only).
//   - `fork_info_popup::render` — the **reused Phase-5 `ForkInfoPopup`**
//     (SPEC §10.9), here showing the *incoming parent's* lineage.
//   - `sub_flow_footer::render` — the shared sub-flow footer.
//   - `render_screen_title` / `redesign_box`.
//
// The **only** differences from Install's preview (SPEC §5.3):
//   1. Title / subline = the parsed **parent** code's packed `name` /
//      `author` (same honest fallback rules — SPEC §4.2 author-absent rule
//      is authoritative; the parent's name absent ⇒ `Shared modlist`,
//      author absent ⇒ drop the `by @… · ` segment, never fabricate).
//   2. The primary CTA is **`Begin Import →`** (hint "downloads mods,
//      applies selection + order, then drops you on Step 2"), and there is
//      **NO `allow_auto_install` gate** — forking is always allowed
//      regardless of the bit (SPEC §13.3: "Create → Import-and-modify
//      (fork-paste) ignores `allow_auto_install`"). No draft banner, no
//      disabled-primary, no `Open in Create →` secondary.
//   3. `← Back` returns to **fork-paste** (not Install's paste).
//
// The `⑂ fork info` button (shown only when the parent's `forked_from` is
// non-empty) opens the reused `ForkInfoPopup` with the **incoming parent's**
// chain + the parent's own name/author as the current node — exactly the
// SPEC §10.9 "Install preview / fork-preview" trigger ("uses the parsed
// share code's `forked_from` + its `name`/`author`").
//
// **Symbol-glyph rule (cmap-verified, HANDOFF caveat).** `⑂` U+2442 is
// cmap-ABSENT even in the full FiraCode Nerd build → the `⑂ fork info`
// button paints a VECTOR fork (the same approach `stage_preview`'s
// `fork_info_button` / `fork_info_popup::paint_fork_glyph` use). Base-
// FiraCode `← →` (cmap-present) ride through `sub_flow_footer`'s
// glyph-aware buttons. The `ForkInfoPopup` handles its own glyphs (reused).
//
// **Parse-failure path.** A real fork-paste parse failure is surfaced
// honestly (ScreenTitle + the error + a `← Back` footer), not a blank box —
// the redesign's honest-error stance, identical to Install's preview.
//
// SPEC: §5.3 (fork-preview — `ForkPreviewScreen`), §13.3 (Provenance — no
//       `allow_auto_install` gate on the fork path), §10.9 (ForkInfoPopup),
//       §4.2 (the Install-preview chassis this mirrors + author-absent
//       rule), §1 (carve-out #5 provenance fields).

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length /
// line-count lints are subjective style on a faithfully-mirrored screen
// (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::create::state_create::CreateScreenState;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::install::{preview_counts, preview_tabs};
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// SPEC §4.2-authoritative honest fallback when the parent code carries no
/// packed `name` (never fabricate; the same string Install's preview uses).
const FALLBACK_TITLE: &str = "Shared modlist";

/// What the fork-preview stage wants the dispatcher to do next. `pub(crate)`:
/// `render` reads the BIO `pub(crate)` `ModlistSharePreview` (carve-out #5
/// visibility) via `CreateScreenState`, so the API can't be more public;
/// the only caller is the in-crate `page_create` dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ForkPreviewOutcome {
    /// Stay on the fork-preview stage.
    #[default]
    Stay,
    /// `← Back` clicked — return to fork-paste (the dispatcher clears the
    /// cached preview).
    Back,
    /// `Begin Import →` clicked — advance to fork-download (the chassis;
    /// the registry/lineage append + Workspace route fire on completion).
    BeginImport,
}

/// Render the fork-preview stage. `state.fork_preview` /
/// `state.fork_preview_parse_error` were populated by the dispatcher's
/// parse-on-transition (one-shot). Mutates `state.fork_active_preview_tab` /
/// `state.fork_info_open` in place.
pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ctx: &egui::Context,
    state: &mut CreateScreenState,
) -> ForkPreviewOutcome {
    // ── Parse-failure path: honest error, not a blank box. ──
    if let Some(err) = state.fork_preview_parse_error.clone() {
        return render_parse_error(ui, palette, &err);
    }
    let Some(preview) = state.fork_preview.clone() else {
        return render_parse_error(
            ui,
            palette,
            "No preview is available. Go back and paste a share code.",
        );
    };

    let mut outcome = ForkPreviewOutcome::Stay;

    // ── Title row: ScreenTitle (parent name/subline) + (conditional)
    //    `⑂ fork info`. The parent's packed name drives the title; absent ⇒
    //    the honest fallback (SPEC §4.2 / §5.3 — never fabricate). ──
    let title = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_TITLE);
    let subline = build_subline(preview.author.as_deref());
    let has_lineage = !preview.forked_from.is_empty();

    ui.horizontal_top(|ui| {
        let fork_btn_w = if has_lineage { 110.0 } else { 0.0 };
        let title_w = (ui.available_width() - fork_btn_w).max(120.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                render_screen_title(ui, palette, title, Some(&subline));
            },
        );
        if has_lineage {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add_space(0.0);
                if fork_info_button(ui, palette).clicked() {
                    state.fork_info_open = true;
                }
            });
        }
    });

    // ── NO `allow_auto_install` gate (SPEC §13.3 — forking is always
    //    allowed). No draft banner, unlike Install's preview. ──

    // ── Overview Box (4-col grid) — identical to Install's preview. ──
    overview_box(ui, palette, &preview);
    ui.add_space(12.0);

    // ── Tab strip (merge) + Content Box — the reused Phase-5 widgets. ──
    preview_tabs::render_tab_strip(ui, palette, &mut state.fork_active_preview_tab);

    let content_h = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(80.0);
    let content_frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(14));
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), content_h),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            content_frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(content_h - 28.0);
                preview_tabs::render_tab_body(ui, palette, state.fork_active_preview_tab, &preview);
            });
        },
    );

    // ── SubFlowFooter (SPEC §5.3): `← Back` → fork-paste + `Begin Import →`
    //    primary (always enabled — NO `allow_auto_install` gate). ──
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some("downloads mods, applies selection + order, then drops you on Step 2"),
        PrimaryBtn {
            label: "Begin Import",
            disabled: false,
        },
    );

    if footer.back_clicked {
        outcome = ForkPreviewOutcome::Back;
    } else if footer.primary_clicked {
        outcome = ForkPreviewOutcome::BeginImport;
    }

    // ── ForkInfoPopup (SPEC §10.9) — the **reused Phase-5 widget**,
    //    rendered last so it floats above the stage. Identity = the parsed
    //    parent's top-level name/author (NEVER from `forked_from`). ──
    if state.fork_info_open {
        let self_author = preview.author.as_deref().unwrap_or("").trim();
        let self_name = preview
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(FALLBACK_TITLE);
        let result = fork_info_popup::render(
            ctx,
            palette,
            "create_fork_preview",
            &preview.forked_from,
            &SelfNode {
                name: self_name,
                author: self_author,
            },
        );
        if result == fork_info_popup::ForkInfoOutcome::Closed {
            state.fork_info_open = false;
        }
    }

    outcome
}

/// SPEC §4.2 subline (the Install-preview rule, authoritative on the
/// absent-author case): `by @<author> · review …` when `author` is present;
/// just `review …` when absent. Never invent an author.
fn build_subline(author: Option<&str>) -> String {
    let tail = "review what will be installed before BIO downloads anything";
    match author.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!("by {a} \u{00B7} {tail}"),
        None => tail.to_string(),
    }
}

/// The Overview Box — a 4-column grid, identical to Install's preview
/// (`Game` / `Mods` / `Components` / `BGEE/BG2EE entries`). `Mods` =
/// distinct TP2 across both logs via the reused `preview_counts`.
fn overview_box(ui: &mut egui::Ui, palette: ThemePalette, p: &ModlistSharePreview) {
    redesign_box(ui, palette, None, |ui| {
        let total_w = ui.available_width();
        let col_gap = 16.0;
        let col_w = ((total_w - col_gap * 3.0) / 4.0).max(60.0);

        let components = p.bgee_entries + p.bg2ee_entries;
        let mods = preview_counts::distinct_mod_count(&p.bgee_log_text, &p.bg2ee_log_text);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;
            overview_cell(ui, palette, col_w, "Game", &p.game_install);
            overview_cell(ui, palette, col_w, "Mods", &mods.to_string());
            overview_cell(ui, palette, col_w, "Components", &components.to_string());
            overview_cell(
                ui,
                palette,
                col_w,
                "BGEE/BG2EE entries",
                &format!("{}/{}", p.bgee_entries, p.bg2ee_entries),
            );
        });
    });
}

/// One Overview cell: `<label>: <strong value>`, 14px (Install-preview
/// parity).
fn overview_cell(ui: &mut egui::Ui, palette: ThemePalette, width: f32, label: &str, value: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(
                egui::RichText::new(format!("{label}:"))
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.label(
                egui::RichText::new(value)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_bold".into()))
                    .color(redesign_text_primary(palette)),
            );
        },
    );
}

/// The `⑂ fork info` title-row button. `⑂` U+2442 is cmap-ABSENT in the
/// shipped fonts → painted as a VECTOR fork + the prose in Poppins, side by
/// side (the same chassis Install's `stage_preview::fork_info_button` uses;
/// sketchy border, no accent fill, active-press transform).
fn fork_info_button(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let color = redesign_text_primary(palette);
    let label = "fork info";
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), color);

    let fork_w = 9.0;
    let gap = 5.0;
    let content_w = fork_w + gap + galley.size().x;
    let content_h = galley.size().y.max(fork_w);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    let pressed = response.is_pointer_button_down_on();
    let rect = if pressed {
        rect.translate(egui::vec2(1.0, 1.0))
    } else {
        rect
    };

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, redesign_shell_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let total_w = fork_w + gap + galley.size().x;
        let start_x = rect.center().x - total_w / 2.0;
        let cy = rect.center().y;
        paint_fork_glyph(painter, egui::pos2(start_x + fork_w / 2.0, cy), color);
        painter.text(
            egui::pos2(start_x + fork_w + gap, cy),
            egui::Align2::LEFT_CENTER,
            label,
            font,
            color,
        );
    }

    response
}

/// Paint `⑂` (fork) as a vector — a stem that splits into two tines (see the
/// module header for why this is not a font glyph; identical geometry to
/// `stage_preview::paint_fork_glyph` / `fork_info_popup::paint_fork_glyph`).
fn paint_fork_glyph(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4, color);
    let half_h = 4.5;
    let split_y = center.y - 0.5;
    let tine_dx = 3.0;
    painter.line_segment(
        [
            egui::pos2(center.x, center.y + half_h),
            egui::pos2(center.x, split_y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x - tine_dx, center.y - half_h),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x, split_y),
            egui::pos2(center.x + tine_dx, center.y - half_h),
        ],
        stroke,
    );
}

/// Honest parse-failure render: `ScreenTitle` + the error + a `← Back`
/// footer (no Overview / tabs / primary). Identical stance to Install's
/// preview parse-error path.
fn render_parse_error(ui: &mut egui::Ui, palette: ThemePalette, err: &str) -> ForkPreviewOutcome {
    render_screen_title(
        ui,
        palette,
        "Import and modify",
        Some("the pasted share code could not be read"),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(err)
            .size(13.0)
            .family(egui::FontFamily::Name("poppins_light".into()))
            .color(redesign_text_faint(palette)),
    );

    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        Some("fix the code on the paste screen, then preview again"),
        PrimaryBtn {
            label: "Begin Import",
            disabled: true,
        },
    );
    if footer.back_clicked {
        ForkPreviewOutcome::Back
    } else {
        ForkPreviewOutcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subline_includes_author_when_present() {
        assert_eq!(
            build_subline(Some("@b2bs")),
            "by @b2bs \u{00B7} review what will be installed before BIO downloads anything"
        );
    }

    #[test]
    fn subline_drops_author_segment_when_absent_or_blank() {
        // SPEC §4.2 author-absent rule — authoritative; never fabricate.
        let tail = "review what will be installed before BIO downloads anything";
        assert_eq!(build_subline(None), tail);
        assert_eq!(build_subline(Some("")), tail);
        assert_eq!(build_subline(Some("   ")), tail);
    }

    #[test]
    fn fallback_title_is_the_spec_authoritative_string() {
        // SPEC §4.2 / §5.3 — never fabricate a parent name.
        assert_eq!(FALLBACK_TITLE, "Shared modlist");
    }
}
