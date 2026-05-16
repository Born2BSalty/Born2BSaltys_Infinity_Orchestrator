// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Install Modlist — Stage 3 (downloading). SPEC §4.3 ("Downloading &
// extracting" — the shared `ImportDownloadScreen`), P5.T12. Reused by
// Phase 6's Create → fork-download with a different title / sub /
// continueLabel (the `DownloadScreenCopy` parameter exists for that — only
// the Install path is wired this run, per the dispatch brief).
//
// Mirrors `wireframe-preview/screens.jsx::ImportDownloadScreen`
// (line 3709-3765):
//   <div sk-page flex column height:100% padding:"20px 28px">
//     <ScreenTitle title={title} sub={sub} />
//     <Box label="overall progress" padding:14 marginBottom:14 flexShrink:0>
//       <div flex align:center gap:12>
//         <Label fontSize:16 width:180>{done} / {total} mods · {pct}%</Label>
//         <div flex:1 height:14 ...sketchyBorder bg:input-bg overflow:hidden>
//           <div width:`${pct}%` height:100% bg:accent />
//         </div>
//       </div>
//       {hint && <Label hand color:text-faint fontSize:14>{hint}</Label>}
//     </Box>
//     <Box label="mod progress" padding:12 minHeight:360>
//       <div grid cols:"1.8fr 1fr 130px 120px" gap:"6px 12px" align:center
//            fontSize:14>
//         mod / source / status / progress  (column headers, hand,text-muted)
//         {rows.map(m => (
//           <Label color:{queued? text-faint : text}>{m.name}</Label>
//           <Label fontSize:13 color:text-faint>{m.source}</Label>
//           <Label color:{statusColor}>{statusText}</Label>
//           <div height:8 ...sketchyBorder bg:input-bg overflow:hidden>
//             <div width:{barPct} bg:{queued? transparent : accent} />
//         ))}
//     </Box>
//     <div flex:1 />
//     <SubFlowFooter onBack={onCancel} backLabel="Cancel"
//                    onPrimary={onContinue} primaryLabel={continueLabel} />
//   </div>
//
// Wireframe `statusText` / `statusColor` / `barPct` (verbatim mapping):
//   done       → "✓ staged"        · success-green · bar 100%
//   extracting → "extracting..."   · text (normal) · bar 80%
//   downloading→ "downloading N%"  · text (normal) · bar N%
//   queued     → "queued"          · text-faint    · bar 0% (transparent fill)
//
// **Symbol-glyph rule (cmap-verified, HANDOFF caveat).** The `✓` U+2713 in
// "✓ staged" IS present in the full FiraCode Nerd build (math/dingbat-check
// range, cmap-verified) → it is rendered as a real glyph in `firacode_nerd`
// (not a vector). The footer's `← Cancel` rides through `sub_flow_footer`'s
// glyph-aware Back button (base-FiraCode `←` present). No Misc-Symbols /
// emoji glyph appears on this screen, so no vector painting is required here.
//
// ──────────────────────────────────────────────────────────────────────────
//  Live data is RESOLVED-DEFERRED, not an open escalation. The Run-5
//  escalation was decided by the user on 2026-05-16: SPEC §13.12a now
//  defines the per-install/global directory model + content-addressed
//  archive staging + the import→auto-build reuse contract, and assigns the
//  *live* wiring to Phase 7 P7.T17 (the pipeline terminates in the install
//  runtime). Phase 5 ships THIS §4.3 chassis only — that is the agreed
//  scope, not a gap. See SPEC §13.12a + overview.md 2026-05-16 revision log.
//
//  Why the grid is empty until P7.T17 (context, not a TODO): the per-mod
//  list is a byproduct of BIO's `modlist_auto_build` pipeline —
//  `import_modlist_share_code` (writes logs/TOML to the game folders +
//  resets the workflow) → scan → apply-saved-log → update-preview →
//  update-check worker → download → extract → rescan → install — driven by
//  `app_step2_saved_log_flow` through `app_update_cycle::poll_before_render`
//  and gated on a fully configured Step 1. Per SPEC §13.12a the global game
//  paths reach the orchestrator-owned `WizardState` via
//  `sync_paths_from_settings` (Settings → Paths §11.2 — the Install screen
//  never collected them), and a net-new content-addressed staging layer
//  *wraps* `app_step2_update_download` / `_extract` with zero BIO edit —
//  that is P7.T17's job, not a reimplementation or fork. Until it runs,
//  `DownloadProgress` has no feed, so every row renders `queued` and
//  auto-advance never fires: the screen stays navigable (Cancel → Preview)
//  and lights up additively the moment P7.T17 feeds it (the same
//  forward-compatible model Phase 5 used elsewhere).
// ──────────────────────────────────────────────────────────────────────────
//
// SPEC: §4.3 (Downloading), §4.4 (the stage it auto-advances into — the
//       Phase-7 stub this run), §12.1 / §12.2 (tokens / tones),
//       §1 (CRITICAL DIRECTIVE — reuse-vs-carve-out decision order).
//       Wireframe: screens.jsx:3709-3765.

// rationale: the `f32 as u8` casts are pixel-radius roundings of small
// positive constants and the percentage maths is saturating by construction
// (Cat 2); the doc-paragraph-length / line-count lints are subjective style
// on a faithfully-mirrored screen carrying a load-bearing escalation note
// (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_long_first_doc_paragraph,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_shell_bg, redesign_success,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// The `✓` staged-checkmark glyph. U+2713 IS present in the full FiraCode
/// Nerd build (cmap-verified, HANDOFF caveat) → rendered as a glyph, not a
/// vector. Kept as a named constant so the symbol-glyph rule is visible at
/// the call site.
const CHECK_STAGED: &str = "\u{2713}"; // ✓

/// Per-mod download/extract lifecycle (SPEC §4.3; wireframe `m.status`).
/// Ordered as the row progresses: `Queued` → `Downloading` → `Extracting`
/// → `Staged`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    /// Not started yet. Faint text, empty (transparent-fill) bar.
    #[default]
    Queued,
    /// Archive fetch in progress. Normal text, bar at `progress`%.
    Downloading {
        /// 0..=100 — clamped on render.
        progress: u8,
    },
    /// Archive extraction in progress. Normal text, bar held at 80%
    /// (wireframe `barPct`).
    Extracting,
    /// Downloaded + extracted + staged. Success-green text, full bar.
    Staged,
}

impl ModDownloadStatus {
    /// Wireframe `statusText(m)` — the verbatim per-row status caption.
    pub fn status_text(self) -> String {
        match self {
            ModDownloadStatus::Queued => "queued".to_string(),
            ModDownloadStatus::Downloading { progress } => {
                format!("downloading {}%", progress.min(100))
            }
            ModDownloadStatus::Extracting => "extracting...".to_string(),
            // The check is a separate glyph (firacode_nerd) laid before the
            // word at the call site — `status_text` returns the prose only so
            // the glyph/prose split mirrors `sub_flow_footer`.
            ModDownloadStatus::Staged => "staged".to_string(),
        }
    }

    /// Wireframe `barPct(m)` as a 0.0..=1.0 fraction (the wireframe's "0%" /
    /// "80%" / "100%" / "N%").
    pub fn bar_fraction(self) -> f32 {
        match self {
            ModDownloadStatus::Queued => 0.0,
            ModDownloadStatus::Downloading { progress } => f32::from(progress.min(100)) / 100.0,
            ModDownloadStatus::Extracting => 0.80,
            ModDownloadStatus::Staged => 1.0,
        }
    }

    /// `true` only for `Staged` — the bar/text use success-green (wireframe
    /// `s === "done"`).
    pub fn is_done(self) -> bool {
        matches!(self, ModDownloadStatus::Staged)
    }

    /// `true` only for `Queued` — the row's name + status use `text-faint`
    /// and the bar fill is transparent (wireframe `s === "queued"`).
    pub fn is_queued(self) -> bool {
        matches!(self, ModDownloadStatus::Queued)
    }
}

/// One row of the SPEC §4.3 4-column grid (mod / source / status / progress).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDownloadRow {
    /// Mod display name (wireframe `m.name`).
    pub name: String,
    /// Source label, e.g. a repo or page host (wireframe `m.source`).
    pub source: String,
    /// Lifecycle status driving the status text + bar.
    pub status: ModDownloadStatus,
}

/// The Stage-3 download/extract progress model. Lives on
/// `InstallScreenState`. Populated by the resolved download orchestration
/// once the SPEC-CONFLICT escalation is decided (see the module header) — so
/// this run it stays empty and the screen renders the SPEC §4.3 chassis with
/// no rows / no progress (navigable + forward-compatible).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Per-mod rows, modlist order.
    pub rows: Vec<ModDownloadRow>,
}

impl DownloadProgress {
    /// Count of rows that have finished (reached `Staged`) — the "N" in the
    /// wireframe's "N / T mods".
    pub fn completed(&self) -> usize {
        self.rows.iter().filter(|r| r.status.is_done()).count()
    }

    /// Total row count — the "T" in the wireframe's "N / T mods".
    pub fn total(&self) -> usize {
        self.rows.len()
    }

    /// Overall completion percentage 0..=100 (integer, like the wireframe's
    /// `overall`). Empty list ⇒ 0 (no divide-by-zero; also the "nothing to
    /// do / not started" rendering).
    pub fn overall_pct(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        // Average of each row's bar fraction → a smooth overall bar (a
        // downloading row contributes partial progress, not 0/1). Matches
        // the wireframe's single accent bar that moves before any row is
        // fully staged.
        let sum: f32 = self.rows.iter().map(|r| r.status.bar_fraction()).sum();
        ((sum / total as f32) * 100.0).round() as u32
    }

    /// `true` when there is at least one row and every row is `Staged` — the
    /// production auto-advance condition (SPEC §4.3: "the next stage
    /// transitions automatically when downloads complete"). Empty ⇒ `false`
    /// (an empty list is "not started", never "complete").
    pub fn all_staged(&self) -> bool {
        !self.rows.is_empty() && self.rows.iter().all(|r| r.status.is_done())
    }
}

/// The reusable screen copy (wireframe `ImportDownloadScreen` props
/// `title` / `sub` / `hint` / `continueLabel`). The Install path passes the
/// SPEC §4.3 strings; Phase 6's fork-download passes its own — only the
/// Install path is wired this run.
#[derive(Debug, Clone, Copy)]
pub struct DownloadScreenCopy {
    /// `ScreenTitle` title.
    pub title: &'static str,
    /// `ScreenTitle` sub.
    pub sub: &'static str,
    /// Faint hand-style hint under the overall-progress bar (wireframe
    /// `hint`). `None` ⇒ no hint line.
    pub hint: Option<&'static str>,
}

impl DownloadScreenCopy {
    /// SPEC §4.3 + the Install-path wireframe invocation (screens.jsx:610).
    pub const INSTALL: DownloadScreenCopy = DownloadScreenCopy {
        title: "Downloading & extracting",
        sub: "fetching mod archives \u{2014} install starts automatically when ready",
        hint: Some("after download: install runs without further prompts (no review step)"),
    };
}

/// What the stage wants the dispatcher to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadingOutcome {
    /// Stay on the downloading stage (still in progress / nothing clicked).
    #[default]
    Stay,
    /// `← Cancel` clicked — back to the Preview stage (SPEC §4.3: "Cancel
    /// (← back)").
    Cancel,
    /// Downloads + extracts all finished — auto-advance to the next stage
    /// (SPEC §4.3 / §4.4: the install runtime, which is the Phase-7 stub
    /// this run). In production this fires with no manual click; the
    /// wireframe's `simulate complete →` primary is a wireframe-only
    /// affordance and is intentionally NOT shipped (see the run report's
    /// judgment-call note re: an optional dev-mode manual advance).
    Advance,
}

/// Render the Stage-3 download/extract screen. `progress` is the per-mod
/// model (empty this run — see the module header SPEC-CONFLICT note). Returns
/// what the dispatcher should do next.
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
) -> DownloadingOutcome {
    render_screen_title(ui, palette, copy.title, Some(copy.sub));
    ui.add_space(12.0);

    // ── Box label="overall progress" ──────────────────────────────────────
    render_overall_progress(ui, palette, copy.hint, progress);
    ui.add_space(14.0);

    // ── Box label="mod progress" — the 4-column grid ──────────────────────
    render_mod_progress(ui, palette, progress);

    // Bottom-pin the footer (wireframe `<div flex:1 />` spacer) while
    // reserving its footprint so it never overflows the visible area —
    // identical chassis to every other Install stage.
    let spacer = (ui.available_height() - sub_flow_footer::FOOTER_HEIGHT_PX).max(0.0);
    if spacer > 0.0 {
        ui.add_space(spacer);
    }

    // SPEC §4.3 footer: `Cancel` (← back) + (production) auto-advance on
    // completion. There is no manual "continue" in production — the
    // wireframe's `simulate complete →` is wireframe-only. The footer always
    // paints a right-aligned primary, so we paint a disabled placeholder
    // (`Waiting…`) that never emits a click; the real forward transition is
    // the `Advance` outcome below, driven by `progress.all_staged()`.
    let footer = sub_flow_footer::render(
        ui,
        palette,
        Some(BackBtn { label: "Cancel" }),
        None::<sub_flow_footer::SecondaryBtn<'_>>,
        None,
        PrimaryBtn {
            label: "Waiting\u{2026}",
            disabled: true,
        },
    );

    if footer.back_clicked {
        DownloadingOutcome::Cancel
    } else if progress.all_staged() {
        // Production auto-advance (SPEC §4.3). Never fires this run because
        // `progress` is empty pending the SPEC-CONFLICT decision.
        DownloadingOutcome::Advance
    } else {
        DownloadingOutcome::Stay
    }
}

/// `Box label="overall progress"` — the big "N / T mods · P%" label + the
/// accent progress bar + the optional faint hint (wireframe lines 3723-3733).
fn render_overall_progress(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    hint: Option<&str>,
    progress: &DownloadProgress,
) {
    box_frame(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("overall progress")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(8.0);

        let pct = progress.overall_pct();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            // Fixed 180px label column (wireframe `width:180`).
            let (label_rect, _) =
                ui.allocate_exact_size(egui::vec2(180.0, 20.0), egui::Sense::hover());
            ui.painter().text(
                egui::pos2(label_rect.left(), label_rect.center().y),
                egui::Align2::LEFT_CENTER,
                format!(
                    "{} / {} mods \u{00B7} {pct}%",
                    progress.completed(),
                    progress.total()
                ),
                egui::FontId::new(16.0, egui::FontFamily::Name("poppins_medium".into())),
                redesign_text_primary(palette),
            );

            // The flex:1 bar (wireframe height:14, sketchy border, input-bg
            // track, accent fill at `pct`%).
            let bar_w = ui.available_width();
            let (track, _) = ui.allocate_exact_size(egui::vec2(bar_w, 14.0), egui::Sense::hover());
            paint_bar(ui, palette, track, f64::from(pct) / 100.0, true);
        });

        if let Some(h) = hint {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(h)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
        }
    });
}

/// `Box label="mod progress"` — the 4-column grid (wireframe lines
/// 3734-3755). `gridTemplateColumns: "1.8fr 1fr 130px 120px"`.
fn render_mod_progress(ui: &mut egui::Ui, palette: ThemePalette, progress: &DownloadProgress) {
    box_frame(palette).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            egui::RichText::new("mod progress")
                .size(11.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_muted(palette)),
        );
        ui.add_space(8.0);

        // Column widths: the two fixed columns are wireframe-exact (130 / 120
        // px); the two flexible columns split the remainder 1.8 : 1 (wireframe
        // `1.8fr 1fr`). 12px inter-column gap (wireframe grid `gap:"6px 12px"`).
        let col_gap = 12.0;
        let status_w = 130.0;
        let prog_w = 120.0;
        let flex_total = (ui.available_width() - status_w - prog_w - col_gap * 3.0).max(120.0);
        let mod_w = flex_total * (1.8 / 2.8);
        let src_w = flex_total * (1.0 / 2.8);

        egui::Grid::new("stage_downloading_mod_grid")
            .num_columns(4)
            .spacing(egui::vec2(col_gap, 6.0))
            .min_col_width(0.0)
            .show(ui, |ui| {
                // Header row (wireframe: hand, text-muted).
                grid_header(ui, palette, "mod", mod_w);
                grid_header(ui, palette, "source", src_w);
                grid_header(ui, palette, "status", status_w);
                grid_header(ui, palette, "progress", prog_w);
                ui.end_row();

                if progress.rows.is_empty() {
                    // No rows yet (this run: always — see the module header
                    // SPEC-CONFLICT note). Render an honest, faint placeholder
                    // line rather than a blank box (the redesign's
                    // honest-empty-state stance; consistent with
                    // stage_preview's parse-error path).
                    ui.label(
                        egui::RichText::new("no mods queued")
                            .size(13.0)
                            .family(egui::FontFamily::Name("poppins_light".into()))
                            .color(redesign_text_faint(palette)),
                    );
                    ui.label("");
                    ui.label("");
                    ui.label("");
                    ui.end_row();
                    return;
                }

                for row in &progress.rows {
                    render_grid_row(ui, palette, row, mod_w, src_w, status_w, prog_w);
                    ui.end_row();
                }
            });
    });
}

/// One data row of the 4-column grid (wireframe lines 3741-3752).
fn render_grid_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &ModDownloadRow,
    mod_w: f32,
    src_w: f32,
    status_w: f32,
    prog_w: f32,
) {
    // Column 1 — mod name. `text-faint` while queued, else normal text
    // (wireframe `color: statusColor === text-faint ? text-faint : text`).
    let name_color = if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    sized_label(ui, mod_w, &row.name, 14.0, "poppins_medium", name_color);

    // Column 2 — source. Always faint, 13px (wireframe).
    sized_label(
        ui,
        src_w,
        &row.source,
        13.0,
        "poppins_light",
        redesign_text_faint(palette),
    );

    // Column 3 — status. Color per wireframe `statusColor`: done →
    // success-green, queued → text-faint, else normal text. The `Staged`
    // case lays the `✓` glyph (firacode_nerd) before the prose
    // (poppins_medium), mirroring `sub_flow_footer`'s glyph/prose split.
    let status_color = if row.status.is_done() {
        redesign_success(palette)
    } else if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    if row.status.is_done() {
        staged_cell(ui, palette, status_w, status_color);
    } else {
        sized_label(
            ui,
            status_w,
            &row.status.status_text(),
            14.0,
            "poppins_medium",
            status_color,
        );
    }

    // Column 4 — the per-row progress bar (wireframe height:8, sketchy
    // border, input-bg track; fill transparent while queued, else accent).
    let (track, _) = ui.allocate_exact_size(egui::vec2(prog_w, 8.0), egui::Sense::hover());
    paint_bar(
        ui,
        palette,
        track,
        f64::from(row.status.bar_fraction()),
        !row.status.is_queued(),
    );
}

/// The `✓ staged` cell — glyph in `firacode_nerd` (U+2713 is present,
/// cmap-verified), prose in `poppins_medium`, both success-green.
fn staged_cell(ui: &mut egui::Ui, _palette: ThemePalette, w: f32, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let glyph_font = egui::FontId::new(14.0, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(14.0, egui::FontFamily::Name("poppins_medium".into()));
    let glyph_galley = painter.layout_no_wrap(CHECK_STAGED.to_string(), glyph_font.clone(), color);
    let gap = 5.0;
    let cy = rect.center().y;
    painter.text(
        egui::pos2(rect.left(), cy),
        egui::Align2::LEFT_CENTER,
        CHECK_STAGED,
        glyph_font,
        color,
    );
    painter.text(
        egui::pos2(rect.left() + glyph_galley.size().x + gap, cy),
        egui::Align2::LEFT_CENTER,
        "staged",
        prose_font,
        color,
    );
}

/// A grid header cell — hand-style, `text-muted` (wireframe `Label hand
/// color:text-muted`).
fn grid_header(ui: &mut egui::Ui, palette: ThemePalette, text: &str, w: f32) {
    sized_label(
        ui,
        w,
        text,
        14.0,
        "poppins_light",
        redesign_text_muted(palette),
    );
}

/// Allocate a fixed-width cell and left-center a single-line label in it
/// (egui `Grid` columns don't hard-clip to a fraction width, so we allocate
/// the exact column rect and paint into it — keeps the 4 columns aligned to
/// the wireframe's `gridTemplateColumns`).
fn sized_label(
    ui: &mut egui::Ui,
    w: f32,
    text: &str,
    size: f32,
    family: &'static str,
    color: egui::Color32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 18.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().text(
            egui::pos2(rect.left(), rect.center().y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::new(size, egui::FontFamily::Name(family.into())),
            color,
        );
    }
}

/// Paint a sketchy-bordered progress bar into `track`: input-bg fill, 1.5px
/// border, accent fill clamped to `frac` (0.0..=1.0). `filled=false` keeps the
/// fill transparent (wireframe: queued rows show an empty track).
fn paint_bar(ui: &egui::Ui, palette: ThemePalette, track: egui::Rect, frac: f64, filled: bool) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    if filled {
        let frac = frac.clamp(0.0, 1.0) as f32;
        if frac > 0.0 {
            let fill_rect = egui::Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * frac, track.height()),
            );
            painter.rect_filled(fill_rect, radius, redesign_accent(palette));
        }
    }
    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

/// The shared sketchy Box chassis (shell-bg fill, 1.5px strong border, 3px
/// radius, 14px inner padding — matches `redesign_box`; we use a local frame
/// because this screen draws a section caption + custom interior layout
/// rather than `redesign_box`'s simple body closure).
fn box_frame(palette: ThemePalette) -> egui::Frame {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(14))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_text_is_wireframe_verbatim() {
        // screens.jsx:3711-3714 `statusText` — exact strings (the `Staged`
        // case returns prose only; the `✓` glyph is laid separately).
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(
            ModDownloadStatus::Downloading { progress: 42 }.status_text(),
            "downloading 42%"
        );
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "extracting...");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "staged");
    }

    #[test]
    fn downloading_progress_is_clamped_to_100() {
        // A misbehaving worker reporting >100 must not render "downloading
        // 250%" nor a bar past the track.
        assert_eq!(
            ModDownloadStatus::Downloading { progress: 250 }.status_text(),
            "downloading 100%"
        );
        let f = ModDownloadStatus::Downloading { progress: 250 }.bar_fraction();
        assert!((f - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn bar_fraction_matches_wireframe_barpct() {
        // screens.jsx:3715-3718 `barPct`: queued 0%, extracting 80%, done
        // 100%, downloading N%.
        assert!((ModDownloadStatus::Queued.bar_fraction() - 0.0).abs() < f32::EPSILON);
        assert!((ModDownloadStatus::Extracting.bar_fraction() - 0.80).abs() < f32::EPSILON);
        assert!((ModDownloadStatus::Staged.bar_fraction() - 1.0).abs() < f32::EPSILON);
        assert!(
            (ModDownloadStatus::Downloading { progress: 50 }.bar_fraction() - 0.50).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn is_done_and_is_queued_are_exclusive_and_correct() {
        assert!(ModDownloadStatus::Queued.is_queued());
        assert!(!ModDownloadStatus::Queued.is_done());
        assert!(ModDownloadStatus::Staged.is_done());
        assert!(!ModDownloadStatus::Staged.is_queued());
        assert!(!ModDownloadStatus::Extracting.is_queued());
        assert!(!ModDownloadStatus::Extracting.is_done());
        assert!(!ModDownloadStatus::Downloading { progress: 1 }.is_queued());
    }

    fn row(name: &str, status: ModDownloadStatus) -> ModDownloadRow {
        ModDownloadRow {
            name: name.to_string(),
            source: "src".to_string(),
            status,
        }
    }

    #[test]
    fn empty_progress_is_zero_and_not_complete() {
        // The "not started / pending the SPEC-CONFLICT decision" rendering:
        // no divide-by-zero, 0%, never auto-advances.
        let p = DownloadProgress::default();
        assert_eq!(p.completed(), 0);
        assert_eq!(p.total(), 0);
        assert_eq!(p.overall_pct(), 0);
        assert!(
            !p.all_staged(),
            "an empty list is 'not started', not 'complete'"
        );
    }

    #[test]
    fn completed_counts_only_staged_rows() {
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Downloading { progress: 10 }),
                row("c", ModDownloadStatus::Staged),
                row("d", ModDownloadStatus::Queued),
            ],
        };
        assert_eq!(p.completed(), 2);
        assert_eq!(p.total(), 4);
    }

    #[test]
    fn overall_pct_averages_row_fractions() {
        // 4 rows: staged(1.0) + extracting(0.8) + downloading50(0.5) +
        // queued(0.0) → (2.3 / 4) * 100 = 57.5 → round → 58.
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
                row("c", ModDownloadStatus::Downloading { progress: 50 }),
                row("d", ModDownloadStatus::Queued),
            ],
        };
        assert_eq!(p.overall_pct(), 58);
    }

    #[test]
    fn all_staged_only_when_every_row_done() {
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
        };
        assert!(p.all_staged());
        assert_eq!(p.overall_pct(), 100);
        p.rows[1].status = ModDownloadStatus::Extracting;
        assert!(!p.all_staged());
    }

    #[test]
    fn install_copy_is_spec_4_3_verbatim() {
        // SPEC §4.3 + the Install-path wireframe call (screens.jsx:610-617).
        let c = DownloadScreenCopy::INSTALL;
        assert_eq!(c.title, "Downloading & extracting");
        assert_eq!(
            c.sub,
            "fetching mod archives \u{2014} install starts automatically when ready"
        );
        assert_eq!(
            c.hint,
            Some("after download: install runs without further prompts (no review step)")
        );
    }
}
