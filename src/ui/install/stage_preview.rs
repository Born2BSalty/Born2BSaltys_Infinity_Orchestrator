// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install Modlist — Stage 2 (preview). SPEC §4.2 + §13.3 + §10.9, P5.T10.
//
// Mirrors `wireframe-preview/screens.jsx` Install preview return
// (line 674-727):
//   <div sk-page flex column height:100%>
//     <div flex justify:space-between align:flex-start gap:12>
//       <ScreenTitle title={name} sub={`by ${author} · review …`} />
//       {forkedFrom.length>0 && <Btn small>⑂ fork info</Btn>}
//     </div>
//     {/* §4.2 draft-code banner (allow_auto_install==false) */}
//     <Box padding:14 flexShrink:0 marginBottom:12>            // Overview
//       <div grid 4×1fr gap:"6px 16px" fontSize:14>
//         Game / Mods / Components / BGEE/BG2EE entries
//       </div>
//     </Box>
//     <ImportPreviewTabs merge />
//     <Box flex:1 minHeight:0 overflow:auto>{PreviewText}</Box>
//     <SubFlowFooter onBack onPrimary primaryLabel="Import Modlist →" … />
//     <ForkInfoPopup … />
//   </div>
//
// **Provenance (SPEC §4.2 + §13.3, carve-out #5 fields on the parsed
// `ModlistSharePreview`):**
//   - Title    = packed `name`; absent ⇒ honest fallback `Shared modlist`
//                (never fabricate). Phase 5 is consume-only, so in practice
//                no real code carries `name` yet → the fallback is what
//                renders. The display lights up automatically once Phase 6/7
//                generation packs the trio.
//   - Subline  = `by @<author> · review what will be installed before BIO
//                downloads anything`; `author` absent ⇒ drop the
//                `by @<author> · ` segment (SPEC-authoritative over the
//                wireframe's always-present demo author). Never invent one.
//   - `⑂ fork info` button in the title row only when `forked_from` is
//                non-empty → opens the `ForkInfoPopup` (SPEC §10.9).
//
// **`allow_auto_install` gate (SPEC §4.2 + §13.3).** Read off the parsed
// preview (defaults `true` for older / field-less codes — carve-out #5). If
// `false`:
//   - info banner above the Overview Box (SPEC-verbatim copy);
//   - footer `Import Modlist →` disabled (greyed) + tooltip;
//   - secondary `Open in Create →` rendered between `← Back` and the
//     disabled primary → `NavDestination::Create` (the page dispatcher
//     applies the deferred nav; Phase 6 wires the code pre-load).
// `true` / absent ⇒ unchanged: enabled `Import Modlist →` advances to
// Downloading (Run-5 placeholder).
//
// **Overview values** (dispatch-brief settled prep):
//   - Game                = `game_install`
//   - BGEE/BG2EE entries  = `bgee_entries` / `bg2ee_entries`
//   - Components          = `bgee_entries + bg2ee_entries` (empirically
//                           pinned: wireframe 136 == 21+115 — exact, not a
//                           guess)
//   - Mods                = `—` (the phase's unknown-value precedent — the
//                           share preview carries no mod count; deriving it
//                           would need a hand-rolled weidu.log TP2 parser,
//                           an unresolved user decision — surfaced as open
//                           question "D" in the run report). NOT guessed.
//
// **Parse-failure path.** The wireframe assumes a valid code. A real
// paste-stage parse failure is surfaced honestly: when
// `preview_parse_error` is set the stage shows a `ScreenTitle` + the error
// + a `← Back` footer (no Overview / tabs / primary) — consistent with the
// redesign's honest-error stance, not a blank box. The parse itself runs in
// `page_install` on the `Paste → Preview` transition (cached, one-shot).
//
// **Symbol-glyph rule (cmap-verified, HANDOFF caveat).** `⑂` U+2442 is
// ABSENT even in the full FiraCode Nerd build → the `⑂ fork info` button
// paints a VECTOR fork (precedent: `left_rail.rs` nav icons /
// `destination_not_empty.rs::paint_warning_triangle`). Base-FiraCode
// `← →` (present, cmap-verified) ride through `sub_flow_footer`'s
// glyph-aware buttons.
//
// SPEC: §4.1 (footer context), §4.2 (preview), §13.3 (provenance +
//       generation — generation is NOT this run), §10.9 (ForkInfoPopup),
//       §6.4 (file-folder tab pattern), §12.1/§12.2 (tokens / tones),
//       §1 (carve-out #5). Wireframe: screens.jsx:674-727.

// rationale: `f32 as u8` casts are pixel-radius roundings of small positive
// constants — correct by construction (Cat 2); the doc-paragraph-length and
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
use crate::ui::install::preview_tabs;
use crate::ui::install::state_install::InstallScreenState;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn, SecondaryBtn};
use crate::ui::orchestrator::widgets::dialogs::fork_info_popup::{self, SelfNode};
use crate::ui::orchestrator::widgets::{redesign_box, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_SHADOW_OFFSET_BTN_PX,
    ThemePalette, redesign_border_strong, redesign_pill_danger, redesign_shadow, redesign_shell_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// SPEC-authoritative honest fallback when the code carries no packed
/// `name` (never fabricate). SPEC §4.2.
const FALLBACK_TITLE: &str = "Shared modlist";

/// The unknown-value placeholder for `Mods` (open question D — see header).
const UNKNOWN_VALUE: &str = "\u{2014}"; // —

/// SPEC §4.2-verbatim draft-code banner copy.
const DRAFT_BANNER: &str = "Draft modlist code \u{2014} this is not from a verified install. \
Review and customize the components in Create \u{2192} Import and modify before installing.";

/// SPEC §4.2-verbatim disabled-primary tooltip.
const DISABLED_IMPORT_TIP: &str =
    "Auto-install disabled for draft codes \u{2014} open in Create to review";

/// What the preview stage wants the dispatcher to do next. `pub(crate)`:
/// `render` takes the BIO `pub(crate)` `ModlistSharePreview` (carve-out #5
/// visibility) via `InstallScreenState`, so the API can't be more public;
/// the only caller is the in-crate `page_install` dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PreviewOutcome {
    /// Stay on the preview stage.
    #[default]
    Stay,
    /// `← Back` clicked — return to Paste (the dispatcher clears the cached
    /// preview).
    Back,
    /// `Open in Create →` clicked (draft-code gate) — the dispatcher
    /// switches to `NavDestination::Create` (Phase 6 wires the pre-load).
    OpenInCreate,
    /// `Import Modlist →` clicked (only emitted when the primary is enabled
    /// — i.e. `allow_auto_install == true`) — advance toward Downloading.
    Advance,
}

/// Render Stage 2. `state.parsed_preview` / `state.preview_parse_error` were
/// populated by the dispatcher's parse-on-transition (one-shot). Mutates
/// `state.active_preview_tab` / `state.fork_info_open` in place.
pub(crate) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    ctx: &egui::Context,
    state: &mut InstallScreenState,
) -> PreviewOutcome {
    // ── Parse-failure path: honest error, not a blank box. ──
    if let Some(err) = state.preview_parse_error.clone() {
        return render_parse_error(ui, palette, &err);
    }

    // No preview + no error should not happen (the dispatcher always parses
    // before entering Preview), but stay total: treat it as "go back".
    let Some(preview) = state.parsed_preview.clone() else {
        return render_parse_error(
            ui,
            palette,
            "No preview is available. Go back and paste a share code.",
        );
    };

    let mut outcome = PreviewOutcome::Stay;

    // ── Title row: ScreenTitle + (conditional) `⑂ fork info` button. ──
    let title = preview
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_TITLE);
    let subline = build_subline(preview.author.as_deref());
    let has_lineage = !preview.forked_from.is_empty();

    ui.horizontal_top(|ui| {
        // ScreenTitle takes the row; the fork-info button is flush-right.
        // Reserve the button's width so the title wraps before colliding.
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

    // ── §4.2 draft-code banner (allow_auto_install == false). ──
    let auto_install = preview.allow_auto_install;
    if !auto_install {
        draft_banner(ui, palette);
        ui.add_space(12.0);
    }

    // ── Overview Box (4-col grid). ──
    overview_box(ui, palette, &preview);
    ui.add_space(12.0);

    // ── Tab strip (merge) + Content Box (fills remaining height). ──
    preview_tabs::render_tab_strip(ui, palette, &mut state.active_preview_tab);

    // The Content Box stretches to fill the space between the strip and the
    // footer (wireframe `flex:1; minHeight:0; overflow:auto`). Reserve the
    // footer footprint so it stays inside the visible area.
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
                preview_tabs::render_tab_body(ui, palette, state.active_preview_tab, &preview);
            });
        },
    );

    // ── SubFlowFooter (SPEC §4.2). ──
    // Primary `Import Modlist →` is disabled when `allow_auto_install ==
    // false`; the `Open in Create →` secondary appears only in that case
    // (the escape hatch). `← Back` → Paste always.
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Back" }),
        if auto_install {
            None
        } else {
            Some(SecondaryBtn {
                label: "Open in Create",
            })
        },
        Some(if auto_install {
            "downloads, extracts, then runs install \u{2014} no review step"
        } else {
            DISABLED_IMPORT_TIP
        }),
        PrimaryBtn {
            label: "Import Modlist",
            disabled: !auto_install,
        },
    );

    if footer.back_clicked {
        outcome = PreviewOutcome::Back;
    } else if footer.secondary_clicked {
        outcome = PreviewOutcome::OpenInCreate;
    } else if footer.primary_clicked {
        // `primary_clicked` is only ever true when the primary is enabled
        // (sub_flow_footer suppresses clicks while disabled), so this is
        // unreachable when `auto_install == false` — the gate holds.
        outcome = PreviewOutcome::Advance;
    }

    // ── ForkInfoPopup (SPEC §10.9) — non-blocking, rendered last so it
    // floats above the stage. Its own identity is the parsed preview's
    // top-level name/author (NEVER from `forked_from`). ──
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
            "install_preview",
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

/// SPEC §4.2 subline. `by @<author> · review …` when `author` is present;
/// just `review …` when absent (SPEC-authoritative — never invent an
/// author). The wireframe always carries a demo author; the absent path is
/// what actually renders in Phase 5 (consume-only, no real code packs it).
fn build_subline(author: Option<&str>) -> String {
    let tail = "review what will be installed before BIO downloads anything";
    match author.map(str::trim).filter(|s| !s.is_empty()) {
        Some(a) => format!("by {a} \u{00B7} {tail}"),
        None => tail.to_string(),
    }
}

/// The Overview Box — a 4-column grid (wireframe `gridTemplateColumns:
/// repeat(4, 1fr); gap: 6px 16px; fontSize:14`). Each cell is
/// `Label: <strong value>`.
fn overview_box(ui: &mut egui::Ui, palette: ThemePalette, p: &ModlistSharePreview) {
    redesign_box(ui, palette, None, |ui| {
        // 4 equal columns. egui has no CSS grid; lay one horizontal row of
        // four equal-width cells (the wireframe grid is a single row of 4).
        let total_w = ui.available_width();
        let col_gap = 16.0;
        let col_w = ((total_w - col_gap * 3.0) / 4.0).max(60.0);

        // `Components` = BGEE + BG2EE entries (empirically pinned:
        // wireframe 136 == 21 + 115). `Mods` = unknown (open question D).
        let components = p.bgee_entries + p.bg2ee_entries;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;
            overview_cell(ui, palette, col_w, "Game", &p.game_install);
            overview_cell(ui, palette, col_w, "Mods", UNKNOWN_VALUE);
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

/// One Overview cell: `<label>: <strong value>` (wireframe `<Label>Game:
/// <strong>{…}</strong></Label>`), 14px.
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

/// The §4.2 draft-code info banner (wireframe uses a tinted Box for the
/// reinstall warning at the same spot; SPEC §4.2 specifies this banner copy
/// for the `allow_auto_install == false` gate). Coral-tinted to read as a
/// caution, matching the redesign's `pill_danger` tone family.
fn draft_banner(ui: &mut egui::Ui, palette: ThemePalette) {
    // Soft coral fill (the `#e69a96` danger tone at low alpha — same family
    // the wireframe's reinstall banner uses: `rgba(230,154,150,0.12)`).
    let fill = egui::Color32::from_rgba_premultiplied(0x1B, 0x12, 0x11, 31);
    let frame = egui::Frame::default()
        .fill(fill)
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_pill_danger(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .shadow(egui::epaint::Shadow {
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
        ui.label(
            egui::RichText::new(DRAFT_BANNER)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_pill_danger(palette)),
        );
    });
}

/// The `⑂ fork info` title-row button (wireframe `<Btn small>⑂ fork
/// info</Btn>`). `⑂` U+2442 is cmap-ABSENT in the shipped fonts (verified)
/// → painted as a VECTOR fork + the prose in Poppins, side by side (the
/// same chassis as the small `redesign_btn`: sketchy border, no accent
/// fill, active-press transform). Returns the `Response` for `.clicked()`.
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
/// module header for why this is not a font glyph; precedent:
/// `destination_not_empty.rs::paint_warning_triangle`,
/// `fork_info_popup.rs::paint_fork_glyph`). Each widget file paints its own
/// vectors per the codebase convention.
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

/// Honest parse-failure render: `ScreenTitle` + the error message + a
/// `← Back` footer (no Overview / tabs / primary). The wireframe assumes a
/// valid code; a real parse failure must be surfaced, not blanked.
fn render_parse_error(ui: &mut egui::Ui, palette: ThemePalette, err: &str) -> PreviewOutcome {
    render_screen_title(
        ui,
        palette,
        "Preview",
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
        None::<SecondaryBtn<'_>>,
        Some("fix the code on the paste screen, then preview again"),
        PrimaryBtn {
            label: "Import Modlist",
            disabled: true,
        },
    );
    if footer.back_clicked {
        PreviewOutcome::Back
    } else {
        PreviewOutcome::Stay
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
        let tail = "review what will be installed before BIO downloads anything";
        assert_eq!(build_subline(None), tail);
        assert_eq!(build_subline(Some("")), tail);
        assert_eq!(build_subline(Some("   ")), tail);
    }

    #[test]
    fn fallback_title_is_the_spec_authoritative_string() {
        // SPEC §4.2 — never fabricate a name; the honest generic fallback.
        assert_eq!(FALLBACK_TITLE, "Shared modlist");
    }

    #[test]
    fn draft_banner_copy_is_spec_verbatim() {
        // SPEC §4.2 exact banner string.
        assert!(DRAFT_BANNER.starts_with("Draft modlist code"));
        assert!(DRAFT_BANNER.contains("not from a verified install"));
        assert!(DRAFT_BANNER.contains("Create \u{2192} Import and modify"));
    }
}
