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
//  Live wiring — Phase 7 P7.T17 (SPEC §13.12a). Phase 5 shipped this §4.3
//  chassis with an empty grid (the agreed forward-compatible scope after
//  the user's 2026-05-16 SPEC-CONFLICT decision); P7.T17 now feeds it
//  live. `render_live` (below) is the orchestrator-aware entry the
//  Install-Modlist dispatcher calls: it derives the per-install dirs +
//  arms BIO's import → auto-build pipeline via
//  `install_runtime::auto_build_driver` (which composes
//  `import_modlist_share_code` + BIO's saved-log/auto-build flow read-only
//  — zero BIO edit), interposes the net-new content-addressed staging
//  layer (`install_runtime::archive_store`) at the download/extract
//  boundary AROUND `app_step2_update_download`/`_extract` (reused
//  unchanged), builds `DownloadProgress` from the live BIO auto-build
//  state every frame (`DownloadProgress::from_wizard_state`), and advances
//  to the stage-4 seam when the pipeline reaches the install hand-off. The
//  pipeline itself is driven by the orchestrator's existing per-frame
//  `poll_step2_channels` (`advance_pending_saved_log_flow`) +
//  `poll_step5_before_render`/`start_step5_after_render` — this screen
//  only arms it, interposes the boundary, and renders the feed. The
//  preexisting parameterless `render` stays for the Phase-6 fork-download
//  chassis (still chassis-only there until that path is wired).
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

use crate::app::state::WizardState;
use crate::install_runtime::archive_store;
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
/// `InstallScreenState`. Phase 7 P7.T17 feeds it live from BIO's
/// auto-build state via [`DownloadProgress::from_wizard_state`] (the
/// Phase-5 empty-grid chassis is still used by the not-yet-wired
/// fork-download path).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Per-mod rows, modlist order.
    pub rows: Vec<ModDownloadRow>,
}

impl DownloadProgress {
    /// **P7.T17 live feed (SPEC §4.3 / §13.12a).** Project BIO's
    /// auto-build / update-download / extract state onto the §4.3 per-mod
    /// grid. One row per resolved download asset
    /// (`step2.update_selected_update_assets` — the mods the imported
    /// share code resolved to a concrete archive), its status derived from
    /// the same BIO progress signals the legacy update-selected popup
    /// reads:
    ///
    ///   - in `update_selected_downloaded_sources` **and**
    ///     `update_selected_extracted_sources` ⇒ `Staged` (✓).
    ///   - in `update_selected_downloaded_sources` only ⇒ `Extracting`
    ///     (downloaded, extract pending/running).
    ///   - `update_selected_download_running` ⇒ the not-yet-done assets
    ///     show `Downloading <agg>%`. BIO does **not** expose a per-asset
    ///     download %, only an aggregate it writes into `scan_status` as
    ///     `"Downloading updates: N/M"` (the *only* download-progress
    ///     signal BIO emits — `app_step2_update_download::poll_step2_
    ///     update_download` from the worker's `Progress { completed,
    ///     total }`). The aggregate is parsed from there and mirrored onto
    ///     each in-flight row — the most honest per-row value BIO's
    ///     granularity allows (and it advances as batches complete, never
    ///     a fake spinner).
    ///   - otherwise ⇒ `Queued`.
    ///
    /// The `downloaded` / `extracted` source vectors are
    /// `"<label> -> <path>"` / `"<label> -> <path>"` strings
    /// (`app_step2_update_download.rs:140` / `_extract` `remove_extracted_
    /// update_entries`), so membership is matched by the asset `label`
    /// prefix — the same join BIO itself uses to reconcile them.
    #[must_use]
    pub fn from_wizard_state(state: &WizardState) -> Self {
        let s2 = &state.step2;

        // Aggregate download % parsed from BIO's `scan_status` — the ONLY
        // download-progress signal BIO emits (it writes
        // `"Downloading updates: N/M"` from the worker's
        // `Progress { completed, total }`; there is no numeric field).
        // `None` (status not in that form yet) ⇒ in-flight rows show 0%
        // until the first batch reports — honest, never a fabricated %.
        let agg_pct: u8 = parse_download_aggregate_pct(&s2.scan_status).unwrap_or(0);

        let label_done = |list: &[String], label: &str| {
            list.iter().any(|e| {
                e.split(" -> ")
                    .next()
                    .map(str::trim)
                    .is_some_and(|l| l == label)
            })
        };

        let rows = s2
            .update_selected_update_assets
            .iter()
            .map(|a| {
                let downloaded = label_done(&s2.update_selected_downloaded_sources, &a.label);
                let extracted = label_done(&s2.update_selected_extracted_sources, &a.label);
                let status = if extracted {
                    ModDownloadStatus::Staged
                } else if downloaded {
                    // Downloaded; extract pending or running.
                    ModDownloadStatus::Extracting
                } else if s2.update_selected_download_running {
                    ModDownloadStatus::Downloading { progress: agg_pct }
                } else {
                    ModDownloadStatus::Queued
                };
                ModDownloadRow {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    status,
                }
            })
            .collect();

        Self { rows }
    }
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

/// Render the Stage-3 download/extract **chassis** with a caller-supplied
/// `progress` model. Used directly by the not-yet-wired Phase-6
/// fork-download path (empty grid there) and by the render gate; the
/// Install-Modlist live path uses [`render_live`] (which feeds `progress`
/// from BIO's auto-build state + drives the pipeline). Auto-advances when
/// every row is `Staged` (SPEC §4.3 production auto-advance — never the
/// wireframe's `simulate complete →`, which is wireframe-only).
pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
) -> DownloadingOutcome {
    let back_clicked = render_chrome(ui, palette, copy, progress);
    if back_clicked {
        DownloadingOutcome::Cancel
    } else if progress.all_staged() {
        DownloadingOutcome::Advance
    } else {
        DownloadingOutcome::Stay
    }
}

/// **P7.T17 — the live Install-Modlist Downloading stage (SPEC §4.3 /
/// §13.12a).** Orchestrator-aware: arms BIO's import → auto-build pipeline
/// once, interposes the content-addressed staging layer at the
/// download/extract boundary every frame (around BIO's reused-unchanged
/// `app_step2_update_download` / `_extract`), feeds the §4.3 grid from the
/// live BIO state, and advances to the stage-4 seam when the pipeline
/// reaches the install hand-off.
///
/// The pipeline is **not** driven from here — the orchestrator's existing
/// per-frame `OrchestratorApp::poll_step2_channels`
/// (`advance_pending_saved_log_flow`) +
/// `poll_step5_before_render`/`start_step5_after_render` already run it
/// (P6.T2c / P7.T1). This screen only: (1) arms it once
/// (`auto_build_driver::prepare_install_dirs_and_maybe_import` — derives
/// the per-install dirs + `import_modlist_share_code` + `arm_auto_build`,
/// **never** flipping `start_install_requested`; the pipeline's own
/// `start_auto_build_install` does that after staging), (2) interposes
/// `archive_store` at the boundary, (3) renders the live feed, (4)
/// returns `Advance` at the stage-4 seam (P7.T15 / Run-4b consumes it —
/// for Run-4a `stage_installing` is still the documented stub).
///
/// Returns `Cancel` if the user clicks `← Cancel` (the caller resets the
/// arm latch + progress and returns to Preview), `Advance` at the seam,
/// else `Stay`.
pub fn render_live(
    ui: &mut egui::Ui,
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    copy: DownloadScreenCopy,
) -> DownloadingOutcome {
    use crate::install_runtime::auto_build_driver;

    let palette = orchestrator.theme_palette;

    // The Install-Modlist destination (paste-stage `FolderInput`) + the
    // game from the parsed share-code preview (the redesign never collects
    // the game on Install — SPEC §4 / §13.12a; it is the payload's game).
    let destination = orchestrator
        .install_screen_state
        .destination
        .trim()
        .to_string();
    let game = orchestrator
        .install_screen_state
        .parsed_preview
        .as_ref()
        .map(|p| crate::registry::model::Game::from_legacy_string(&p.game_install))
        .unwrap_or_default();
    // Install Modlist paste is a share-code-consuming workflow; the
    // destination-not-empty `Continue` choice promotes it to
    // Continue-Partial (still share-code-consuming — `--download` ON, plus
    // `-s`/`-c`). Either way `auto_build_driver` imports + arms.
    let workflow = if orchestrator.install_screen_state.is_partial() {
        crate::install_runtime::flag_policies::InstallWorkflow::ContinuePartialInstall
    } else {
        crate::install_runtime::flag_policies::InstallWorkflow::ShareCodeConsuming
    };
    let code = orchestrator
        .install_screen_state
        .import_code
        .trim()
        .to_string();

    // ── (1) Arm ONCE. Re-running the import / `arm_auto_build` every
    //    frame would re-set the `pending_saved_log_*` flags and reset the
    //    pipeline mid-flight. The latch lives on `InstallScreenState`
    //    (reset by the caller on Cancel → Preview). A failure leaves the
    //    latch true (do not spin-retry a bad code) + surfaces the BIO-
    //    status text in the grid's empty state via the live feed. ──
    if !orchestrator.install_screen_state.pipeline_armed {
        orchestrator.install_screen_state.pipeline_armed = true;
        match auto_build_driver::prepare_install_dirs_and_maybe_import(
            &mut orchestrator.wizard_state,
            &destination,
            game,
            workflow,
            &code,
        ) {
            Ok(_) => {}
            Err(err) => {
                // Surface in BIO's status line so the (otherwise empty)
                // grid reflects the failure rather than spinning. The
                // pipeline simply never starts; Cancel → Preview lets the
                // user fix the code/destination.
                orchestrator.wizard_state.step2.scan_status =
                    format!("Auto Build could not start: {err}");
                tracing::warn!(
                    target = "orchestrator",
                    "P7.T17 pipeline arm failed: {err} (Downloading stays navigable)"
                );
            }
        }
    }

    // ── (2) Content-addressed staging interposition AROUND BIO's
    //    reused-unchanged download/extract (SPEC §13.12a). Both calls are
    //    no-ops until there is something to act on, so running them every
    //    frame is safe + idempotent:
    //      • `stage_known_archives` BEFORE BIO downloads — drops assets
    //        already in the store at this modlist's resolved hash (no
    //        re-download) + places them at BIO's deterministic extract
    //        path. Gated to BEFORE download starts (`!download_running &&
    //        no downloaded sources yet`) so it does not race BIO mid-
    //        fetch.
    //      • `ingest_downloaded_archives` AFTER BIO's download lands —
    //        hashes + content-addresses + records the per-install lock.
    //        Gated to AFTER download finished
    //        (`!download_running && some downloaded sources`). ──
    let s2 = &orchestrator.wizard_state.step2;
    let download_started =
        s2.update_selected_download_running || !s2.update_selected_downloaded_sources.is_empty();
    if orchestrator.install_screen_state.pipeline_armed
        && !destination.is_empty()
        && !download_started
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
    {
        archive_store::stage_known_archives(&mut orchestrator.wizard_state, &destination);
    }
    if !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        // Hash the asset set BIO just resolved. `archive_file_name` is the
        // same logical name BIO's download/extract use, so re-deriving it
        // from the (unchanged) asset list is exact.
        let names: Vec<String> = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .map(crate::app::app_step2_update_download::archive_file_name)
            .collect();
        archive_store::ingest_downloaded_archives(&orchestrator.wizard_state, &destination, &names);
    }

    // ── (3) Build the live feed from BIO's auto-build state + render the
    //    §4.3 chassis. ──
    let progress = DownloadProgress::from_wizard_state(&orchestrator.wizard_state);
    let back_clicked = render_chrome(ui, palette, copy, &progress);

    // ── (4) Outcome. Cancel → caller resets latch + returns to Preview.
    //    Advance when BIO's pipeline reached the install hand-off
    //    (`start_auto_build_install` set current_step=4 +
    //    start_install_requested / install_running) — the stage-4 seam
    //    (P7.T15 / Run-4b). A *stopped* pipeline (preflight/source
    //    failure) is finished-but-not-reached-install: stay, the grid's
    //    BIO status line shows why (no silent advance). ──
    if back_clicked {
        return DownloadingOutcome::Cancel;
    }
    if auto_build_driver::pipeline_reached_install(&orchestrator.wizard_state) {
        return DownloadingOutcome::Advance;
    }
    auto_build_driver::log_if_pipeline_stopped(&orchestrator.wizard_state);
    DownloadingOutcome::Stay
}

/// The shared §4.3 chrome (title + overall-progress Box + 4-col grid +
/// footer). Returns whether the footer's `← Cancel` was clicked. Used by
/// both [`render`] (chassis) and [`render_live`] (live feed) so the visual
/// is bit-identical regardless of data source.
fn render_chrome(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
) -> bool {
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
    // the caller's `all_staged()` / pipeline-reached-install check.
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
    footer.back_clicked
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

/// Parse the aggregate download % out of BIO's `scan_status` line — the
/// **only** download-progress signal BIO emits. `app_step2_update_
/// download::poll_step2_update_download` sets it to exactly
/// `"Downloading updates: N/M"` (from the worker's `Progress { completed,
/// total }`). Returns `Some(0..=100)` only for that exact shape with
/// `M > 0`; any other status (idle / scanning / extracting / a finished
/// line) ⇒ `None` (the caller renders in-flight rows at 0%, never a
/// fabricated value). Format-coupled to BIO's literal (read-only — if BIO
/// ever changes the string this degrades to 0%, never to a wrong %).
fn parse_download_aggregate_pct(status: &str) -> Option<u8> {
    let rest = status.trim().strip_prefix("Downloading updates: ")?;
    let (done, total) = rest.split_once('/')?;
    let done: usize = done.trim().parse().ok()?;
    let total: usize = total.trim().parse().ok()?;
    if total == 0 {
        return None;
    }
    let pct = ((done.min(total) as f32 / total as f32) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    Some(pct)
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

    #[test]
    fn parse_download_aggregate_pct_only_matches_bios_literal() {
        // BIO's exact `app_step2_update_download::poll_step2_update_download`
        // status (`"Downloading updates: N/M"`).
        assert_eq!(
            parse_download_aggregate_pct("Downloading updates: 0/7"),
            Some(0)
        );
        assert_eq!(
            parse_download_aggregate_pct("Downloading updates: 7/7"),
            Some(100)
        );
        // 3/7 → 42.857 → round → 43.
        assert_eq!(
            parse_download_aggregate_pct("Downloading updates: 3/7"),
            Some(43)
        );
        // Any other status ⇒ None (the caller renders 0%, never a fake %).
        assert_eq!(parse_download_aggregate_pct("Idle"), None);
        assert_eq!(parse_download_aggregate_pct("Scanning..."), None);
        assert_eq!(
            parse_download_aggregate_pct("Download updates finished: 7 downloaded, 0 failed"),
            None
        );
        // Degenerate / malformed ⇒ None, never a panic or a wrong %.
        assert_eq!(
            parse_download_aggregate_pct("Downloading updates: 1/0"),
            None
        );
        assert_eq!(
            parse_download_aggregate_pct("Downloading updates: x/y"),
            None
        );
    }

    #[test]
    fn from_wizard_state_classifies_every_lifecycle_row() {
        // SPEC §4.3 live feed: one row per resolved asset; status from the
        // authoritative downloaded/extracted membership + the parsed
        // aggregate download %.
        let mut st = WizardState::default();
        let asset = |label: &str, src: &str| crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.to_string(),
            source_id: src.to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("https://x/{label}.zip"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![
            asset("EET", "github:eet"),
            asset("cdtweaks", "github:cdt"),
            asset("stratagems", "github:scs"),
            asset("spell_rev", "weasel:sr"),
        ];
        // EET fully done (downloaded + extracted) ⇒ Staged.
        st.step2.update_selected_downloaded_sources = vec![
            "EET -> C:/a/EET.zip".to_string(),
            "cdtweaks -> C:/a/cdt.zip".to_string(),
        ];
        st.step2.update_selected_extracted_sources = vec!["EET -> C:/m/EET".to_string()];
        // cdtweaks downloaded but not extracted ⇒ Extracting.
        // stratagems / spell_rev not downloaded; a download is running ⇒
        // stratagems shows Downloading <agg>, spell_rev too (no per-asset
        // %, both mirror the aggregate).
        st.step2.update_selected_download_running = true;
        st.step2.scan_status = "Downloading updates: 2/4".to_string(); // → 50%

        let p = DownloadProgress::from_wizard_state(&st);
        assert_eq!(p.rows.len(), 4);
        assert_eq!(p.rows[0].status, ModDownloadStatus::Staged, "EET staged");
        assert_eq!(p.rows[0].source, "github:eet");
        assert_eq!(
            p.rows[1].status,
            ModDownloadStatus::Extracting,
            "cdtweaks downloaded, extract pending ⇒ Extracting"
        );
        assert_eq!(
            p.rows[2].status,
            ModDownloadStatus::Downloading { progress: 50 },
            "in-flight row mirrors BIO's parsed aggregate %"
        );
        assert_eq!(
            p.rows[3].status,
            ModDownloadStatus::Downloading { progress: 50 }
        );

        // Not running, nothing downloaded ⇒ all Queued.
        let mut idle = WizardState::default();
        idle.step2.update_selected_update_assets = vec![asset("m", "s")];
        let q = DownloadProgress::from_wizard_state(&idle);
        assert_eq!(q.rows[0].status, ModDownloadStatus::Queued);

        // No resolved assets ⇒ empty grid (the chassis' honest empty
        // state; never auto-advances).
        let empty = DownloadProgress::from_wizard_state(&WizardState::default());
        assert!(empty.rows.is_empty());
        assert!(!empty.all_staged());
    }
}
