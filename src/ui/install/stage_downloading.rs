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
//       (D2) the rows live inside a vertical ScrollArea so 50+ mods are
//       all reachable; the overall-progress box above stays fixed.
//       (D4) 3 columns — the fabricated per-row progress-bar column is
//       removed (BIO emits no per-asset %, so a per-row bar would be fake;
//       the lone honest bar is the single overall one in the box above).
//       <div grid cols:"1.8fr 1fr 130px" gap:"6px 12px" align:center>
//         mod / source / status            (column headers, hand,text-muted)
//         {rows.map(m => (
//           <Label color:{queued? text-faint : text}>{m.name}</Label>
//           <Label fontSize:13 color:text-faint>{m.source}</Label>
//           <Label color:{statusColor}>{statusText}</Label>
//         ))}
//     </Box>
//     <div flex:1 />
//     <SubFlowFooter onBack={onCancel} backLabel="Cancel"
//                    onPrimary={onContinue} primaryLabel={continueLabel} />
//   </div>
//
// `statusText` / `statusColor` (D4 — status only, no per-row %):
//   done       → "✓ staged"        · success-green
//   extracting → "extracting..."   · text (normal)
//   downloading→ "downloading"     · text (normal)   [no fabricated N%]
//   queued     → "queued"          · text-faint
//
// The single overall bar (the fixed box above) = the average of each
// row's **monotonic** `phase_fraction` (Queued 0 ≤ Downloading .45 ≤
// Extracting .80 ≤ Staged 1.0) — D3: it never regresses (fixes the
// reported 100→80 stall), and the "N / T" count counts download-complete
// rows so it advances through the phases (fixes the "stuck 0/51").
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
//
//  **Final P7 Fix-Run (SPEC §13.13 / §13.1 / §13.3).** In `render_live`'s
//  one-shot `pipeline_armed` arm, **after**
//  `auto_build_driver::prepare_install_dirs_and_maybe_import` returns `Ok`
//  (so `import_modlist_share_code` populated `WizardState` — the §13.13
//  bundle's `pack_meta` exports from it), this screen calls
//  `install_runtime::install_modlist_registration::register_and_write_
//  install_start_artifacts`. That registers a net-new in-progress
//  `ModlistEntry` for a fresh Install-Modlist *paste* (the exact
//  `operations_create::create_modlist` convention — SPEC §13.1; a
//  Reinstall reuses its existing `pending_reinstall_id` entry, no second
//  registration), writes the committed `start_hooks::write_install_start_
//  artifacts` §13.13 bundle for it (`modlist-import-code.txt` +
//  install-start `latest_share_code` + `install_started_at`, variant-gated
//  by the Run-2 matrix), and sets `OrchestratorApp::active_install_
//  modlist_id` so the C3 clean-exit edge flips it InProgress → Installed
//  (the Install-Modlist screen has no `loaded_workspace_id`). This closes
//  the P7.T11 / SPEC §13.13 / Verification-#5 gap the Run-4a trigger-split
//  opened (Install-Modlist-paste & Reinstall bypass `on_install_start`).
//  It NEVER flips `start_install_requested` (the pipeline's
//  `start_auto_build_install` owns that) and NEVER re-derives dirs /
//  re-applies flag policies / does the Reinstall state-flip (all already
//  done by `prepare_install_dirs_and_maybe_import` /
//  `reinstall_flip_at_install_click`). One-shot via the same
//  `pipeline_armed` latch. Zero BIO source.
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
    redesign_border_strong, redesign_input_bg, redesign_pill_danger, redesign_shell_bg,
    redesign_success, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// The `✓` staged-checkmark glyph. U+2713 IS present in the full FiraCode
/// Nerd build (cmap-verified, HANDOFF caveat) → rendered as a glyph, not a
/// vector. Kept as a named constant so the symbol-glyph rule is visible at
/// the call site.
const CHECK_STAGED: &str = "\u{2713}"; // ✓

/// Per-mod download/extract lifecycle (SPEC §4.3; wireframe `m.status`).
/// Ordered as the row progresses: `Queued` → `Downloading` → `Extracting`
/// → `Staged`.
///
/// **D4 — status-only, no fabricated per-row %.** BIO emits ONLY an
/// aggregate download % (no per-asset progress — see
/// `DownloadProgress::from_wizard_state`); the old `Downloading { progress
/// }` mirrored that single aggregate onto *every* in-flight row, so 51
/// rows all showed an identical "downloading 6%" (the user's "confusing"
/// complaint) and a meaningless per-row bar. The redesign drops the
/// fabricated per-row % entirely: a row carries **status only**
/// (`Downloading` has no number); the lone honest progress signal is the
/// single overall bar (the aggregate) plus the advancing count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    /// Not started yet. Faint text.
    #[default]
    Queued,
    /// Archive fetch in progress (status only — no per-row %).
    Downloading,
    /// Archive extraction in progress.
    Extracting,
    /// Downloaded + extracted + staged. Success-green text.
    Staged,
}

impl ModDownloadStatus {
    /// The per-row status caption (D4 — status only, **no** "N%"). The
    /// wireframe's `statusText` had a `downloading <p>%`; the redesign
    /// drops the fabricated per-row number (BIO has no per-asset %), so a
    /// downloading row reads just `downloading`.
    pub fn status_text(self) -> String {
        match self {
            ModDownloadStatus::Queued => "queued".to_string(),
            ModDownloadStatus::Downloading => "downloading".to_string(),
            ModDownloadStatus::Extracting => "extracting...".to_string(),
            // The check is a separate glyph (firacode_nerd) laid before the
            // word at the call site — `status_text` returns the prose only so
            // the glyph/prose split mirrors `sub_flow_footer`.
            ModDownloadStatus::Staged => "staged".to_string(),
        }
    }

    /// **D3 — strictly monotonic non-decreasing phase fraction** (Queued ≤
    /// Downloading ≤ Extracting ≤ Staged) used only to compute the single
    /// overall bar. Because each later phase's fraction is ≥ the previous
    /// one, a row advancing (Downloading → Extracting → Staged) can only
    /// raise the average — the overall bar **never regresses** (this is the
    /// fix for the reported 100→80 stall, which was caused by the old
    /// `Downloading{100}` = 1.0 dropping to `Extracting` = 0.80 when a row
    /// moved from download to extract). There is no per-row bar any more
    /// (D4); this is purely an input to `DownloadProgress::overall_pct`.
    pub fn phase_fraction(self) -> f32 {
        match self {
            ModDownloadStatus::Queued => 0.0,
            // Downloading contributes partial progress so the bar moves
            // during the (longest) fetch phase instead of being 0 until
            // the first archive stages.
            ModDownloadStatus::Downloading => 0.45,
            // ≥ Downloading (monotonic — extraction is strictly more
            // progress than downloading, never less).
            ModDownloadStatus::Extracting => 0.80,
            ModDownloadStatus::Staged => 1.0,
        }
    }

    /// `true` only for `Staged` — the row's text uses success-green
    /// (wireframe `s === "done"`).
    pub fn is_done(self) -> bool {
        matches!(self, ModDownloadStatus::Staged)
    }

    /// **D3 — a row counts toward the advancing "N / T" once its archive
    /// has finished downloading** (`Extracting` or `Staged`). This makes
    /// the count advance *through* the phases (download → extract → stage)
    /// instead of being stuck `0 / 51` until the very last archive stages
    /// (the reported complaint). It is honest: the archive really is
    /// fetched once a row reaches Extracting; the remaining work is local
    /// unpacking. (`all_staged()` — the production auto-advance — still
    /// keys on *every* row being truly `Staged`, so correctness of "fully
    /// done" is unchanged.)
    pub fn download_complete(self) -> bool {
        matches!(
            self,
            ModDownloadStatus::Extracting | ModDownloadStatus::Staged
        )
    }

    /// `true` only for `Queued` — the row's name + status use `text-faint`
    /// (wireframe `s === "queued"`).
    pub fn is_queued(self) -> bool {
        matches!(self, ModDownloadStatus::Queued)
    }
}

/// One row of the SPEC §4.3 grid. **D4: 3 columns now** (mod / source /
/// status) — the fabricated per-row progress-bar column is removed (BIO has
/// no per-asset %; the only honest bar is the single overall one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDownloadRow {
    /// Mod display name (wireframe `m.name`).
    pub name: String,
    /// Source label, e.g. a repo or page host (wireframe `m.source`).
    pub source: String,
    /// Lifecycle status driving the status text (D4 — status only).
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
    ///   - `update_selected_download_running` ⇒ `Downloading` (**status
    ///     only — D4**). BIO does **not** expose a per-asset download %
    ///     (only an aggregate `"Downloading updates: N/M"` in
    ///     `scan_status`), so the old code mirrored that single aggregate
    ///     onto *every* in-flight row → 51 identical "downloading 6%" (the
    ///     user's "confusing" complaint). The redesign shows status only;
    ///     the aggregate drives the **single overall bar** instead (see
    ///     `overall_pct`), the one honest progress signal.
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
                    // D4 — status only; NO fabricated per-row %.
                    ModDownloadStatus::Downloading
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
    /// **D3 — the advancing "N" in "N / T mods".** Counts rows whose
    /// archive has finished downloading (`Extracting` or `Staged`), so the
    /// count climbs *through* the phases (download → extract → stage)
    /// instead of being stuck `0 / 51` until the very last archive stages
    /// (the reported "stuck 0/51" complaint). Honest — the archive really
    /// is fetched once a row reaches Extracting. (`all_staged()`, the
    /// production auto-advance, still requires *every* row truly `Staged`,
    /// so "fully complete" correctness is unchanged.)
    pub fn completed(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status.download_complete())
            .count()
    }

    /// Total row count — the "T" in the wireframe's "N / T mods".
    pub fn total(&self) -> usize {
        self.rows.len()
    }

    /// **D3 — the single honest overall bar, monotonic non-decreasing.**
    /// 0..=100 integer = the average of each row's **monotonic**
    /// `phase_fraction` (Queued 0 ≤ Downloading .45 ≤ Extracting .80 ≤
    /// Staged 1.0). Because every later phase's fraction is ≥ the earlier
    /// one, a row only ever *raises* the average as it advances — so the
    /// bar **cannot regress** (the fix for the reported 100→80 stall, which
    /// the old non-monotonic `Downloading{100}`=1.0 → `Extracting`=0.80
    /// drop caused). Empty ⇒ 0 (no divide-by-zero; also the "not started"
    /// rendering). This is the lone honest progress bar (D4 — no per-row
    /// bars).
    pub fn overall_pct(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        let sum: f32 = self.rows.iter().map(|r| r.status.phase_fraction()).sum();
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
    // The chassis path (Phase-6 fork-download) has no pipeline-arm step →
    // no arm-error surface.
    let back_clicked = render_chrome(ui, palette, copy, progress, None);
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

        // ── Fix A (the PLAN-GAP resolution — modlist id at derive time).
        //    The `-u` `weidu_component_logs` dir now lives at
        //    `%APPDATA%\bio\modlists\<id>\weidu_component_logs` (no-space,
        //    program-controlled — WeiDU's `-u` preflight rejects a
        //    space-containing destination). That needs the modlist id
        //    *before* `derive_per_install_dirs` runs:
        //      • Reinstall ⇒ the id is `pending_reinstall_id` (set by
        //        `reinstall_route::start_reinstall` before this screen).
        //      • Fresh Install-Modlist paste ⇒ the entry (and its id) was
        //        previously minted *inside* `register_install_modlist_
        //        paste`, which runs AFTER the derivation. Mint it ONCE here
        //        (same one-shot `pipeline_armed` latch) and thread the SAME
        //        id into BOTH the derivation and
        //        `register_and_write_install_start_artifacts` (which reuses
        //        it instead of re-minting) — behavior-neutral: the entry is
        //        byte-identical; only the id's birthplace moves one call
        //        earlier. Stored on `install_screen_state` so the
        //        registration step reads the SAME id. ──
        let modlist_id: String = orchestrator
            .pending_reinstall_id
            .clone()
            .unwrap_or_else(|| {
                orchestrator
                    .install_screen_state
                    .install_modlist_id
                    .clone()
                    .unwrap_or_else(crate::registry::ids::new_modlist_id)
            });
        // Persist the resolved id so the post-import registration reuses it
        // (only meaningful for the fresh-paste path; harmless for Reinstall,
        // whose registration step ignores it in favor of
        // `pending_reinstall_id`).
        orchestrator.install_screen_state.install_modlist_id = Some(modlist_id.clone());

        match auto_build_driver::prepare_install_dirs_and_maybe_import(
            &mut orchestrator.wizard_state,
            &modlist_id,
            &destination,
            game,
            workflow,
            &code,
        ) {
            Ok(_) => {
                // ── FIX 1 (SPEC §13.12a / §13.12 #5) — arm the
                //    download-archive policy so BIO's reused-unchanged
                //    `start_step2_update_download` actually runs (it
                //    early-returns unless `step1.download_archive == true`
                //    AND `step1.mods_archive_folder` is non-empty — BIO
                //    defaults `false` / `""`). The Install-Modlist-paste /
                //    Reinstall pipeline reaches the download tick via THIS
                //    `render_live` arm and never runs the workspace
                //    `on_install_start` (the sole `apply_flags` caller,
                //    which only sets `download`, never the archive fields)
                //    nor the workspace-open `sync_paths_from_settings`
                //    (which copies `mods_archive_folder` but never
                //    `download_archive`) — so without this the downloader
                //    silently no-ops ("downloading never starts"). The
                //    Mods-archive value is sourced EXACTLY as
                //    `sync_paths_from_settings` reads it (Settings → Paths;
                //    `Step1Settings → Step1State` conversion's
                //    `mods_archive_folder`) — mirrored, never invented; a
                //    `SettingsStore::load()` failure ⇒ empty string ⇒ the
                //    BIO downloader still no-ops with its own honest "Mods
                //    Archive folder is empty" status (no panic). Set AFTER
                //    `prepare_install_dirs_and_maybe_import`'s import so it
                //    is the final word before the per-frame poll's first
                //    `advance_pending_saved_log_flow` download tick (and
                //    survives `import_modlist_share_code`). The `&mut
                //    orchestrator.wizard_state` borrow taken by
                //    `prepare_install_dirs_and_maybe_import` above has ended
                //    (returned by value into this `match`), so reading
                //    `orchestrator.settings_store` here is sound. Covers
                //    BOTH Install-Modlist-paste and Reinstall (Reinstall
                //    routes Home→confirm→Install-Modlist preview→this
                //    Downloading screen). SPEC §13.12a settles the
                //    `download_archive`-unconditional question (the
                //    Mods-archive stage is "always" used, "never
                //    per-install" — not a user toggle), so no PLAN GAP.
                let mods_archive_folder = orchestrator
                    .settings_store
                    .load()
                    .map(|settings| {
                        let from: crate::app::state::Step1State = settings.step1.into();
                        from.mods_archive_folder
                    })
                    .unwrap_or_default();
                auto_build_driver::arm_download_archive_policy(
                    &mut orchestrator.wizard_state,
                    &mods_archive_folder,
                );

                // ── Final P7 Fix-Run (SPEC §13.13 / §13.1 — resolution A).
                //    The import succeeded ⇒ `WizardState` is populated, so
                //    the §13.13 bundle's `pack_meta`
                //    (`export_modlist_share_code(&wizard_state)`) now has the
                //    imported weidu logs to export. This is the SAME one-shot
                //    point as `arm_auto_build` (inside the `pipeline_armed`
                //    latch, reset by the caller on Cancel→Preview), so it
                //    fires exactly once per install start — never per frame.
                //    `register_and_write_install_start_artifacts`:
                //      • Reinstall (`pending_reinstall_id`) ⇒ reuse the
                //        existing entry (NO second registration; its
                //        Installed→InProgress flip is the Install-click
                //        site's job, already wired);
                //      • fresh Install-Modlist paste ⇒ register a net-new
                //        in-progress `ModlistEntry` (the exact
                //        `create_modlist` convention — SPEC §13.1) + write
                //        the empty `workspace.json` + atomic `modlists.json`
                //        save (the `start_scratch` precedent);
                //      • both ⇒ write the SPEC §13.13 bundle
                //        (`modlist-import-code.txt` + install-start
                //        `latest_share_code` + `install_started_at`,
                //        variant-gated by the Run-2 matrix) AFTER the import,
                //        and set `active_install_modlist_id` so the C3
                //        clean-exit edge flips THIS entry InProgress →
                //        Installed (the Install screen has no
                //        `loaded_workspace_id`). Closes the P7.T11 / SPEC
                //        §13.13 / Verification-#5 gap the Run-4a
                //        trigger-split opened. The `&mut orchestrator.wizard_
                //        state` borrow above has ended (returned by value),
                //        so the helper's `&mut OrchestratorApp` is sound.
                //    NEVER flips `start_install_requested` (the pipeline's
                //    `start_auto_build_install` owns that — a premature flip
                //    installs an empty Mods folder, the P7.T17 hazard);
                //    NEVER re-derives dirs / re-applies flag policies / does
                //    the Reinstall state-flip (all already done above /
                //    upstream). Zero BIO source.
                crate::install_runtime::install_modlist_registration
                    ::register_and_write_install_start_artifacts(orchestrator);
            }
            Err(err) => {
                // ── Non-masking arm-failure surface (the "it just sits
                //    there, no feedback" fix). Keep the one-shot latch
                //    `true` (do NOT spin-retry a bad code — re-importing
                //    every frame churns I/O; the original design intent)
                //    but record the error so `render_chrome` paints it
                //    PROMINENTLY. Previously only `step2.scan_status` was
                //    set — the empty grid hides it, so the screen looked
                //    like a permanent inert mystery "0 / 0 mods · no mods
                //    queued". Cancel → Preview (`clear_preview`) resets the
                //    latch + this error so the user can fix the
                //    code/destination and re-arm. ──
                orchestrator.install_screen_state.pipeline_arm_error = Some(err.clone());
                orchestrator.wizard_state.step2.scan_status =
                    format!("Auto Build could not start: {err}");
                tracing::warn!(
                    target = "orchestrator",
                    "P7.T17 pipeline arm failed: {err} (Downloading stays navigable; \
                     surfaced on-screen)"
                );
            }
        }
    }

    // ── (2) Content-addressed staging interposition AROUND BIO's
    //    reused-unchanged download/extract (SPEC §13.12a).
    //
    //    **D1 — these are ONE-SHOT per state transition, NOT per render
    //    frame (the reported freeze/hang fix).** Both calls are idempotent
    //    (running once is correct), but they do real disk I/O and
    //    `ingest_downloaded_archives` **FNV-hashes every downloaded
    //    archive**. Running `ingest` on the egui render path *every frame*
    //    for the whole post-download window (which spans extraction)
    //    re-hashed all archives every frame → the egui loop blocked → the
    //    reported hang (worst at extraction). The fix: gate each behind a
    //    one-shot latch on `InstallScreenState` so it fires exactly once at
    //    its state transition; the dedupe/coexist/lock semantics are
    //    unchanged (the functions themselves are untouched — only the
    //    scheduling changes). The latches reset on Cancel→Preview
    //    (`clear_preview`) so a re-entry re-stages from scratch.
    //      • `stage_known_archives` — runs ONCE, BEFORE BIO downloads
    //        (drops assets already in the store at this modlist's resolved
    //        hash + places them at BIO's deterministic extract path). Gated
    //        to before download starts so it does not race BIO mid-fetch.
    //      • `ingest_downloaded_archives` — runs ONCE, AFTER BIO's download
    //        lands (hashes + content-addresses + records the per-install
    //        lock). Gated to after download finished. Doing this single
    //        pass is the same total hashing BIO's download already did once
    //        — it is the *repeated per-frame* hashing that froze the UI. ──
    let s2 = &orchestrator.wizard_state.step2;
    let download_started =
        s2.update_selected_download_running || !s2.update_selected_downloaded_sources.is_empty();
    if !orchestrator.install_screen_state.archives_staged
        && orchestrator.install_screen_state.pipeline_armed
        && !destination.is_empty()
        && !download_started
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
    {
        // One-shot: the moment there are resolved assets and download has
        // not started, place any store-known archives and latch — never
        // re-run per frame.
        orchestrator.install_screen_state.archives_staged = true;
        archive_store::stage_known_archives(&mut orchestrator.wizard_state, &destination);
    }
    if !orchestrator.install_screen_state.archives_ingested
        && !destination.is_empty()
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
        // One-shot: the first frame BIO's download has finished, hash +
        // content-address the resolved set exactly once, then latch (the
        // priority freeze fix — no per-frame re-hash of GB archives across
        // the whole extraction window).
        orchestrator.install_screen_state.archives_ingested = true;
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
    // `from_wizard_state` returns an owned `DownloadProgress` (the
    // `&wizard_state` borrow ends here), so reading the arm-error off
    // `install_screen_state` next is borrow-sound.
    let progress = DownloadProgress::from_wizard_state(&orchestrator.wizard_state);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let back_clicked = render_chrome(ui, palette, copy, &progress, arm_error.as_deref());

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
///
/// `arm_error` is the **non-masking arm-failure surface**: `Some` only on
/// the live path when `prepare_install_dirs_and_maybe_import` returned
/// `Err` (the one-shot latch stays armed — no per-frame re-import — but the
/// failure is painted PROMINENTLY here instead of being buried in the
/// empty-grid-hidden `step2.scan_status`, which made the screen look like a
/// permanent inert mystery). `None` for the chassis / happy path → the
/// chrome is bit-identical to before.
fn render_chrome(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    copy: DownloadScreenCopy,
    progress: &DownloadProgress,
    arm_error: Option<&str>,
) -> bool {
    render_screen_title(ui, palette, copy.title, Some(copy.sub));
    ui.add_space(12.0);

    // ── Non-masking arm-failure banner (only when the pipeline arm
    //    failed). Danger-bordered, full-width, above the (empty) progress
    //    box so it cannot be missed — the fix for "it just sits there, no
    //    feedback". ──
    if let Some(err) = arm_error {
        render_arm_error_banner(ui, palette, err);
        ui.add_space(14.0);
    }

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

/// `Box label="mod progress"` — the per-mod grid.
///
/// **D2 — the grid is vertically scrollable.** With 50+ mods the list
/// previously overflowed the box and the lower rows were unreachable. It is
/// now wrapped in a vertical `egui::ScrollArea` (the overall-progress box
/// stays fixed above — it is rendered separately by `render_chrome`).
///
/// **D4 — 3 columns** (mod / source / status); the fabricated per-row
/// progress-bar column is removed (BIO emits no per-asset %; the only
/// honest bar is the single overall one in the fixed box above). Columns:
/// status fixed at 130px, the two flexible columns split the remainder
/// 1.8 : 1 (the wireframe's `1.8fr 1fr`), 12px gap.
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

        let col_gap = 12.0;
        let status_w = 130.0;
        // D4: only 3 columns now — the remainder (after the fixed status
        // column + 2 inter-column gaps) splits 1.8 : 1.
        let flex_total = (ui.available_width() - status_w - col_gap * 2.0).max(120.0);
        let mod_w = flex_total * (1.8 / 2.8);
        let src_w = flex_total * (1.0 / 2.8);

        // Header row stays fixed above the scrolled body so the columns
        // are always labelled while the rows scroll.
        egui::Grid::new("stage_downloading_mod_grid_header")
            .num_columns(3)
            .spacing(egui::vec2(col_gap, 6.0))
            .min_col_width(0.0)
            .show(ui, |ui| {
                grid_header(ui, palette, "mod", mod_w);
                grid_header(ui, palette, "source", src_w);
                grid_header(ui, palette, "status", status_w);
                ui.end_row();
            });

        if progress.rows.is_empty() {
            // No rows yet (e.g. the not-yet-wired fork-download chassis, or
            // before the pipeline resolves assets). Honest faint placeholder
            // rather than a blank box (the redesign's honest-empty-state
            // stance; consistent with stage_preview's parse-error path).
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("no mods queued")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_faint(palette)),
            );
            return;
        }

        // D2: the rows scroll. `auto_shrink([false, true])` keeps the
        // scroll area full-width but only as tall as needed (capped by the
        // box's available height — `render_chrome` reserved the footer's
        // footprint so this never overruns the visible area).
        egui::ScrollArea::vertical()
            .id_salt("stage_downloading_mod_scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("stage_downloading_mod_grid")
                    .num_columns(3)
                    .spacing(egui::vec2(col_gap, 6.0))
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        for row in &progress.rows {
                            render_grid_row(ui, palette, row, mod_w, src_w, status_w);
                            ui.end_row();
                        }
                    });
            });
    });
}

/// One data row of the D4 3-column grid (mod / source / status — no per-row
/// progress bar).
fn render_grid_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &ModDownloadRow,
    mod_w: f32,
    src_w: f32,
    status_w: f32,
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

    // Column 3 — status (D4 — the row's only state signal: queued →
    // downloading → extracting... → ✓ staged). Color: done → success-green,
    // queued → text-faint, else normal text. The `Staged` case lays the
    // `✓` glyph (firacode_nerd) before the prose (poppins_medium),
    // mirroring `sub_flow_footer`'s glyph/prose split.
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

// (D4) `parse_download_aggregate_pct` was removed: BIO's aggregate
// `"Downloading updates: N/M"` was only ever used to fabricate a per-row
// "downloading N%" mirrored onto every in-flight row. D4 drops that fake
// per-row %; the single honest overall bar is now the monotonic
// `phase_fraction` average (`DownloadProgress::overall_pct`), so the
// status-string parser is dead and has been deleted (no per-frame string
// parse either).

/// **The non-masking arm-failure banner.** A full-width danger-bordered
/// box (same chassis as `box_frame` but a danger stroke + a danger-toned
/// caption) that states the import / per-install-dir failure in plain
/// view, plus a quiet "Cancel → fix the code/destination → retry" hint.
/// Painted only when `prepare_install_dirs_and_maybe_import` returned
/// `Err`. This is what makes the screen diagnosable instead of a permanent
/// inert "0 / 0 mods · no mods queued" with the real reason hidden in the
/// empty grid's status sub-text.
fn render_arm_error_banner(ui: &mut egui::Ui, palette: ThemePalette, err: &str) {
    egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_pill_danger(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("could not start the download")
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_bold".into()))
                    .color(redesign_pill_danger(palette)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(err)
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Click Cancel, fix the import code or destination, and try again.",
                )
                .size(12.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_faint(palette)),
            );
        });
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
    fn status_text_is_status_only_no_fabricated_pct() {
        // D4: status only — a downloading row reads just "downloading"
        // (NOT "downloading N%"; BIO has no per-asset %).
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(ModDownloadStatus::Downloading.status_text(), "downloading");
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "extracting...");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "staged");
        // No status caption carries a '%' anywhere (the D4 guarantee).
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
        ] {
            assert!(
                !s.status_text().contains('%'),
                "D4: no fabricated per-row % in any status caption ({:?})",
                s
            );
        }
    }

    #[test]
    fn phase_fraction_is_strictly_monotonic_non_decreasing() {
        // D3: Queued ≤ Downloading ≤ Extracting ≤ Staged — the property
        // that makes the overall bar never regress (a row advancing can
        // only raise the average). Pin the exact ordering + the boundary
        // values.
        let q = ModDownloadStatus::Queued.phase_fraction();
        let d = ModDownloadStatus::Downloading.phase_fraction();
        let e = ModDownloadStatus::Extracting.phase_fraction();
        let s = ModDownloadStatus::Staged.phase_fraction();
        assert!(q <= d, "Queued ≤ Downloading ({q} ≤ {d})");
        assert!(d <= e, "Downloading ≤ Extracting ({d} ≤ {e})");
        assert!(e <= s, "Extracting ≤ Staged ({e} ≤ {s})");
        assert!((q - 0.0).abs() < f32::EPSILON);
        assert!((s - 1.0).abs() < f32::EPSILON);
        // Strictly increasing across the four phases (no two phases share a
        // fraction — so progress is always visible as a row advances).
        assert!(q < d && d < e && e < s, "strictly increasing per phase");
    }

    #[test]
    fn is_done_and_is_queued_and_download_complete_are_correct() {
        assert!(ModDownloadStatus::Queued.is_queued());
        assert!(!ModDownloadStatus::Queued.is_done());
        assert!(!ModDownloadStatus::Queued.download_complete());
        assert!(ModDownloadStatus::Staged.is_done());
        assert!(!ModDownloadStatus::Staged.is_queued());
        assert!(ModDownloadStatus::Staged.download_complete());
        assert!(!ModDownloadStatus::Extracting.is_queued());
        assert!(!ModDownloadStatus::Extracting.is_done());
        assert!(
            ModDownloadStatus::Extracting.download_complete(),
            "D3: a row past download (Extracting) counts toward the advancing N"
        );
        assert!(
            !ModDownloadStatus::Downloading.download_complete(),
            "D3: a still-downloading row is NOT yet download-complete"
        );
        assert!(!ModDownloadStatus::Downloading.is_queued());
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
    fn completed_counts_download_complete_rows_so_it_advances_through_phases() {
        // D3: "N" counts Extracting + Staged (download finished) so it
        // climbs through the phases instead of being stuck 0/T until the
        // last archive stages.
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting), // counts (downloaded)
                row("c", ModDownloadStatus::Downloading), // not yet
                row("d", ModDownloadStatus::Queued),
            ],
        };
        assert_eq!(
            p.completed(),
            2,
            "Staged + Extracting both count toward N (download complete)"
        );
        assert_eq!(p.total(), 4);
    }

    #[test]
    fn overall_pct_uses_monotonic_phase_fractions() {
        // 4 rows: staged(1.0) + extracting(0.80) + downloading(0.45) +
        // queued(0.0) → (2.25 / 4) * 100 = 56.25 → round → 56.
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
                row("c", ModDownloadStatus::Downloading),
                row("d", ModDownloadStatus::Queued),
            ],
        };
        assert_eq!(p.overall_pct(), 56);
    }

    #[test]
    fn overall_bar_never_regresses_as_a_row_advances_phases() {
        // THE D3 regression-fix proof (the reported 100→80 stall): a single
        // row walking Queued → Downloading → Extracting → Staged must yield
        // a non-decreasing overall %.
        let mut p = DownloadProgress {
            rows: vec![row("a", ModDownloadStatus::Queued)],
        };
        let q = p.overall_pct();
        p.rows[0].status = ModDownloadStatus::Downloading;
        let d = p.overall_pct();
        p.rows[0].status = ModDownloadStatus::Extracting;
        let e = p.overall_pct();
        p.rows[0].status = ModDownloadStatus::Staged;
        let s = p.overall_pct();
        assert!(
            q <= d && d <= e && e <= s,
            "overall % must be monotonic non-decreasing across phases \
             (got {q} → {d} → {e} → {s}) — the 100→80 stall must not recur"
        );
        assert_eq!(s, 100, "all-staged ⇒ 100%");
    }

    #[test]
    fn all_staged_only_when_every_row_truly_staged() {
        // Correctness of the production auto-advance is UNCHANGED by D3 —
        // it still requires every row Staged (Extracting is not enough).
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
        };
        assert!(p.all_staged());
        assert_eq!(p.overall_pct(), 100);
        p.rows[1].status = ModDownloadStatus::Extracting;
        assert!(
            !p.all_staged(),
            "an Extracting row (download-complete but not staged) must NOT auto-advance"
        );
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
    fn from_wizard_state_classifies_every_lifecycle_row_status_only() {
        // SPEC §4.3 live feed: one row per resolved asset; status from the
        // authoritative downloaded/extracted membership. D4: in-flight rows
        // are `Downloading` (status only — NOT a mirrored aggregate %).
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
        // both `Downloading` (status only — NO per-row %, NO 51-identical-%
        // confusion).
        st.step2.update_selected_download_running = true;
        st.step2.scan_status = "Downloading updates: 2/4".to_string();

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
            ModDownloadStatus::Downloading,
            "D4: in-flight row is status-only Downloading (no fabricated %)"
        );
        assert_eq!(p.rows[3].status, ModDownloadStatus::Downloading);
        // The two in-flight rows are byte-identical in status (no
        // per-row-% divergence) — exactly the "51 identical 6%" confusion
        // the redesign removes (now they are an honest plain "downloading").
        assert_eq!(p.rows[2].status, p.rows[3].status);

        // Not running, nothing downloaded ⇒ all Queued.
        let mut idle = WizardState::default();
        idle.step2.update_selected_update_assets = vec![asset("m", "s")];
        let q = DownloadProgress::from_wizard_state(&idle);
        assert_eq!(q.rows[0].status, ModDownloadStatus::Queued);

        // No resolved assets ⇒ empty grid (honest empty state).
        let empty = DownloadProgress::from_wizard_state(&WizardState::default());
        assert!(empty.rows.is_empty());
        assert!(!empty.all_staged());
    }
}
