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
//       4 columns (mod / source / status / **per-mod byte bar**) — the
//       per-mod bar carries this mod's *own live byte fraction* during
//       download, advancing frame-by-frame (Wabbajack-grade), independent
//       of the 4-wide pool.
//       <div grid cols:"1.8fr 1fr 120px 150px" gap:"6px 12px" align:center>
//         mod / source / status / progress (column headers, hand,text-muted)
//         {rows.map(m => (
//           <Label color:{queued? text-faint : text}>{m.name}</Label>
//           <Label fontSize:13 color:text-faint>{m.source}</Label>
//           <Label color:{statusColor}>{statusText}</Label>
//           <Bar frac={m.bar_fraction()} />
//         ))}
//     </Box>
//     <div flex:1 />
//     <SubFlowFooter onBack={onCancel} backLabel="Cancel"
//                    onPrimary={onContinue} primaryLabel={continueLabel} />
//   </div>
//
// `statusText` / `statusColor`:
//   done       → "✓ staged"            · success-green
//   skipped    → "✓ already downloaded"· success-green (DL-Run 1 cached)
//   extracting → "extracting..."       · text (normal)
//   downloading→ "downloading"         · text (normal)
//   queued     → "queued"              · text-faint
//
// **DL-Run 2 — Wabbajack-grade two-phase model (this reverses the prior
// "D4 status-only / one-determinate-step" interim rationale; that was true
// of BIO's serial worker, which the redesign no longer uses for the
// download).** The screen is two explicit ordered phases, each its own
// independent 0→100, NEVER sharing a bar:
//   • **Download** overall = `Σ downloaded_bytes ÷ Σ expected_bytes`
//     across every to-fetch mod (a TRUE byte aggregate from the parallel
//     `stream_downloader`'s per-asset byte deltas — NOT N/M, NOT a
//     heuristic blend). DL-Run-1-skipped (already-present-by-hash) mods
//     count instantly-complete (their full size in both numerator and
//     denominator) so a mostly-cached install feels smooth/fast/honest
//     ("48 of 51 already present, downloading 3"). Monotonic.
//   • **Extract** overall = a SEPARATE bar that starts at 0 when the
//     extract phase begins and climbs `extracted ÷ total` to 100,
//     independently — it NEVER inherits the download value. (BIO's extract
//     exposes per-archive completion, not bytes, so it is count-granular
//     but rendered as its own clean monotonic 0→100.)
// The per-mod bar = THIS mod's live `downloaded_bytes / expected_size`,
// the WHOLE 0→1 fill during download (no artificial band-clamp), read
// fresh every egui frame from the byte map the drain keeps current — so a
// row at 37% renders beside a sibling at 81%, never a clump snapping
// 20→100. No-Content-Length ⇒ a graceful animated indeterminate fill.
// A clear phase label tells the user which phase + the N/M · P%.
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
/// → `Staged`. `Skipped` is the DL-Run-1 already-present-by-hash terminal
/// (the Wabbajack "already have it" — never downloaded, presented straight
/// to extract / staged).
///
/// **DL-Run 2.** The in-flight `Downloading` row's bar is NOT a phase nub
/// — it is the mod's *own live byte fraction* (`bytes / Content-Length`),
/// the whole 0→1 fill, read fresh every frame from `ModDownloadRow
/// ::per_byte` (see [`ModDownloadRow::bar_fraction`]). `phase_fraction`
/// here is only the non-byte fallback (chassis / fork-download / a
/// pre-first-byte frame / a no-Content-Length row).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    /// Not started yet. Faint text.
    #[default]
    Queued,
    /// Archive fetch in progress. The per-mod bar is this mod's live
    /// `bytes / Content-Length` (`ModDownloadRow::bar_fraction`), not a
    /// phase nub.
    Downloading,
    /// Archive extraction in progress.
    Extracting,
    /// Downloaded + extracted + staged. Success-green text.
    Staged,
    /// **DL-Run 1 — already present on disk by content hash** (the
    /// Wabbajack skip): never downloaded, the bytes were placed at BIO's
    /// extract path. Renders instantly-satisfied (full bar, success-green
    /// "✓ already downloaded") and counts complete in the download byte
    /// aggregate so a mostly-cached install does not lurch / sit at a false
    /// low.
    Skipped,
}

impl ModDownloadStatus {
    /// The per-row status caption. `Downloading` reads just `downloading`
    /// (the live fraction is on the per-mod bar, not the caption);
    /// `Skipped` reads `already downloaded` (prose only — the `✓` glyph is
    /// laid by the call site, mirroring `sub_flow_footer`'s glyph/prose
    /// split, the same as `Staged`).
    pub fn status_text(self) -> String {
        match self {
            ModDownloadStatus::Queued => "queued".to_string(),
            ModDownloadStatus::Downloading => "downloading".to_string(),
            ModDownloadStatus::Extracting => "extracting...".to_string(),
            // The check is a separate glyph (firacode_nerd) laid before the
            // word at the call site — `status_text` returns the prose only so
            // the glyph/prose split mirrors `sub_flow_footer`.
            ModDownloadStatus::Staged => "staged".to_string(),
            ModDownloadStatus::Skipped => "already downloaded".to_string(),
        }
    }

    /// **The non-byte fallback per-mod fill (0.0..=1.0).** The live path
    /// uses [`ModDownloadRow::bar_fraction`], which prefers the mod's real
    /// byte fraction; this is only reached when there is NO byte signal
    /// (chassis / fork-download / a frame before the first byte delta / a
    /// no-Content-Length row). Strictly monotonic across the lifecycle
    /// (Queued 0 < Downloading < Extracting < Staged/Skipped 1.0) so a
    /// fallback row never visually regresses.
    pub fn phase_fraction(self) -> f32 {
        match self {
            ModDownloadStatus::Queued => 0.0,
            // No byte signal yet — a small determinate nub so the row reads
            // as active (the live byte fraction takes over the moment the
            // first delta arrives via `ModDownloadRow::per_byte`).
            ModDownloadStatus::Downloading => 0.15,
            // Archive downloaded; local unpack in progress.
            ModDownloadStatus::Extracting => 0.65,
            // Staged and Skipped are both fully-satisfied terminals.
            ModDownloadStatus::Staged | ModDownloadStatus::Skipped => 1.0,
        }
    }

    /// `true` for the fully-satisfied terminals (`Staged` or DL-Run-1
    /// `Skipped`) — the row's text uses success-green.
    pub fn is_done(self) -> bool {
        matches!(self, ModDownloadStatus::Staged | ModDownloadStatus::Skipped)
    }

    /// **A row whose archive is no longer being fetched** — `Extracting`,
    /// `Staged`, or DL-Run-1 `Skipped` (its bytes are on disk; nothing left
    /// to download for it). Drives the Download-phase "N" count and the
    /// download-byte-aggregate "complete" treatment. (`all_staged()` — the
    /// production auto-advance — still keys on *every* row being a
    /// fully-extracted terminal.)
    pub fn download_complete(self) -> bool {
        matches!(
            self,
            ModDownloadStatus::Extracting | ModDownloadStatus::Staged | ModDownloadStatus::Skipped
        )
    }

    /// `true` only for `Queued` — the row's name + status use `text-faint`
    /// (wireframe `s === "queued"`).
    pub fn is_queued(self) -> bool {
        matches!(self, ModDownloadStatus::Queued)
    }

    /// `true` only for the DL-Run-1 already-present-by-hash terminal — the
    /// status cell reads "✓ already downloaded" instead of "✓ staged".
    pub fn is_skipped(self) -> bool {
        matches!(self, ModDownloadStatus::Skipped)
    }
}

/// One row of the SPEC §4.3 grid (mod / source / status / per-mod bar).
///
/// **#1 (P7.T17) — the per-mod bar now carries a REAL byte fraction.**
/// BIO's serial loop exposed no per-asset byte signal (only the aggregate
/// `"N/M"`), so the prior bar was a determinate phase step. The net-new
/// `install_runtime::stream_downloader` reads each response's
/// `Content-Length` and accumulates bytes, so a `Downloading` row's bar is
/// `bytes / total` while it streams. `per_byte` is `Some((bytes, total))`
/// once the streaming downloader has reported progress for this row;
/// `total` is `None` for a no-`Content-Length` response (indeterminate —
/// the bar then shows an active-but-unmeasured nub). Cleared / absent for
/// the chassis path (Phase-6 fork-download) → that path falls back to the
/// phase-fraction bar exactly as before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDownloadRow {
    /// Mod display name (wireframe `m.name`).
    pub name: String,
    /// Source label, e.g. a repo or page host (wireframe `m.source`).
    pub source: String,
    /// Lifecycle status driving the status text + tone.
    pub status: ModDownloadStatus,
    /// **#1 — live per-mod byte progress** from
    /// `install_runtime::stream_downloader`. `Some((downloaded_bytes,
    /// Some(content_length)))` ⇒ a real fraction; `Some((bytes, None))` ⇒
    /// byte-count with an indeterminate total (no `Content-Length`);
    /// `None` ⇒ no byte signal yet / the chassis path (phase-fraction
    /// fallback). Drained into here by `OrchestratorApp::
    /// drain_stream_download` each frame.
    pub per_byte: Option<(u64, Option<u64>)>,
    /// **DL-Run 2 — this mod's expected archive size in bytes** (from the
    /// share code's per-archive `{name,size,hash}` the DL-Run-1 exporter
    /// baked in — matched by `archive_file_name`). `Some` ⇒ the download
    /// byte aggregate uses it as this mod's denominator share (and a
    /// `Skipped` row contributes its full size to both numerator and
    /// denominator). `None` ⇒ no baked size (pre-redesign / third-party /
    /// un-hashable code) — the aggregate falls back to this row's live
    /// `Content-Length` for the denominator (and an indeterminate row is
    /// excluded from the determinate denominator). Carried through the
    /// per-frame rebuild from `DownloadProgress::expected_sizes`.
    pub expected_size: Option<u64>,
}

impl ModDownloadRow {
    /// **DL-Run 2 — the per-mod progress-bar fill (0.0..=1.0), the WHOLE
    /// byte fraction (no artificial band-clamp).** Read fresh every egui
    /// frame from `per_byte` (the value `OrchestratorApp::
    /// drain_stream_download` keeps current from the parallel
    /// `stream_downloader`'s per-asset byte deltas), so the bar grows
    /// frame-by-frame, decoupled from the 4-wide pool — a row at 0.37 next
    /// to a sibling at 0.81, never a clump snapping 0.20→1.0:
    ///
    ///   - `Downloading` with a determinate size (a live `Content-Length`
    ///     OR a share-code-baked `expected_size`) ⇒ `bytes / size` over
    ///     the **entire** bar, **including 0** before the first byte (an
    ///     empty track that fills as bytes arrive). This is STRICTLY
    ///     monotonic from 0 — no artificial "starting nub" the real first
    ///     chunk could fall *below* (a 64 KiB chunk of a 600 KB archive is
    ///     0.107 — a 0.15 nub would jerk backward to it; the source of a
    ///     subtle reverse-jank). The prior 0.64 band-ceiling is also gone
    ///     (it made a byte-complete row paint ~64% then jerk to full when
    ///     the lifecycle flipped it to `Extracting`). The status-vector
    ///     reclassification (Downloading → Extracting → Staged) advances
    ///     the *row's phase*; the bar simply tracks bytes 0→1.
    ///   - `Downloading` with **no determinate size at all** (no
    ///     `Content-Length`, no baked size) ⇒ the indeterminate active
    ///     fill: `is_indeterminate()` is true and the caller paints a
    ///     moving marquee (honest active-but-unmeasured — SPEC §4.3
    ///     "graceful"; never a fake %, never a frozen bar). The value
    ///     returned here is just the marquee's placeholder.
    ///   - any other case (Queued / Extracting / Staged / Skipped) ⇒
    ///     `status.phase_fraction()` (Extracting/Staged/Skipped are
    ///     post-download so the byte fraction is ≈1.0 anyway — phase is
    ///     the right signal there; a Queued row is 0).
    ///
    /// Monotonic per row by construction: the running byte total is
    /// non-decreasing within download (0 → … → size), and the phase steps
    /// are strictly increasing (Queued 0 < Downloading < Extracting <
    /// Staged/Skipped 1.0), so a row's own bar never regresses.
    #[must_use]
    pub fn bar_fraction(&self) -> f32 {
        if self.status == ModDownloadStatus::Downloading {
            // The determinate size: a live Content-Length, else the
            // share-code-baked expected size.
            let size = self
                .per_byte
                .and_then(|(_, t)| t)
                .filter(|&t| t > 0)
                .or(self.expected_size.filter(|&s| s > 0));
            if let Some(size) = size {
                // True byte fraction over the WHOLE bar, INCLUDING 0
                // before the first byte (strictly monotonic from empty —
                // no nub to jerk back from).
                let got = self.per_byte.map_or(0, |(b, _)| b);
                return (got as f32 / size as f32).clamp(0.0, 1.0);
            }
            // No determinate size anywhere ⇒ indeterminate (the caller
            // paints a moving marquee via `is_indeterminate`).
            return ModDownloadStatus::Downloading.phase_fraction();
        }
        self.status.phase_fraction()
    }

    /// **DL-Run 2 — `true` when this row's download fill is an honest
    /// *indeterminate*** — an in-flight `Downloading` row with **no
    /// determinate size at all**: no live `Content-Length` (`per_byte`
    /// total `None`/0) AND no share-code-baked `expected_size`. The caller
    /// paints a moving marquee instead of a static fill so the user sees
    /// it is genuinely active-but-unmeasured (never a fake %, never a
    /// frozen bar). If EITHER a Content-Length OR a baked size is known
    /// the row is determinate (a real `bytes / size` bar) and this is
    /// `false`.
    #[must_use]
    pub fn is_indeterminate(&self) -> bool {
        if self.status != ModDownloadStatus::Downloading {
            return false;
        }
        let has_content_length = matches!(self.per_byte, Some((_, Some(t))) if t > 0);
        let has_baked_size = matches!(self.expected_size, Some(s) if s > 0);
        !has_content_length && !has_baked_size
    }

    /// **DL-Run 2 — this row's contribution to the Download byte
    /// aggregate** as `(downloaded_bytes, denominator_bytes)`, or `None`
    /// when the row has no determinate size at all (excluded from the
    /// determinate aggregate; the screen then falls back to the count for
    /// just that row — see [`DownloadProgress::download_overall_fraction`]).
    ///
    ///   - `Skipped` (DL-Run 1 already-present) ⇒ `(size, size)` — counts
    ///     instantly-complete (so a mostly-cached install is honest, not
    ///     lurched / sat at a false low).
    ///   - `Extracting` / `Staged` ⇒ `(size, size)` — its archive is fully
    ///     fetched; it is complete for the *download* phase.
    ///   - `Downloading` / `Queued` with a known size ⇒
    ///     `(bytes_so_far, size)` where `size` prefers the baked
    ///     `expected_size`, else this row's live `Content-Length`.
    ///   - no known size (no baked size, no Content-Length yet) ⇒ `None`.
    #[must_use]
    pub fn download_bytes_pair(&self) -> Option<(u64, u64)> {
        // The denominator: the share-code-baked size if present, else the
        // live Content-Length the streamer reported for this row.
        let known_size = self
            .expected_size
            .or_else(|| self.per_byte.and_then(|(_, t)| t).filter(|&t| t > 0));
        match self.status {
            ModDownloadStatus::Skipped
            | ModDownloadStatus::Extracting
            | ModDownloadStatus::Staged => {
                // Download-complete for this mod: full size on both sides.
                known_size.map(|s| (s, s))
            }
            ModDownloadStatus::Downloading | ModDownloadStatus::Queued => {
                let size = known_size?;
                let got = self.per_byte.map_or(0, |(b, _)| b).min(size);
                Some((got, size))
            }
        }
    }
}

/// **DL-Run 2 — the two explicit ordered phases.** Each is its own
/// independent 0→100; the screen NEVER shares one bar between them. The
/// install screen takes over after Extract (the #1 0/0-flash hold logic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallPhase {
    /// Fetching mod archives — the Download byte aggregate is live.
    #[default]
    Downloading,
    /// All archives fetched; unpacking — the Extract count bar is live
    /// (a SEPARATE 0→100 that starts at 0 here, never inheriting Download).
    Extracting,
}

impl InstallPhase {
    /// The phase-indicator verb the chrome shows ("Downloading …" /
    /// "Extracting …").
    pub fn verb(self) -> &'static str {
        match self {
            InstallPhase::Downloading => "Downloading",
            InstallPhase::Extracting => "Extracting",
        }
    }
}

/// **DL-Run 2 — one already-present-by-hash (DL-Run-1-skipped) mod.**
/// Captured the frame `archive_skip::skip_present_archives` drops it from
/// `update_selected_update_assets` (it then vanishes from BIO's resolved
/// set). It is re-injected into the §4.3 grid as an instantly-satisfied
/// row and its `size` counts complete in the Download byte aggregate so a
/// mostly-cached install is honest ("48 of 51 already present").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkippedMod {
    /// Mod display name (the asset `label`).
    pub name: String,
    /// Source label (the asset `source_id`).
    pub source: String,
    /// The archive's known byte size (the share-code-baked
    /// `ArchiveMeta.size`, matched by `archive_file_name`). `None` if the
    /// code carried no size for it (still rendered satisfied; just not a
    /// determinate aggregate contributor).
    pub size: Option<u64>,
}

/// The Stage-3 download/extract progress model. Lives on
/// `InstallScreenState`. Phase 7 P7.T17 feeds it live from BIO's
/// auto-build state via [`DownloadProgress::from_wizard_state`] (the
/// Phase-5 empty-grid chassis is still used by the not-yet-wired
/// fork-download path).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Per-mod rows for the *to-fetch* set, modlist order (one per resolved
    /// asset). DL-Run-1-skipped mods are NOT here (BIO dropped them) — they
    /// are in [`Self::skipped`] and rendered as instant ✓ rows ABOVE these.
    pub rows: Vec<ModDownloadRow>,
    /// **DL-Run 2 — already-present-by-hash mods** (DL-Run 1 dropped them
    /// from BIO's resolved set; captured into `InstallScreenState
    /// ::skipped_mods` and carried here). Rendered as instantly-satisfied
    /// "✓ already downloaded" rows and counted complete in the Download
    /// byte aggregate.
    pub skipped: Vec<SkippedMod>,
    /// **DL-Run 2 — the per-mod expected archive size map**, keyed by
    /// asset index into `step2.update_selected_update_assets` (== `rows`
    /// index). Decoded once from the share code's `{name,size,hash}` and
    /// carried through the per-frame `rows` rebuild (merged onto
    /// `ModDownloadRow::expected_size`). Empty for a fieldless /
    /// pre-redesign / third-party code (the aggregate then uses each row's
    /// live `Content-Length`).
    pub expected_sizes: std::collections::BTreeMap<usize, u64>,
    /// **#1 (P7.T17 / SPEC §4.3) — the persistent per-mod byte map**,
    /// keyed by asset index into `step2.update_selected_update_assets`
    /// (== row index, since `from_wizard_state` builds one row per asset
    /// in that order). `OrchestratorApp::drain_stream_download` writes
    /// here every frame from the parallel `stream_downloader`'s byte
    /// deltas. `from_wizard_state` rebuilds `rows` from BIO state each
    /// frame but **carries this map through unchanged + merges it onto the
    /// rebuilt rows**, so a `Downloading` row shows its real `bytes /
    /// total` fraction even though the row vector itself is reconstructed
    /// per frame. `(bytes, Some(content_length))` ⇒ a real fraction;
    /// `(bytes, None)` ⇒ indeterminate (no `Content-Length`). Empty for
    /// the chassis path (phase-fraction fallback).
    pub asset_bytes: std::collections::BTreeMap<usize, (u64, Option<u64>)>,
    /// **DL Fix-Set v2 (Fix 1c) — the live BIO extract progress snapshot
    /// `(completed, total)`**, sourced every frame from the shared
    /// `Arc<Mutex<Option<(usize, usize)>>>` on `OrchestratorApp::extract_
    /// progress` (written by the forwarder thread `extract_intercept::
    /// start_extract_with_intercept` spawns when the download finishes).
    /// `Some(_)` while extract is in flight ⇒ the Extract bar reads a
    /// real `completed / total` fraction; `None` (pre-extract / post-
    /// Finished / chassis path) ⇒ the bar falls back to the count path
    /// (`extracted_count() / extract_total()`, which doesn't update until
    /// BIO bulk-assigns `update_selected_extracted_sources` at extract-
    /// `Finished` — the defect this fixes). Not persisted across frames;
    /// `render_live` re-snapshots from the handle every frame.
    pub extract_progress: Option<(usize, usize)>,
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
    ///   - `update_selected_download_running` ⇒ `Downloading`. The per-mod
    ///     bar is THIS mod's live `bytes / Content-Length`
    ///     ([`ModDownloadRow::bar_fraction`]) from the byte map merged
    ///     below — the whole 0→1 fill, frame-by-frame, NOT a phase nub.
    ///   - otherwise ⇒ `Queued`.
    ///
    /// The `downloaded` / `extracted` source vectors are
    /// `"<label> -> <path>"` / `"<label> -> <path>"` strings
    /// (`app_step2_update_download.rs:140` / `_extract` `remove_extracted_
    /// update_entries`), so membership is matched by the asset `label`
    /// prefix — the same join BIO itself uses to reconcile them.
    ///
    /// **#1 — `prior_bytes` is the persistent per-mod byte map** (from the
    /// previous frame's `DownloadProgress::asset_bytes`, populated by
    /// `OrchestratorApp::drain_stream_download` from the parallel
    /// `stream_downloader`). It is carried through unchanged and merged
    /// onto the freshly-rebuilt rows so a `Downloading` row shows its real
    /// `bytes / total` fraction even though the row vector is reconstructed
    /// per frame. Pass an empty map for the chassis / first-frame path
    /// (phase-fraction fallback).
    ///
    /// **DL-Run 2 — `prior_skipped` / `prior_expected` are carried
    /// through** so the DL-Run-1 already-present rows + the share-code
    /// byte denominators survive the per-frame rebuild. Pass empties for
    /// the chassis / first-frame path.
    ///
    /// **Fix 1e** — under Fix 1e the asset list KEEPS skipped assets (so
    /// BIO's `build_extract_jobs` finds them). The §4.3 grid renders
    /// skipped mods via the dedicated `skipped` rows path (the "✓
    /// already downloaded" caption), so the to-fetch row list MUST
    /// filter out asset indices that are in the skipped set — otherwise
    /// the same mod would appear twice (once as a Skipped row, once as
    /// a Staged/Extracting/Downloading row). The filter uses the labels
    /// of `prior_skipped` (the skipped-mod labels are unique under the
    /// SkippedMod model — Fix 1e built them from the actual asset
    /// labels). The original asset index `i` is **preserved** for
    /// `prior_bytes` / `prior_expected` lookups (they key on the index
    /// in the unchanged list).
    #[must_use]
    pub fn from_wizard_state_full(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
        prior_skipped: &[SkippedMod],
        prior_expected: &std::collections::BTreeMap<usize, u64>,
    ) -> Self {
        let s2 = &state.step2;

        let label_done = |list: &[String], label: &str| {
            list.iter().any(|e| {
                e.split(" -> ")
                    .next()
                    .map(str::trim)
                    .is_some_and(|l| l == label)
            })
        };
        // **Fix 1e** — the set of skipped-row labels (the names rendered
        // in the dedicated "✓ already downloaded" rows). Asset indices
        // whose label matches are excluded from the to-fetch row list
        // (no double-render). The original asset index is preserved
        // throughout so byte/size lookups still hit.
        let skipped_labels: std::collections::HashSet<&str> =
            prior_skipped.iter().map(|s| s.name.as_str()).collect();

        let rows = s2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                if skipped_labels.contains(a.label.as_str()) {
                    return None; // rendered as a Skipped row, not here
                }
                let downloaded = label_done(&s2.update_selected_downloaded_sources, &a.label);
                let extracted = label_done(&s2.update_selected_extracted_sources, &a.label);
                let status = if extracted {
                    ModDownloadStatus::Staged
                } else if downloaded {
                    // Downloaded; extract pending or running.
                    ModDownloadStatus::Extracting
                } else if s2.update_selected_download_running {
                    // #1 / DL-Run 2 — the per-mod bar uses the REAL byte
                    // fraction from `per_byte` (merged below from the
                    // persistent map) over the whole 0→1 bar.
                    ModDownloadStatus::Downloading
                } else {
                    ModDownloadStatus::Queued
                };
                Some(ModDownloadRow {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    status,
                    // Carry the live byte readout + the share-code expected
                    // size for THIS asset index through the per-frame
                    // rebuild (#1 / DL-Run 2). Indices are stable under
                    // Fix 1e (asset list is never mutated).
                    per_byte: prior_bytes.get(&i).copied(),
                    expected_size: prior_expected.get(&i).copied(),
                })
            })
            .collect();

        Self {
            rows,
            skipped: prior_skipped.to_vec(),
            expected_sizes: prior_expected.clone(),
            asset_bytes: prior_bytes.clone(),
            // The live snapshot is sourced by `render_live` from the
            // orchestrator-owned handle, not from BIO state — leave it
            // `None` here; the caller fills it after the rebuild (see
            // `render_live`).
            extract_progress: None,
        }
    }

    /// Back-compat: rows + the byte map only (no skipped / expected map).
    /// Kept for the existing live call site path; prefer
    /// [`Self::from_wizard_state_full`].
    #[must_use]
    pub fn from_wizard_state_with_bytes(
        state: &WizardState,
        prior_bytes: &std::collections::BTreeMap<usize, (u64, Option<u64>)>,
    ) -> Self {
        Self::from_wizard_state_full(state, prior_bytes, &[], &std::collections::BTreeMap::new())
    }

    /// Back-compat / chassis convenience: the projection with no live byte
    /// map / skipped / expected map (phase-fraction fallback). Used by the
    /// render gate + the not-yet-wired fork-download path.
    #[must_use]
    pub fn from_wizard_state(state: &WizardState) -> Self {
        Self::from_wizard_state_full(
            state,
            &std::collections::BTreeMap::new(),
            &[],
            &std::collections::BTreeMap::new(),
        )
    }

    /// **#1 — record a live per-mod byte delta** (called by
    /// `OrchestratorApp::drain_stream_download` from the parallel
    /// `stream_downloader`'s `AssetProgress`/`AssetDone`). `index` is the
    /// asset index into `step2.update_selected_update_assets` (== row
    /// index). Monotonic by construction (the downloader only ever sends a
    /// non-decreasing running total per asset). Also reflected onto the
    /// matching row if present so the same-frame render sees it.
    pub fn set_asset_bytes(&mut self, index: usize, bytes: u64, total: Option<u64>) {
        self.asset_bytes.insert(index, (bytes, total));
        if let Some(row) = self.rows.get_mut(index) {
            row.per_byte = Some((bytes, total));
        }
    }
}

impl DownloadProgress {
    /// **DL-Run 2 — which of the two ordered phases is live.** `Extracting`
    /// once every to-fetch archive is downloaded (or skipped) — i.e. no row
    /// is still `Downloading`/`Queued` AND there is at least one row/skip
    /// (an extract really is the next phase). Otherwise `Downloading`. The
    /// chassis / empty model is `Downloading` (the default, no-op).
    pub fn phase(&self) -> InstallPhase {
        let any_work = !self.rows.is_empty() || !self.skipped.is_empty();
        let still_fetching = self.rows.iter().any(|r| {
            matches!(
                r.status,
                ModDownloadStatus::Downloading | ModDownloadStatus::Queued
            )
        });
        if any_work && !still_fetching {
            InstallPhase::Extracting
        } else {
            InstallPhase::Downloading
        }
    }

    /// **Total mod count = to-fetch rows + DL-Run-1-skipped.** The "T" in
    /// "N / T mods" (skipped mods ARE part of the modlist; they are just
    /// instantly satisfied).
    pub fn total(&self) -> usize {
        self.rows.len() + self.skipped.len()
    }

    /// **The Download-phase "N"** — mods whose archive is no longer being
    /// fetched (download-complete rows + every skipped mod). Climbs as
    /// archives land; a fully-cached install starts at `skipped == N`.
    pub fn downloaded_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status.download_complete())
            .count()
            + self.skipped.len()
    }

    /// **The Extract-phase "N" — truly-extracted to-fetch rows ONLY.**
    /// DL-Run-1-skipped mods are deliberately **not** counted here: their
    /// archive is a *download-phase* concern (already placed/satisfied),
    /// and they are dropped from BIO's `update_selected_update_assets`, so
    /// BIO's reused-unchanged extract loop never processes them — they are
    /// not part of the extract *work*. Counting them would make the Extract
    /// bar start above 0 at extract-start (it must start at exactly 0 — the
    /// user-directed "a new extracting phase from 0 to 100"). So Extract is
    /// strictly the fraction of *fetched* archives BIO has unpacked.
    pub fn extracted_count(&self) -> usize {
        self.rows.iter().filter(|r| r.status.is_done()).count()
    }

    /// The number of archives the **extract phase** processes — the
    /// to-fetch rows (what BIO actually unpacks). Skipped mods are NOT
    /// extract work (see [`Self::extracted_count`]).
    fn extract_total(&self) -> usize {
        self.rows.len()
    }

    /// Generic "N / T done" for the chrome's phase line — the (N, T) for
    /// the **currently live phase**: Download = (download-complete +
    /// skipped) / (all mods); Extract = the live `(completed, total)` if
    /// the Fix-1c snapshot is present, else the count-fallback. Always
    /// non-decreasing within a run.
    pub fn completed(&self) -> usize {
        match self.phase() {
            InstallPhase::Downloading => self.downloaded_count(),
            InstallPhase::Extracting => self.extract_completed_total().0,
        }
    }

    /// **DL-Run 2 — the Download overall fraction.** Two modes, chosen by
    /// what the share code carries:
    ///
    ///   - **Homogeneous-known-size** (every to-fetch row has a determinate
    ///     size — a baked `expected_size` OR a live `Content-Length`): a
    ///     TRUE byte aggregate `Σ downloaded_bytes ÷ Σ expected_bytes`
    ///     across every to-fetch mod, with DL-Run-1-skipped mods counted
    ///     instantly-complete (their full size on both sides). The smooth
    ///     byte path — every 64 KiB chunk advances the bar.
    ///   - **Any-row-lacks-known-size** (an old / fieldless / pre-redesign
    ///     code that baked no per-archive sizes, AND the active row's
    ///     server sent no `Content-Length`): **pure count** — `(download-
    ///     complete rows + skipped) / (all rows + skipped)`. Skipped mods
    ///     always count instantly complete (numerator + denominator). This
    ///     is the Fix 1a fallback (the old "% of currently-known bytes" bug:
    ///     active-pool sizes dominated the denominator so the bar tracked
    ///     the active batch as the total instead of all mods).
    ///
    /// 0.0..=1.0. Monotonic: byte totals are non-decreasing and a row only
    /// ever moves `fetching → complete`. Returns `0.0` for the empty /
    /// chassis model.
    pub fn download_overall_fraction(&self) -> f32 {
        if self.rows.is_empty() && self.skipped.is_empty() {
            return 0.0;
        }
        // Fix 1a — pure-count fallback when ANY row lacks a known size.
        // The bug: mixing byte-known and count-only rows let active-pool
        // sizes dominate the denominator (the bar tracked "% of currently-
        // known bytes", not "% of all mods' bytes"). When any row is
        // size-unknown, every row is treated as a single count-unit
        // (skipped mods instantly complete on both sides).
        if self.any_row_lacks_known_size() {
            let denom = self.rows.len() + self.skipped.len();
            if denom == 0 {
                return 0.0;
            }
            let downloaded = self
                .rows
                .iter()
                .filter(|r| r.status.download_complete())
                .count()
                + self.skipped.len();
            return (downloaded as f32 / denom as f32).clamp(0.0, 1.0);
        }
        // Homogeneous known-size — TRUE byte aggregate (the smooth path).
        let mut num: f64 = 0.0;
        let mut den: f64 = 0.0;
        for r in &self.rows {
            if let Some((got, size)) = r.download_bytes_pair() {
                num += got as f64;
                den += size as f64;
            }
        }
        // Skipped mods are instantly complete; size is known here (the
        // homogeneous-known-size branch) so it adds to both sums.
        for s in &self.skipped {
            if let Some(sz) = s.size {
                num += sz as f64;
                den += sz as f64;
            }
        }
        if den <= 0.0 {
            return 0.0;
        }
        (num / den).clamp(0.0, 1.0) as f32
    }

    /// **Fix 1a — `true` when ANY to-fetch row has NO known size** (no
    /// baked `expected_size` AND no live positive `Content-Length`). Drives
    /// the [`Self::download_overall_fraction`] pure-count fallback: the old
    /// "% of currently-known bytes" mode let the active pool's sizes
    /// dominate the denominator so the bar tracked the active batch as the
    /// total instead of every mod. Pure count is the honest aggregate when
    /// the share code carries no sizes (a pre-DL-Run-1 / third-party code).
    /// Empty rows ⇒ `false` (no row to lack a size).
    #[must_use]
    pub fn any_row_lacks_known_size(&self) -> bool {
        self.rows.iter().any(|r| {
            let baked = r.expected_size.is_some_and(|s| s > 0);
            let live = r.per_byte.and_then(|(_, t)| t).is_some_and(|t| t > 0);
            !baked && !live
        })
    }

    /// **DL-Run 2 — the Download overall percent** (0..=100), the byte
    /// aggregate rounded. The Download bar reads this; it NEVER carries
    /// extract progress.
    pub fn download_overall_pct(&self) -> u32 {
        (self.download_overall_fraction() * 100.0).round() as u32
    }

    /// **DL-Run 2 + DL Fix-Set v2 (Fix 1c) — the Extract overall fraction.**
    /// A SEPARATE 0→100 that starts at EXACTLY 0 when the extract phase
    /// begins and climbs independently — NEVER inherits the Download value.
    /// Two modes:
    ///
    ///   - **Live snapshot (Fix 1c)**: when [`Self::extract_progress`] is
    ///     `Some((completed, total))` (the forwarder thread the orchestrator
    ///     spawns at extract-kick wrote the latest BIO `Progress` event),
    ///     return `completed / total.max(1)`. This is the real live extract
    ///     fraction — climbs frame-by-frame as BIO unpacks each archive.
    ///   - **Count fallback**: `extracted_rows / extract_total()`. Used
    ///     pre-snapshot, post-`Finished` (the handle is cleared then), the
    ///     chassis path, or anywhere the intercept isn't wired. The count
    ///     path only updates at extract-`Finished` (BIO bulk-assigns
    ///     `update_selected_extracted_sources` then) — the very defect
    ///     Fix 1c addresses for the live path; the fallback is still
    ///     monotonic + correct, just count-granular.
    ///
    /// A fully-cached install (zero to-fetch rows — nothing for BIO to
    /// extract) is `1.0` once in the extract phase (no extract work =
    /// complete, so it auto-advances honestly).
    /// 0.0..=1.0; empty / pre-extract ⇒ 0.0.
    pub fn extract_overall_fraction(&self) -> f32 {
        if self.phase() != InstallPhase::Extracting {
            return 0.0;
        }
        // **Fix 1c — prefer the live forwarder-snapshot when present.**
        if let Some((completed, total)) = self.extract_progress {
            return (completed as f32 / total.max(1) as f32).clamp(0.0, 1.0);
        }
        let to_extract = self.extract_total();
        if to_extract == 0 {
            // Fully-cached: no archive for BIO to extract ⇒ the extract
            // phase is trivially complete (not a false 0 that never moves).
            return 1.0;
        }
        (self.extracted_count() as f32 / to_extract as f32).clamp(0.0, 1.0)
    }

    /// **DL-Run 2 — the Extract overall percent** (0..=100). The Extract
    /// bar reads this; 0 until the extract phase begins.
    pub fn extract_overall_pct(&self) -> u32 {
        (self.extract_overall_fraction() * 100.0).round() as u32
    }

    /// **DL Fix-Set v2 (Fix 1c) — `(completed, total)` for the Extract
    /// phase's N/T display.** When the live snapshot is present (the
    /// forwarder thread snapshotted a BIO `Progress` event), returns
    /// those numbers so the §4.3 chrome's "N / T mods" line tracks the
    /// real mid-extract progress. Otherwise falls back to the
    /// count-based `(extracted_count(), extract_total())` (which only
    /// updates at extract-`Finished` — the count-fallback for the
    /// chassis / pre-snapshot path).
    #[must_use]
    pub fn extract_completed_total(&self) -> (usize, usize) {
        if let Some((completed, total)) = self.extract_progress {
            return (completed, total);
        }
        (self.extracted_count(), self.extract_total())
    }

    /// `true` when there is at least one mod and every to-fetch row is a
    /// fully-extracted terminal (`Staged`) — the production auto-advance
    /// condition (SPEC §4.3: the next stage transitions automatically when
    /// downloads + extracts complete). Skipped mods do not block it (their
    /// content was presented to extract; once BIO unpacks them the rows it
    /// tracks are `Staged`). Empty ⇒ `false` (not started, never complete).
    pub fn all_staged(&self) -> bool {
        if self.rows.is_empty() && self.skipped.is_empty() {
            return false;
        }
        self.rows.iter().all(|r| r.status.is_done())
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

        match auto_build_driver::prepare_install_dirs_and_maybe_import(
            &mut orchestrator.wizard_state,
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
    // **Fix 1e** — `download_started` heuristic is dropped. The
    // `archives_staged` one-shot latch is sufficient (the pre-Fix-1e
    // `!download_running && downloaded_sources.is_empty()` check was
    // there to avoid racing BIO mid-fetch; under Fix 1e the latch is
    // strictly tighter — it fires once on its arm and is never re-armed
    // until `clear_preview()`, regardless of pre-population). Keeping
    // it would have been false-positive under Fix 1e because the skip
    // pass pre-populates `downloaded_sources` — but the latch is THE
    // guard.
    if !orchestrator.install_screen_state.archives_staged
        && orchestrator.install_screen_state.pipeline_armed
        && !destination.is_empty()
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

        // ── Download-Overhaul Run 1 — **the Wabbajack-installer
        //    checksum-then-skip core** (SPEC §13.12a / §13.3 / §13.12 #2).
        //    `archive_store::stage_known_archives` above only reuses an
        //    archive when THIS modlist's per-install lock recorded a hash
        //    (a *prior attempt* of the same modlist). The real bug the user
        //    hit is a **first** install re-downloading archives whose exact
        //    bytes are already in the global Mods-archive folder. This pass
        //    closes it: decode the per-archive `{name,size,hash}` the
        //    exporter baked into the pasted code
        //    (`share_export::decode_archive_meta`), size-pre-filter +
        //    hash-through-the-persistent-cache the on-disk archives, and
        //    DROP every wanted asset whose expected hash is already present
        //    (placing those bytes at BIO's deterministic extract path so
        //    BIO's reused-unchanged extract `.exists()` gate is satisfied)
        //    — so the parallel `stream_downloader`, which fetches exactly
        //    `update_selected_update_assets`, **never re-downloads them**.
        //    Empty (fieldless / pre-redesign / third-party code) ⇒ a pure
        //    no-op (today's always-download — never an error). Composes
        //    with `stage_known_archives`: that runs first (prior-attempt
        //    lock reuse), this then skips anything else already on disk by
        //    content. ONE-SHOT via the same `archives_staged` latch (it is
        //    set above) — `clear_preview` resets it for a re-entry. The
        //    decoded expected set + the pre-skip asset list are cached on
        //    `install_screen_state` so the post-download VERIFY uses the
        //    exact same expected set + can hash exactly what the streamer
        //    could have fetched. Zero BIO source. ──
        let expected =
            crate::registry::share_export::decode_archive_meta(&code).unwrap_or_default();
        // Capture the pre-skip resolved set for the post-download verify
        // (a skipped archive was content-verified present; a fetched one
        // must be verified — both come from this full list).
        // **Fix 1e** — the list is no longer mutated by `skip_present_
        // archives`, so `pre_skip_assets` is equal to the asset list
        // itself; we still capture it for the verify pass (its contract
        // didn't change — it operates on this list).
        orchestrator.install_screen_state.pre_skip_assets = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .clone();
        let skip = crate::install_runtime::archive_skip::skip_present_archives(
            &mut orchestrator.wizard_state,
            &expected,
        );

        // ── **DL Fix-Set v2 (Fix 1e)** — `summary.skipped_assets`
        //    carries the exact assets that were skipped (no diff
        //    needed); read it directly. The orchestrator-side
        //    **skip-index set** is the indices into the unchanged asset
        //    list whose `archive_file_name` matches a skipped asset's
        //    (a multiset-correct match by name + label pair — same-name
        //    different-source can't both be skipped since the hash is
        //    the identity). The §4.3 grid renders the
        //    SkippedMod rows from `skipped_assets` (the streamer never
        //    touches their indices). The expected-size map is keyed by
        //    the asset index in the unchanged list. ──
        let by_name: std::collections::HashMap<&str, &crate::registry::share_export::ArchiveMeta> =
            expected.iter().map(|m| (m.name.as_str(), m)).collect();
        let skipped_mods: Vec<SkippedMod> = skip
            .skipped_assets
            .iter()
            .map(|a| {
                let name = crate::app::app_step2_update_download::archive_file_name(a);
                SkippedMod {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    size: by_name.get(name.as_str()).map(|m| m.size),
                }
            })
            .collect();
        // Build the skip-index set: for each skipped asset (by label +
        // archive_file_name pair), find its index in the unchanged asset
        // list. A label collision is impossible in practice (BIO derives
        // it from the source); for safety we consume occurrences so a
        // duplicate name picks distinct indices.
        let mut skip_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut consumed: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for skipped_asset in &skip.skipped_assets {
            let name = crate::app::app_step2_update_download::archive_file_name(skipped_asset);
            for (i, a) in orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .iter()
                .enumerate()
            {
                if consumed.contains(&i) {
                    continue;
                }
                if a.label == skipped_asset.label
                    && crate::app::app_step2_update_download::archive_file_name(a) == name
                {
                    skip_indices.insert(i);
                    consumed.insert(i);
                    break;
                }
            }
        }
        let expected_sizes: std::collections::BTreeMap<usize, u64> = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let name = crate::app::app_step2_update_download::archive_file_name(a);
                by_name.get(name.as_str()).map(|m| (i, m.size))
            })
            .collect();
        orchestrator.install_screen_state.skipped_mods = skipped_mods;
        orchestrator.install_screen_state.expected_archive_sizes = expected_sizes;
        orchestrator.install_screen_state.skip_indices = skip_indices;

        orchestrator.install_screen_state.expected_archive_meta = expected;
        tracing::info!(
            target = "orchestrator",
            "checksum-then-skip: {} already-present (kept in list for \
             extract, streamer bypasses by index), {} missing (will fetch), \
             {} no-expected-hash, {} candidates hashed ({} persistent-cache \
             hits); DL-Run-2 captured {} skipped-mod rows + {} expected-size \
             denominators + {} skip indices",
            skip.skipped_present,
            skip.missing_on_disk,
            skip.no_expected_hash,
            skip.hashed_candidates,
            skip.cache_hits,
            orchestrator.install_screen_state.skipped_mods.len(),
            orchestrator
                .install_screen_state
                .expected_archive_sizes
                .len(),
            orchestrator.install_screen_state.skip_indices.len(),
        );
    }

    // ── (2b) **#1 — kick the net-new PARALLEL streaming downloader**
    //    (SPEC §4.3 / §13.12a), interposed **between** `stage_known_
    //    archives` (above — already dropped any store-satisfied assets so
    //    they are NOT re-downloaded) and `ingest_downloaded_archives`
    //    (below — content-addresses what the pool just fetched). This
    //    REPLACES BIO's serial download sub-phase.
    //
    //    **Fix 1e — one-shot via `download_phase_started`.** The prior
    //    heuristic (`stream_download_rx.is_none() && downloaded_sources.
    //    is_empty()`) was already-true under Fix 1e: `archive_skip` now
    //    pre-populates `downloaded_sources` with one entry per skipped
    //    archive, so the heuristic would re-kick the streamer every
    //    frame. The latch is set just before the call and cleared by
    //    `clear_preview()` on Cancel→Preview.
    //
    //    The kick ALWAYS calls `start_stream_download` (even when every
    //    asset is in `skip_indices` ⇒ the all-cached case): the
    //    streamer's worker loop bypasses skipped indices silently, and
    //    the coordinator falls through to send an immediate `Finished`
    //    when the index range exhausts ⇒ the orchestrator's drain
    //    then runs `apply_result_state` + Fix-1c's `start_extract_with_
    //    intercept`, and BIO's unchanged extract → rescan →
    //    `start_auto_build_install` carries the pipeline forward
    //    (matching the non-skip path's continuation exactly).
    if orchestrator.install_screen_state.pipeline_armed
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && auto_build_driver::download_gate_open(&orchestrator.wizard_state)
        && !orchestrator.install_screen_state.download_phase_started
    {
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = true;
        // Mark the one-shot BEFORE the call (defensive: if the call
        // panics or otherwise re-enters this frame, we must not double-
        // kick). The latch is cleared by `clear_preview()` on
        // Cancel→Preview so a re-arm re-kicks cleanly.
        orchestrator.install_screen_state.download_phase_started = true;
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        if let Some(rx) = crate::install_runtime::stream_downloader::start_stream_download(
            &mut orchestrator.wizard_state,
            &skip_indices,
        ) {
            orchestrator.stream_download_rx = Some(rx);
            tracing::info!(
                target = "orchestrator",
                "Fix 1e — parallel streaming downloader spawned for {} \
                 asset(s); streamer bypasses {} skipped index/indices",
                orchestrator
                    .wizard_state
                    .step2
                    .update_selected_update_assets
                    .len(),
                skip_indices.len()
            );
        }
    }

    // ── Download-Overhaul Run 1 — **post-download verify + mismatch (the
    //    Wabbajack integrity rule)**, run BEFORE the content-addressed
    //    ingest so a hash-mismatched archive is deleted + recorded failed
    //    and is therefore NEVER content-addressed/stored (so a corrupt
    //    download cannot poison the store / be presented to extract). For
    //    every PRE-SKIP asset with a recorded expected hash, hash the
    //    just-downloaded archive: match ⇒ accept + register in the
    //    persistent cache (so the NEXT install skips it); mismatch ⇒
    //    delete the bad file + append a BIO-shaped failed-source string
    //    (BIO's unchanged `auto_build_blocker_before_install` then stops
    //    the auto-build for it and extract — gated on `.exists()` — never
    //    sees it). One-shot via `archives_verified` (same first-frame-
    //    after-download gate as the ingest; `clear_preview` resets it).
    //    Empty expected set ⇒ a pure no-op (today's behavior). Zero BIO
    //    source. ──
    if !orchestrator.install_screen_state.archives_verified
        && !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        && orchestrator.install_screen_state.download_phase_started
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        orchestrator.install_screen_state.archives_verified = true;
        let expected = orchestrator
            .install_screen_state
            .expected_archive_meta
            .clone();
        // **Fix 1e** — the verify pass must operate only on assets the
        // streamer actually FETCHED (not the ones `archive_skip` already
        // content-verified present + placed). Filter `pre_skip_assets`
        // (which under Fix 1e equals `update_selected_update_assets`) by
        // excluding `skip_indices`: only the genuinely-downloaded assets
        // remain. (The pre-populated skipped entries in
        // `update_selected_downloaded_sources` are content-verified by
        // the skip pass itself — the hash matched on disk before they
        // were placed; re-verifying them here would re-hash for no value.)
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        let pre_skip: Vec<_> = orchestrator
            .install_screen_state
            .pre_skip_assets
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                if skip_indices.contains(&i) {
                    None // already content-verified by the skip pass
                } else {
                    Some(a.clone())
                }
            })
            .collect();
        let v = crate::install_runtime::archive_skip::verify_downloaded_archives(
            &mut orchestrator.wizard_state,
            &expected,
            &pre_skip,
        );
        tracing::info!(
            target = "orchestrator",
            "post-download verify: {} verified, {} hash-mismatched \
             (deleted + recorded failed, NOT installed), {} unverifiable",
            v.verified,
            v.mismatched,
            v.unverifiable
        );
    }

    if !orchestrator.install_screen_state.archives_ingested
        && !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        // **Fix 1e** — gate behind `download_phase_started` (the kick
        // latch). The prior `!downloaded_sources.is_empty()` heuristic
        // is now true the moment `archive_skip` pre-populates skipped
        // entries — before the streamer even runs. We want ingest to
        // run AFTER the streamer finishes (its own latch flips
        // `download_running` false on `Finished`).
        && orchestrator.install_screen_state.download_phase_started
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
    // **#1 / DL-Run 2 — carry the persistent per-mod byte map + the
    // DL-Run-1 skipped-mods + the share-code expected-size denominators
    // through the per-frame rebuild.** `from_wizard_state_full`
    // reconstructs `rows` from BIO state (status) but merges (a) the byte
    // map `OrchestratorApp::drain_stream_download` accumulates so a
    // `Downloading` row shows its real `bytes / Content-Length` fraction
    // over the WHOLE bar, (b) the skipped-mod rows so a cached install is
    // honest, (c) the expected-size map so the Download byte aggregate has
    // a stable denominator. The freshly-built model is stored BACK onto
    // `install_screen_state.download_progress` so all three persist across
    // frames AND survive a `set_asset_bytes` write from the next drain.
    let prior_bytes = orchestrator
        .install_screen_state
        .download_progress
        .asset_bytes
        .clone();
    let prior_skipped = orchestrator.install_screen_state.skipped_mods.clone();
    let prior_expected = orchestrator
        .install_screen_state
        .expected_archive_sizes
        .clone();
    let mut progress = DownloadProgress::from_wizard_state_full(
        &orchestrator.wizard_state,
        &prior_bytes,
        &prior_skipped,
        &prior_expected,
    );
    // **DL Fix-Set v2 (Fix 1c)** — snapshot the live BIO extract
    // `(completed, total)` from the shared handle the forwarder thread
    // writes; the Extract phase bar + the chrome's N/T line both read it
    // for the real mid-extract progress (instead of the count-only
    // post-`Finished` fallback). `None` outside the extract window ⇒ the
    // count fallback (chassis + pre-snapshot path) is used.
    progress.extract_progress = orchestrator.extract_progress.lock().ok().and_then(|g| *g);

    // ── **#1 — eliminate the post-extract 0/0 grid flash.** When BIO's
    //    extract empties `update_selected_update_assets` (its
    //    `remove_extracted_update_entries`) the freshly-built `progress`
    //    has zero rows for a few frames before `pipeline_reached_install`
    //    flips us to stage 4 — that transient rendered as a jarring
    //    "0 / 0 mods · 0%". We own the download→extract→install hand-off,
    //    so once the pipeline has reached extract/install we HOLD the last
    //    non-empty grid (the all-staged/extracting view the user just saw)
    //    instead of painting an empty one. The held grid is the prior
    //    frame's `download_progress` (it had rows); we keep showing it
    //    until `pipeline_reached_install` returns and the caller advances
    //    to the install screen. Never affects the chassis path (it has no
    //    pipeline) nor the active download (rows are non-empty then). ──
    if progress.rows.is_empty()
        && !orchestrator
            .install_screen_state
            .download_progress
            .rows
            .is_empty()
        && orchestrator.install_screen_state.pipeline_armed
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        // Hold the last good (rows-populated) grid through the
        // extract→install seam — no 0/0 flash.
        progress = orchestrator.install_screen_state.download_progress.clone();
    } else {
        // Persist the live model so the byte map survives the next frame's
        // rebuild + the next drain's `set_asset_bytes`.
        orchestrator.install_screen_state.download_progress = progress.clone();
    }

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

    // ── Box label="mod progress" — the per-mod grid ───────────────────────
    // Reserve the footer row UP-FRONT and pass the remaining height as the
    // grid's budget so its internal ScrollArea is bounded and the footer
    // stays on-screen. (The prior code ran an UNBOUNDED ScrollArea and only
    // computed the footer spacer AFTER it — with 50+ rows the scroll area
    // ate all the height and the footer was pushed off the frame.)
    let footer_h = sub_flow_footer::FOOTER_HEIGHT_PX;
    let grid_budget = (ui.available_height() - footer_h - 8.0).max(140.0);
    render_mod_progress(ui, palette, progress, grid_budget);

    // The grid box is now bounded ⇒ this reclaims exactly the footer row.
    let spacer = (ui.available_height() - footer_h).max(0.0);
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

/// `Box label="overall progress"` — **DL-Run 2: the TWO distinct ordered
/// phase bars** (Download then Extract) + a phase indicator + the optional
/// faint hint. Each phase is its own independent 0→100 (the wireframe's
/// single bar is replaced by two clearly-labelled rows — the user-directed
/// "smooth fill 0→100, then a new extracting phase 0→100" requirement;
/// recorded as intentional so a review does not collapse them back to one):
///
///   • **Download** — `N / T mods · P%` where P% = the TRUE byte aggregate
///     `Σ bytes ÷ Σ expected` ([`DownloadProgress::download_overall_pct`]);
///     filled accent; this is the active phase's bar while downloading.
///   • **Extract** — a SEPARATE bar that is **0 until the extract phase
///     begins** then climbs `extracted ÷ total` independently
///     ([`DownloadProgress::extract_overall_pct`]). It NEVER inherits the
///     Download value.
///
/// The currently-live phase is named ("Downloading … " / "Extracting … ")
/// and its bar is accent-filled; the not-yet / done phase bar is shown
/// muted so the user always sees both phases and where they are.
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
        ui.add_space(6.0);

        let phase = progress.phase();
        // Download denominator = ALL mods (skipped count as satisfied);
        // Extract denominator = the to-fetch rows BIO actually unpacks
        // (skipped mods are a download-phase concern, not extract work) —
        // so the Extract bar is a clean 0→100 that starts at exactly 0.
        // **Fix 1c** — when the live snapshot is present, the Extract
        // N/T comes from BIO's `Progress { completed, total }` (the
        // forwarder thread's snapshot) so the chrome reads the real
        // mid-extract progress, not the count-only post-`Finished`
        // bulk-assign.
        let dl_total = progress.total();
        let dl_n = progress.downloaded_count();
        let dl_pct = progress.download_overall_pct();
        let (ex_n, ex_total) = progress.extract_completed_total();
        let ex_pct = progress.extract_overall_pct();

        // The phase indicator: which phase is live + its N/T · P%.
        let (verb, n, t, p) = match phase {
            InstallPhase::Downloading => (InstallPhase::Downloading.verb(), dl_n, dl_total, dl_pct),
            InstallPhase::Extracting => (InstallPhase::Extracting.verb(), ex_n, ex_total, ex_pct),
        };
        ui.label(
            egui::RichText::new(format!("{verb} \u{2026} {n} / {t} mods \u{00B7} {p}%"))
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        // ── Phase bar 1 — Download (the byte aggregate over ALL mods).
        //    Accent-filled while it is the live phase, muted once handed
        //    off (it stays full so the user sees download completed). ──
        phase_bar_row(
            ui,
            palette,
            "download",
            dl_n,
            dl_total,
            dl_pct,
            f64::from(dl_pct) / 100.0,
            phase == InstallPhase::Downloading,
        );
        ui.add_space(8.0);
        // ── Phase bar 2 — Extract (a SEPARATE 0→100 over the to-fetch
        //    rows; 0 until the extract phase begins, never inheriting
        //    Download's value). ──
        phase_bar_row(
            ui,
            palette,
            "extract",
            ex_n,
            ex_total,
            ex_pct,
            f64::from(ex_pct) / 100.0,
            phase == InstallPhase::Extracting,
        );

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

/// **DL-Run 2 — one phase row inside the overall-progress Box**: a fixed
/// `<verb> N/T · P%` caption column + the flex track. `active` ⇒ the bar
/// is accent-filled (this is the live phase); inactive ⇒ a muted fill so
/// the user still sees the not-yet / completed phase and its position
/// (it never shows the *other* phase's value — the value passed is already
/// that phase's own independent 0→100).
#[allow(clippy::too_many_arguments)]
fn phase_bar_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    verb: &str,
    n: usize,
    total: usize,
    pct: u32,
    frac: f64,
    active: bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        // Fixed 180px caption column (mirrors the wireframe `width:180`).
        let (label_rect, _) = ui.allocate_exact_size(egui::vec2(180.0, 18.0), egui::Sense::hover());
        let cap_color = if active {
            redesign_text_primary(palette)
        } else {
            redesign_text_faint(palette)
        };
        ui.painter().text(
            egui::pos2(label_rect.left(), label_rect.center().y),
            egui::Align2::LEFT_CENTER,
            format!("{verb}  {n} / {total} \u{00B7} {pct}%"),
            egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
            cap_color,
        );

        // The flex:1 bar (wireframe height:14, sketchy border, input-bg
        // track). Accent fill when this is the live phase; a muted fill
        // otherwise (Download stays full-but-muted after it hands off so
        // the user sees it completed; Extract is empty-muted until it
        // begins).
        let bar_w = ui.available_width();
        let (track, _) = ui.allocate_exact_size(egui::vec2(bar_w, 14.0), egui::Sense::hover());
        paint_phase_bar(ui, palette, track, frac, active);
    });
}

/// Paint a phase bar: input-bg track + 1.5px border (always), accent fill
/// when `active`, a muted (`text-faint`) fill when inactive — so a
/// completed-but-not-live phase still reads as filled-but-handed-off and a
/// not-yet phase reads as an empty waiting track.
fn paint_phase_bar(
    ui: &egui::Ui,
    palette: ThemePalette,
    track: egui::Rect,
    frac: f64,
    active: bool,
) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    let frac = frac.clamp(0.0, 1.0) as f32;
    if frac > 0.0 {
        let fill = if active {
            redesign_accent(palette)
        } else {
            redesign_text_faint(palette)
        };
        let fill_rect =
            egui::Rect::from_min_size(track.min, egui::vec2(track.width() * frac, track.height()));
        painter.rect_filled(fill_rect, radius, fill);
    }
    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

/// `Box label="mod progress"` — the per-mod grid.
///
/// **Vertically scrollable + footer-safe.** With 50+ mods the list overflows
/// the box; the rows live in a vertical `egui::ScrollArea` whose height is
/// **bounded by `max_h`** (the budget `render_chrome` computes *after*
/// reserving the footer row). This is the fix for the footer being pushed
/// off-frame: the scroll area can no longer grow unbounded to the full
/// 50-row content height.
///
/// **4 columns** (mod / source / status / progress). The per-mod progress
/// bar carries THIS mod's live byte fraction (the whole 0→1 fill) during
/// download — `ModDownloadRow::bar_fraction()` — so each mod advances
/// frame-by-frame on its own schedule, separate from the overall bars.
/// DL-Run-1-skipped mods render FIRST as instantly-satisfied "✓ already
/// downloaded" rows. `progress` fixed 150px, `status` fixed 120px; the two
/// flexible columns split the remainder 1.8 : 1 (wireframe `1.8fr 1fr`),
/// 12px gap.
fn render_mod_progress(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    progress: &DownloadProgress,
    max_h: f32,
) {
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
        // **DL-Run 2 — status column widened 120→170** to fit the longest
        // caption "✓ already downloaded" (the DL-Run-1 skipped-mod cell)
        // without clipping into the progress column (the render-gate
        // caught the overrun). The progress column is trimmed 150→130 to
        // keep the net fixed width near the prior 270 (+30 from flex —
        // negligible even at the 960px floor; this is NOT the
        // Phase-8-deferred mod/source *flex* collision, which is
        // unrelated to the status width).
        let status_w = 170.0;
        let prog_w = 130.0;
        // The remainder (after the two fixed columns + 3 inter-column gaps)
        // splits 1.8 : 1.
        let flex_total = (ui.available_width() - status_w - prog_w - col_gap * 3.0).max(120.0);
        let mod_w = flex_total * (1.8 / 2.8);
        let src_w = flex_total * (1.0 / 2.8);

        // Header row stays fixed above the scrolled body so the columns
        // are always labelled while the rows scroll.
        egui::Grid::new("stage_downloading_mod_grid_header")
            .num_columns(4)
            .spacing(egui::vec2(col_gap, 6.0))
            .min_col_width(0.0)
            .show(ui, |ui| {
                grid_header(ui, palette, "mod", mod_w);
                grid_header(ui, palette, "source", src_w);
                grid_header(ui, palette, "status", status_w);
                grid_header(ui, palette, "progress", prog_w);
                ui.end_row();
            });

        if progress.rows.is_empty() && progress.skipped.is_empty() {
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

        // The rows scroll inside a HEIGHT-BOUNDED area. `max_h` is the
        // budget `render_chrome` reserved (it subtracted the footer row),
        // minus this box's label + header + paddings (~64px). Capping the
        // ScrollArea's height is what keeps the footer on-screen — without
        // it `auto_shrink([_, true])` let the area grow to the full 50-row
        // content height and shoved the footer off the frame.
        let scroll_h = (max_h - 64.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("stage_downloading_mod_scroll")
            .max_height(scroll_h)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("stage_downloading_mod_grid")
                    .num_columns(4)
                    .spacing(egui::vec2(col_gap, 6.0))
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        // DL-Run 2 — DL-Run-1-skipped (already-present-by-
                        // hash) mods render FIRST as instantly-satisfied
                        // rows ("✓ already downloaded", full bar) so a
                        // mostly-cached install reads honestly + the user
                        // can see exactly what was reused.
                        for s in &progress.skipped {
                            render_skipped_row(ui, palette, s, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                        for row in &progress.rows {
                            render_grid_row(ui, palette, row, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                    });
            });
    });
}

/// **DL-Run 2 — one DL-Run-1-skipped (already-present-by-hash) mod row.**
/// Rendered instantly-satisfied: name + source normal, status the
/// success-green `✓ already downloaded` cell, the per-mod bar full accent.
/// (The user's "48 of 51 already present, downloading 3" honesty —
/// skipped mods are visible, not silently dropped.)
fn render_skipped_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    s: &SkippedMod,
    mod_w: f32,
    src_w: f32,
    status_w: f32,
    prog_w: f32,
) {
    sized_label(
        ui,
        mod_w,
        &s.name,
        14.0,
        "poppins_medium",
        redesign_text_primary(palette),
    );
    sized_label(
        ui,
        src_w,
        &s.source,
        13.0,
        "poppins_light",
        redesign_text_faint(palette),
    );
    check_prose_cell(
        ui,
        status_w,
        "already downloaded",
        redesign_success(palette),
    );
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(prog_w, 14.0), egui::Sense::hover());
    paint_bar(ui, palette, bar_rect, 1.0, true);
}

/// One data row of the 4-column grid (mod / source / status / per-mod
/// progress bar). **DL-Run 2:** the bar fill is the row's own live byte
/// fraction (`bar_fraction()` — the WHOLE 0→1 fill while downloading), so
/// it advances frame-by-frame individually per mod, separate from the two
/// overall phase bars. A no-`Content-Length` row paints a moving marquee
/// (honest indeterminate, never a fake % or a frozen bar).
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

    // Column 3 — status: queued → downloading → extracting... → ✓ staged
    // (or ✓ already downloaded for a DL-Run-1 skip). Color: done →
    // success-green, queued → text-faint, else normal text. The done case
    // lays the `✓` glyph (firacode_nerd) before the prose (poppins_medium),
    // mirroring `sub_flow_footer`'s glyph/prose split.
    let status_color = if row.status.is_done() {
        redesign_success(palette)
    } else if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    if row.status.is_done() {
        let prose = if row.status.is_skipped() {
            "already downloaded"
        } else {
            "staged"
        };
        check_prose_cell(ui, status_w, prose, status_color);
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

    // Column 4 — the per-mod progress bar (DL-Run 2 core requirement).
    // fill = THIS mod's `bar_fraction()` — its REAL byte fraction
    // (`bytes / Content-Length`) over the WHOLE 0→1 bar while it streams
    // (read fresh every frame), the phase-fraction fallback otherwise.
    // Advances frame-by-frame individually per mod, separate from the
    // overall phase bars. A no-`Content-Length` in-flight row gets a
    // moving marquee (honest indeterminate). Queued rows: empty track.
    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(prog_w, 14.0), egui::Sense::hover());
    if row.is_indeterminate() {
        paint_indeterminate_bar(ui, palette, bar_rect);
    } else {
        paint_bar(
            ui,
            palette,
            bar_rect,
            f64::from(row.bar_fraction()),
            !row.status.is_queued(),
        );
    }
}

/// A `✓ <prose>` status cell — the `✓` glyph in `firacode_nerd` (U+2713
/// is present, cmap-verified), the prose in `poppins_medium`, both in
/// `color`. Used for both `✓ staged` and the DL-Run-1 `✓ already
/// downloaded` skip cell.
fn check_prose_cell(ui: &mut egui::Ui, w: f32, prose: &str, color: egui::Color32) {
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
        prose,
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

// **DL-Run 2.** `parse_download_aggregate_pct` (BIO's `"Downloading
// updates: N/M"` string parser) is GONE: the Download overall bar is no
// longer an N/M aggregate at all — it is the TRUE byte aggregate
// `Σ bytes ÷ Σ expected` (`DownloadProgress::download_overall_fraction`),
// which never reads `scan_status`. The N/M string only ever jumped 0→~70%
// in one step (the streamer writes it once at start + once at finish — the
// user's reported "0 to 70 in jerky fat steps"); the byte aggregate climbs
// smoothly with every 64 KiB chunk. No per-frame string parse remains.

/// **DL-Run 2 — paint an honest *indeterminate* per-mod bar** (a
/// no-`Content-Length` row, mid-download): the sketchy track + a moving
/// accent marquee block (driven by `ctx` frame time), so the user sees the
/// row is genuinely active-but-unmeasured — never a fake %, never a frozen
/// bar. The caller requests continuous repaints while a phase is live, so
/// the marquee animates frame-by-frame.
fn paint_indeterminate_bar(ui: &egui::Ui, palette: ThemePalette, track: egui::Rect) {
    if !ui.is_rect_visible(track) {
        return;
    }
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));

    // A block ~28% of the track sliding left↔right, ~1.6s period. `i.time`
    // is wall-clock seconds; a triangle wave keeps it bouncing inside the
    // track (never clipped past either edge).
    let t = ui.input(|i| i.time) as f32;
    let block = (track.width() * 0.28).max(8.0);
    let travel = (track.width() - block).max(0.0);
    let phase = (t / 1.6).fract(); // 0..1
    let tri = if phase < 0.5 {
        phase * 2.0
    } else {
        2.0 - phase * 2.0
    }; // 0→1→0
    let x = track.left() + travel * tri;
    let block_rect = egui::Rect::from_min_size(
        egui::pos2(x, track.top()),
        egui::vec2(block, track.height()),
    );
    painter.rect_filled(block_rect, radius, redesign_accent(palette));

    painter.rect_stroke(
        track,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
}

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
    use std::collections::BTreeMap;

    // ───────────────────────── row helpers ─────────────────────────

    fn row(name: &str, status: ModDownloadStatus) -> ModDownloadRow {
        ModDownloadRow {
            name: name.to_string(),
            source: "src".to_string(),
            status,
            // No live byte signal in these status/phase tests ⇒
            // phase-fraction fallback.
            per_byte: None,
            expected_size: None,
        }
    }

    fn row_b(status: ModDownloadStatus, per_byte: Option<(u64, Option<u64>)>) -> ModDownloadRow {
        ModDownloadRow {
            name: "m".to_string(),
            source: "src".to_string(),
            status,
            per_byte,
            expected_size: None,
        }
    }

    fn row_sz(
        status: ModDownloadStatus,
        per_byte: Option<(u64, Option<u64>)>,
        expected_size: Option<u64>,
    ) -> ModDownloadRow {
        ModDownloadRow {
            name: "m".to_string(),
            source: "src".to_string(),
            status,
            per_byte,
            expected_size,
        }
    }

    fn skipped(name: &str, size: Option<u64>) -> SkippedMod {
        SkippedMod {
            name: name.to_string(),
            source: "github".to_string(),
            size,
        }
    }

    // ───────────────────── status captions / tones ─────────────────────

    #[test]
    fn status_text_has_no_fabricated_pct_and_skipped_reads_already_downloaded() {
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(ModDownloadStatus::Downloading.status_text(), "downloading");
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "extracting...");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "staged");
        assert_eq!(
            ModDownloadStatus::Skipped.status_text(),
            "already downloaded"
        );
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert!(
                !s.status_text().contains('%'),
                "no fabricated per-row % in any status caption ({s:?})"
            );
        }
    }

    #[test]
    fn is_done_is_queued_is_skipped_download_complete_are_correct() {
        assert!(ModDownloadStatus::Queued.is_queued());
        assert!(!ModDownloadStatus::Queued.is_done());
        assert!(!ModDownloadStatus::Queued.download_complete());

        assert!(ModDownloadStatus::Staged.is_done());
        assert!(ModDownloadStatus::Staged.download_complete());
        assert!(!ModDownloadStatus::Staged.is_skipped());

        // DL-Run-1 Skipped is a fully-satisfied terminal: done +
        // download-complete + is_skipped.
        assert!(ModDownloadStatus::Skipped.is_done());
        assert!(ModDownloadStatus::Skipped.download_complete());
        assert!(ModDownloadStatus::Skipped.is_skipped());
        assert!(!ModDownloadStatus::Skipped.is_queued());

        assert!(ModDownloadStatus::Extracting.download_complete());
        assert!(!ModDownloadStatus::Extracting.is_done());
        assert!(!ModDownloadStatus::Downloading.download_complete());
    }

    #[test]
    fn phase_fraction_is_strictly_monotonic_with_satisfied_terminals_full() {
        let q = ModDownloadStatus::Queued.phase_fraction();
        let d = ModDownloadStatus::Downloading.phase_fraction();
        let e = ModDownloadStatus::Extracting.phase_fraction();
        let s = ModDownloadStatus::Staged.phase_fraction();
        let k = ModDownloadStatus::Skipped.phase_fraction();
        assert!(q < d && d < e && e < s, "strictly increasing q<d<e<s");
        assert!((q - 0.0).abs() < f32::EPSILON);
        assert!((s - 1.0).abs() < f32::EPSILON);
        assert!(
            (k - 1.0).abs() < f32::EPSILON,
            "Skipped is a fully-satisfied terminal (1.0)"
        );
    }

    // ──────────── DL-Run 2 — per-mod bar = WHOLE byte fraction ────────────

    #[test]
    fn bar_fraction_is_the_whole_byte_fraction_no_band_clamp() {
        // The jank fix: a Downloading row's bar is bytes/total over the
        // ENTIRE 0→1 bar — NOT clamped to a 0.64 band (the old clamp made
        // a byte-complete row paint ~64% then jerk to full).
        let half = row_b(ModDownloadStatus::Downloading, Some((50, Some(100))));
        assert!((half.bar_fraction() - 0.5).abs() < 0.001, "50/100 ⇒ 0.5");

        // Near-complete bytes ⇒ a near-FULL bar (no artificial ceiling).
        let almost = row_b(ModDownloadStatus::Downloading, Some((999, Some(1000))));
        assert!(
            almost.bar_fraction() > 0.98,
            "byte-near-complete ⇒ a near-full bar (no 0.64 band-clamp), got {}",
            almost.bar_fraction()
        );

        // Byte-complete ⇒ exactly full (clamped to 1.0, never > 1).
        let full = row_b(ModDownloadStatus::Downloading, Some((1000, Some(1000))));
        assert!((full.bar_fraction() - 1.0).abs() < f32::EPSILON);
        let over = row_b(ModDownloadStatus::Downloading, Some((1200, Some(1000))));
        assert!((over.bar_fraction() - 1.0).abs() < f32::EPSILON);

        // Monotonic: more bytes ⇒ non-decreasing.
        let a = row_b(ModDownloadStatus::Downloading, Some((10, Some(100)))).bar_fraction();
        let b = row_b(ModDownloadStatus::Downloading, Some((60, Some(100)))).bar_fraction();
        assert!(b >= a);
    }

    #[test]
    fn bar_fraction_no_content_length_is_indeterminate_not_a_fake_pct() {
        let nub = row_b(ModDownloadStatus::Downloading, Some((123_456, None)));
        // Falls back to the nub value (the caller paints a moving marquee).
        assert_eq!(
            nub.bar_fraction(),
            ModDownloadStatus::Downloading.phase_fraction()
        );
        assert!(nub.is_indeterminate(), "no Content-Length ⇒ indeterminate");
        // Zero total is treated the same (no divide-by-zero, no full bar).
        let zero = row_b(ModDownloadStatus::Downloading, Some((10, Some(0))));
        assert_eq!(
            zero.bar_fraction(),
            ModDownloadStatus::Downloading.phase_fraction()
        );
        assert!(zero.is_indeterminate());
        // A determinate row is NOT indeterminate.
        assert!(!row_b(ModDownloadStatus::Downloading, Some((5, Some(10)))).is_indeterminate());
        // Only Downloading rows can be indeterminate.
        assert!(!row_b(ModDownloadStatus::Queued, Some((0, None))).is_indeterminate());
    }

    #[test]
    fn bar_fraction_is_strictly_monotonic_from_zero_no_reverse_nub_jerk() {
        // The runtime-trace finding: a Downloading row with a DETERMINATE
        // size (baked expected_size, here) but NO bytes yet must read
        // EXACTLY 0 (an empty track) — NOT a 0.15 nub the real first
        // chunk could fall *below* (64 KiB of a 600 KB archive = 0.107 <
        // 0.15 ⇒ a backward jerk). The bar must climb 0 → … → 1
        // strictly monotonically.
        let sz = Some(600_000u64);
        // No bytes yet, size known ⇒ exactly 0 (empty), determinate.
        let none_yet = row_sz(ModDownloadStatus::Downloading, None, sz);
        assert_eq!(
            none_yet.bar_fraction(),
            0.0,
            "no bytes yet ⇒ empty bar, NOT a 0.15 nub"
        );
        assert!(
            !none_yet.is_indeterminate(),
            "a baked size ⇒ determinate (a real bytes/size bar), never the marquee"
        );
        // The realistic chunk sequence is strictly increasing from 0 —
        // the first 64 KiB chunk (0.107) does NOT jerk backward from any
        // nub (there is no nub).
        let seq: Vec<f32> = [0u64, 65_536, 131_072, 300_000, 599_999, 600_000]
            .iter()
            .map(|&b| {
                row_sz(ModDownloadStatus::Downloading, Some((b, Some(600_000))), sz).bar_fraction()
            })
            .collect();
        for w in seq.windows(2) {
            assert!(w[1] >= w[0], "strictly monotonic from 0: {seq:?}");
        }
        assert_eq!(seq[0], 0.0);
        assert!((seq[seq.len() - 1] - 1.0).abs() < 1e-6);
        // A live Content-Length ALSO makes it determinate-from-0 even with
        // no baked size.
        let cl_only = row_sz(ModDownloadStatus::Downloading, Some((0, Some(1000))), None);
        assert_eq!(cl_only.bar_fraction(), 0.0);
        assert!(!cl_only.is_indeterminate());
    }

    #[test]
    fn bar_fraction_falls_back_to_phase_when_no_byte_signal() {
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert_eq!(row_b(s, None).bar_fraction(), s.phase_fraction());
        }
        // Post-download phases ignore the byte map (phase is the signal).
        assert_eq!(
            row_b(ModDownloadStatus::Extracting, Some((100, Some(100)))).bar_fraction(),
            ModDownloadStatus::Extracting.phase_fraction()
        );
        assert_eq!(
            row_b(ModDownloadStatus::Staged, Some((100, Some(100)))).bar_fraction(),
            1.0
        );
    }

    // ──────── DL-Run 2 — per-row Download byte-aggregate contribution ────────

    #[test]
    fn download_bytes_pair_uses_baked_size_then_content_length() {
        // Baked expected_size is the preferred denominator.
        let r = row_sz(
            ModDownloadStatus::Downloading,
            Some((30, Some(999))),
            Some(100),
        );
        assert_eq!(r.download_bytes_pair(), Some((30, 100)));
        // No baked size ⇒ the live Content-Length.
        let r2 = row_sz(ModDownloadStatus::Downloading, Some((30, Some(120))), None);
        assert_eq!(r2.download_bytes_pair(), Some((30, 120)));
        // No size anywhere ⇒ None (excluded from the determinate sums).
        let r3 = row_sz(ModDownloadStatus::Downloading, Some((30, None)), None);
        assert_eq!(r3.download_bytes_pair(), None);
        // Download-complete rows count full size on both sides.
        let ex = row_sz(ModDownloadStatus::Extracting, None, Some(500));
        assert_eq!(ex.download_bytes_pair(), Some((500, 500)));
        let st = row_sz(ModDownloadStatus::Staged, None, Some(500));
        assert_eq!(st.download_bytes_pair(), Some((500, 500)));
        let sk = row_sz(ModDownloadStatus::Skipped, None, Some(500));
        assert_eq!(sk.download_bytes_pair(), Some((500, 500)));
        // Got is clamped to size (never > 1.0 contribution).
        let over = row_sz(
            ModDownloadStatus::Downloading,
            Some((700, Some(999))),
            Some(500),
        );
        assert_eq!(over.download_bytes_pair(), Some((500, 500)));
    }

    // ───────────────── DL-Run 2 — the two ordered phases ─────────────────

    #[test]
    fn phase_is_downloading_until_all_fetched_then_extracting() {
        // Any Queued/Downloading row ⇒ Downloading phase.
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Downloading),
            ],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Downloading);
        // All rows download-complete (Extracting/Staged) ⇒ Extracting.
        let p2 = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(p2.phase(), InstallPhase::Extracting);
        // Only skipped mods, none fetching ⇒ already past download.
        let p3 = DownloadProgress {
            skipped: vec![skipped("x", Some(10))],
            ..Default::default()
        };
        assert_eq!(p3.phase(), InstallPhase::Extracting);
        // Empty / chassis ⇒ Downloading (the default, no-op).
        assert_eq!(
            DownloadProgress::default().phase(),
            InstallPhase::Downloading
        );
    }

    #[test]
    fn download_overall_is_a_true_byte_aggregate_not_n_over_m() {
        // 3 rows, sizes 100/100/100. Bytes 100 (done) + 50 + 0.
        // Σbytes=150, Σexpected=300 ⇒ 0.5 (NOT count 1/3≈0.33).
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(100))),
                    Some(100),
                ),
            ],
            ..Default::default()
        };
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.5).abs() < 0.001,
            "Σbytes÷Σexpected = 150/300 = 0.5, got {f}"
        );
        assert_eq!(p.download_overall_pct(), 50);
    }

    #[test]
    fn fix_1a_pure_count_fallback_when_any_row_lacks_known_size() {
        // The defect this fixes: with mixed known/unknown sizes, the
        // pre-fix byte aggregate let the active pool's known sizes dominate
        // the denominator, so the overall bar tracked the active batch as
        // the total. The fix: ANY row lacking a known size ⇒ pure count.
        // Three known-size complete rows + one unknown-size active row +
        // one skipped (no size) ⇒ pure count = (3 + 1) / (4 + 1) = 0.8.
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                row_sz(ModDownloadStatus::Staged, None, Some(1000)),
                // The unknown-size row: no baked size, no Content-Length yet.
                row_sz(ModDownloadStatus::Downloading, Some((100, None)), None),
            ],
            skipped: vec![skipped("c1", None)],
            ..Default::default()
        };
        assert!(p.any_row_lacks_known_size());
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.8).abs() < 1e-4,
            "pure count = (3 complete + 1 skipped) / (4 rows + 1 skipped) = 0.8, got {f}"
        );
    }

    #[test]
    fn fix_1a_homogeneous_known_size_uses_byte_aggregate() {
        // All rows have a known size (baked OR live Content-Length) ⇒ the
        // smooth byte path is unchanged: Σbytes ÷ Σexpected.
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
                // Live Content-Length only — still a known size (no baked).
                row_sz(ModDownloadStatus::Downloading, Some((25, Some(100))), None),
            ],
            ..Default::default()
        };
        assert!(!p.any_row_lacks_known_size(), "every row has a known size");
        let f = p.download_overall_fraction();
        // Σbytes = 100 + 50 + 25 = 175; Σexpected = 300 ⇒ ~0.5833.
        assert!(
            (f - (175.0_f32 / 300.0)).abs() < 1e-4,
            "byte aggregate = 175/300, got {f}"
        );
    }

    #[test]
    fn fix_1a_all_skipped_is_full_in_both_modes() {
        // No to-fetch rows ⇒ no row lacks known size (empty `any`) ⇒ byte
        // path; skipped mods contribute full size on both sides ⇒ 1.0.
        let p = DownloadProgress {
            skipped: vec![skipped("a", Some(10)), skipped("b", Some(20))],
            ..Default::default()
        };
        assert!(!p.any_row_lacks_known_size());
        assert!((p.download_overall_fraction() - 1.0).abs() < 1e-6);

        // Skipped-only with no sizes also reads as full (the empty-rows
        // case still goes through the byte branch; den is 0 ⇒ early 0.0 —
        // but the user-facing display: a fully-cached install with no
        // sizes is handled by the count fallback only if ANY row exists.
        // The empty-rows / sized-skip-only combination is correctly 1.0).
        let only_unknown_skipped = DownloadProgress {
            skipped: vec![skipped("a", None)],
            ..Default::default()
        };
        // No rows ⇒ any_row_lacks_known_size = false (vacuous) ⇒ byte path
        // ⇒ no determinate sums (no rows, skip has no size) ⇒ den 0 ⇒ 0.
        // This is acceptable: the screen anyway shows 100% via the chrome
        // (all_staged + extract = 1.0). The aggregate's "0 of 0 bytes" is
        // honest; the chrome's percentage uses pct (rounded — still 0).
        // We document the case via the assertion below.
        assert_eq!(
            only_unknown_skipped.download_overall_fraction(),
            0.0,
            "no rows + skipped-with-no-size ⇒ no determinate bytes (chrome \
             still flips via all_staged / extract-complete)"
        );
    }

    #[test]
    fn fix_1a_partial_skipped_plus_unknown_to_fetch_counts_skipped_complete() {
        // 2 skipped (cached) + 2 to-fetch unknown-size ⇒ pure count.
        // Skipped count as complete on both sides.
        // numerator = 2 (skipped) + 0 (no row download-complete) = 2;
        // denominator = 2 (rows) + 2 (skipped) = 4 ⇒ 0.5.
        let p = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Downloading, Some((50, None)), None),
                row_sz(ModDownloadStatus::Queued, None, None),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", None)],
            ..Default::default()
        };
        assert!(p.any_row_lacks_known_size());
        let f = p.download_overall_fraction();
        assert!(
            (f - 0.5).abs() < 1e-4,
            "pure count = (0 row-complete + 2 skipped) / (2 + 2) = 0.5, got {f}"
        );
        // Once both to-fetch rows complete ⇒ 4/4 = 1.0.
        let p_done = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, Some((50, None)), None),
                row_sz(ModDownloadStatus::Staged, None, None),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", None)],
            ..Default::default()
        };
        assert!(p_done.any_row_lacks_known_size());
        assert!((p_done.download_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn download_overall_climbs_smoothly_with_bytes_and_is_monotonic() {
        // Two 1000-byte archives streaming; the aggregate climbs every
        // delta (the "0 to 70 in jerky steps" fix — it never jumps).
        let mk = |b0: u64, b1: u64| DownloadProgress {
            rows: vec![
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((b0, Some(1000))),
                    Some(1000),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((b1, Some(1000))),
                    Some(1000),
                ),
            ],
            ..Default::default()
        };
        let f0 = mk(0, 0).download_overall_fraction();
        let f1 = mk(100, 50).download_overall_fraction();
        let f2 = mk(400, 300).download_overall_fraction();
        let f3 = mk(1000, 1000).download_overall_fraction();
        assert!((f0 - 0.0).abs() < 1e-6);
        assert!(f1 > f0 && f2 > f1 && f3 > f2, "strictly climbing");
        assert!((f3 - 1.0).abs() < 1e-6, "byte-complete ⇒ 1.0");
        // Every micro-step is small (smooth) — never a 0→0.7 leap.
        assert!(f1 < 0.10 && f2 < 0.40);
    }

    #[test]
    fn download_overall_counts_skipped_mods_complete_so_cached_install_is_honest() {
        // 48 skipped (each 1000) + 3 to-fetch (each 1000), all 3 still
        // at 0 bytes. A mostly-cached install must read ~94%, NOT a false
        // low / a lurch. Σbytes = 48*1000 ; Σexpected = 51*1000.
        let skipped48: Vec<SkippedMod> = (0..48)
            .map(|i| skipped(&format!("c{i}"), Some(1000)))
            .collect();
        let p = DownloadProgress {
            rows: vec![
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(1000))),
                    Some(1000),
                ),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((0, Some(1000))),
                    Some(1000),
                ),
                row_sz(ModDownloadStatus::Queued, None, Some(1000)),
            ],
            skipped: skipped48,
            ..Default::default()
        };
        let f = p.download_overall_fraction();
        assert!(
            (f - (48.0 / 51.0) as f32).abs() < 0.001,
            "48 of 51 cached ⇒ ~0.941 (honest, not lurched), got {f}"
        );
        assert_eq!(p.total(), 51);
        assert_eq!(p.downloaded_count(), 48, "the 48 skipped are past download");
    }

    #[test]
    fn download_overall_indeterminate_rows_get_a_count_share_so_it_reaches_one() {
        // **Fix 1a** — ANY row lacking a known size flips the aggregate to
        // pure count (the "active-pool sizes dominate" defect). The bar is
        // bounded and reaches 1.0 via count, not the old per-row byte/count
        // hybrid (which under-counted unknown rows as a single unit
        // alongside known rows worth potentially-GB).
        let mk = |complete: bool| DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Staged, None, Some(100)),
                row_sz(
                    if complete {
                        ModDownloadStatus::Staged
                    } else {
                        ModDownloadStatus::Downloading
                    },
                    Some((9999, None)),
                    None,
                ), // indeterminate (no size) ⇒ any-lacks-known-size ⇒ pure count
            ],
            ..Default::default()
        };
        // Pure count: 1 done / 2 rows = 0.5 mid-flight; bounded < 1.
        let mid = mk(false).download_overall_fraction();
        assert!(
            (mid - 0.5).abs() < 1e-6,
            "any-lacks-known-size ⇒ pure count = 1/2 = 0.5, got {mid}"
        );
        // Both complete ⇒ 2/2 = exactly 1.0.
        assert!((mk(true).download_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn extract_overall_is_separate_zero_until_extract_begins_never_inherits_download() {
        // While downloading (a row still Downloading) Extract = 0 even
        // though Download is mid-way — the two NEVER share a value.
        let downloading = DownloadProgress {
            rows: vec![
                row_sz(ModDownloadStatus::Extracting, None, Some(100)),
                row_sz(
                    ModDownloadStatus::Downloading,
                    Some((50, Some(100))),
                    Some(100),
                ),
            ],
            ..Default::default()
        };
        assert!(
            downloading.download_overall_fraction() > 0.0,
            "download is in progress"
        );
        assert_eq!(
            downloading.extract_overall_fraction(),
            0.0,
            "Extract is 0 until the extract phase begins (never inherits Download)"
        );
        assert_eq!(downloading.extract_overall_pct(), 0);

        // Extract phase begun (all fetched), 1 of 2 extracted ⇒ 0.5.
        let extracting = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),     // extracted
                row("b", ModDownloadStatus::Extracting), // not yet
            ],
            ..Default::default()
        };
        assert_eq!(extracting.phase(), InstallPhase::Extracting);
        assert!(
            (extracting.extract_overall_fraction() - 0.5).abs() < 0.001,
            "1 of 2 extracted ⇒ 0.5 (its OWN 0→100, count-granular)"
        );
        // And it climbs to 1.0 when all staged.
        let done = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
            ..Default::default()
        };
        assert!((done.extract_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fix_1c_extract_overall_uses_live_snapshot_when_present() {
        // **DL Fix-Set v2 (Fix 1c).** When the orchestrator's forwarder
        // thread snapshots a BIO `Progress { completed, total }` event
        // into the shared handle, the Extract bar reads the real
        // mid-extract progress (instead of count_fallback which doesn't
        // update until BIO bulk-assigns the extracted vector at
        // `Finished`).
        // Phase = Extracting (all rows downloaded, none extracted) ⇒
        // count fallback = 0/2 = 0.0; snapshot says 3/10 ⇒ the bar reads
        // 0.3.
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        // No snapshot ⇒ count fallback (0/2 = 0).
        assert_eq!(p.extract_overall_pct(), 0);
        // Snapshot live ⇒ uses (completed, total) = (3, 10) = 30%.
        p.extract_progress = Some((3, 10));
        assert_eq!(
            p.extract_overall_pct(),
            30,
            "live snapshot (3/10) ⇒ 30%, not the count fallback (0/2)"
        );
        // The chrome's N/T also reads the snapshot via
        // `extract_completed_total`.
        assert_eq!(p.extract_completed_total(), (3, 10));
        assert_eq!(p.completed(), 3, "chrome N tracks the live snapshot");
        // Snapshot clears ⇒ falls back to count.
        p.extract_progress = None;
        assert_eq!(p.extract_completed_total(), (0, 2));
        assert_eq!(p.extract_overall_pct(), 0);
    }

    #[test]
    fn fix_1c_extract_snapshot_only_drives_bar_during_extract_phase() {
        // **Fix 1c safety:** a stale snapshot from a prior extract must
        // not bleed into the Download phase. While `phase ==
        // Downloading` the Extract bar reads 0 regardless of any
        // snapshot value (the existing "never inherits Download"
        // invariant; tested here for the snapshot path too).
        let p = DownloadProgress {
            rows: vec![row_sz(
                ModDownloadStatus::Downloading,
                Some((50, Some(100))),
                Some(100),
            )],
            extract_progress: Some((7, 10)), // a stale value
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Downloading);
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "Extract is 0 during Download phase, even with a snapshot present"
        );
    }

    #[test]
    fn extract_starts_at_exactly_zero_even_with_skipped_mods() {
        // The runtime-trace finding: with DL-Run-1-skipped mods present,
        // at extract-START (all fetched rows downloaded, none extracted
        // yet) the Extract bar MUST read EXACTLY 0 — skipped mods are a
        // download-phase concern (BIO never extracts them; they're dropped
        // from update_selected_update_assets), so they do NOT pre-fill the
        // extract bar (the user-directed "a NEW extracting phase from 0").
        let p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting), // fetched, not yet extracted
                row("b", ModDownloadStatus::Extracting),
            ],
            skipped: vec![skipped("c1", Some(1000)), skipped("c2", Some(2000))],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "extract MUST start at exactly 0% (skipped mods don't pre-fill it)"
        );
        // Download is fully done (2 fetched + 2 skipped, all complete).
        assert_eq!(p.download_overall_pct(), 100);
        // It then climbs over the to-fetch rows ONLY (1 of 2 extracted).
        let mut p2 = p.clone();
        p2.rows[0].status = ModDownloadStatus::Staged;
        assert!(
            (p2.extract_overall_fraction() - 0.5).abs() < 0.001,
            "1 of 2 to-fetch extracted ⇒ 0.5 (skipped NOT in the extract denominator)"
        );
        p2.rows[1].status = ModDownloadStatus::Staged;
        assert!((p2.extract_overall_fraction() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fully_cached_install_extract_phase_is_complete_not_a_stuck_zero() {
        // Everything cached (all skipped, zero to-fetch rows): nothing for
        // BIO to extract ⇒ the extract phase is trivially complete (1.0),
        // NOT a false 0 that never moves — it auto-advances honestly.
        let p = DownloadProgress {
            skipped: vec![skipped("a", Some(10)), skipped("b", Some(20))],
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting, "nothing to fetch");
        assert_eq!(p.download_overall_pct(), 100, "all cached ⇒ download done");
        assert_eq!(
            p.extract_overall_pct(),
            100,
            "no extract work ⇒ extract complete (not a stuck 0)"
        );
        assert!(p.all_staged(), "fully-cached ⇒ auto-advance");
    }

    #[test]
    fn extract_never_shows_70_at_51_of_51_downloaded() {
        // The user's exact complaint: 51/51 downloaded, in extracting
        // phase, must NOT read a stale 70% — Extract is its own 0→100.
        let p = DownloadProgress {
            rows: (0..51)
                .map(|_| row("m", ModDownloadStatus::Extracting))
                .collect(),
            ..Default::default()
        };
        assert_eq!(p.phase(), InstallPhase::Extracting);
        // Download is fully done (every archive fetched) ⇒ Download = 100%
        // (its own bar), Extract = 0% (nothing unpacked yet) — NOT a
        // conflated 70%.
        assert_eq!(
            p.extract_overall_pct(),
            0,
            "0 extracted ⇒ Extract is 0%, never a conflated 70%"
        );
        // The download phase is honestly complete (count-fallback: every
        // row download-complete with no size ⇒ 51/51).
        assert_eq!(p.downloaded_count(), 51);
    }

    #[test]
    fn completed_tracks_the_live_phase_count() {
        // Downloading phase ⇒ completed == downloaded_count.
        let dl = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Extracting),
                row("b", ModDownloadStatus::Downloading),
            ],
            skipped: vec![skipped("s", Some(1))],
            ..Default::default()
        };
        assert_eq!(dl.phase(), InstallPhase::Downloading);
        assert_eq!(
            dl.completed(),
            dl.downloaded_count(),
            "while downloading the N is the download count"
        );
        assert_eq!(dl.downloaded_count(), 2, "1 Extracting + 1 skipped");
        // Extracting phase ⇒ completed == extracted_count.
        let ex = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Extracting),
            ],
            ..Default::default()
        };
        assert_eq!(ex.phase(), InstallPhase::Extracting);
        assert_eq!(ex.completed(), ex.extracted_count());
    }

    #[test]
    fn total_counts_rows_plus_skipped() {
        let p = DownloadProgress {
            rows: vec![row("a", ModDownloadStatus::Queued)],
            skipped: vec![skipped("s1", Some(1)), skipped("s2", None)],
            ..Default::default()
        };
        assert_eq!(p.total(), 3, "1 to-fetch + 2 skipped");
    }

    #[test]
    fn all_staged_only_when_every_fetch_row_truly_staged() {
        let mut p = DownloadProgress {
            rows: vec![
                row("a", ModDownloadStatus::Staged),
                row("b", ModDownloadStatus::Staged),
            ],
            ..Default::default()
        };
        assert!(p.all_staged());
        p.rows[1].status = ModDownloadStatus::Extracting;
        assert!(
            !p.all_staged(),
            "an Extracting row must NOT auto-advance the stage"
        );
        // Skipped-only (everything cached) is all-staged once the rows
        // (none) are all done — a fully-cached install can auto-advance.
        let cached = DownloadProgress {
            skipped: vec![skipped("s", Some(1))],
            ..Default::default()
        };
        assert!(
            cached.all_staged(),
            "fully-cached (no fetch rows) ⇒ all_staged (auto-advance)"
        );
        // Empty/chassis ⇒ not started, never complete.
        assert!(!DownloadProgress::default().all_staged());
    }

    #[test]
    fn empty_progress_is_zero_and_not_complete() {
        let p = DownloadProgress::default();
        assert_eq!(p.completed(), 0);
        assert_eq!(p.total(), 0);
        assert_eq!(p.download_overall_pct(), 0);
        assert_eq!(p.extract_overall_pct(), 0);
        assert!(!p.all_staged());
    }

    // ───────── #1 / DL-Run 2 — the persistent byte map survives rebuild ─────────

    #[test]
    fn set_asset_bytes_persists_and_survives_per_frame_rebuild() {
        use crate::app::state::Step2UpdateAsset;
        let mut st = WizardState::default();
        let mk = |label: &str, src: &str| Step2UpdateAsset {
            game_tab: "BGEE".into(),
            tp_file: format!("{label}/{label}.TP2"),
            label: label.into(),
            source_id: src.into(),
            tag: "v1".into(),
            asset_name: format!("{label}.zip"),
            asset_url: format!("http://x/{label}"),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![mk("A", "github"), mk("B", "weasel")];
        st.step2.update_selected_download_running = true;

        let mut p = DownloadProgress::from_wizard_state(&st);
        p.set_asset_bytes(0, 512, Some(2048));
        assert_eq!(p.asset_bytes.get(&0), Some(&(512, Some(2048))));
        assert_eq!(p.rows[0].per_byte, Some((512, Some(2048))));

        // Frame 2: rebuild from BIO state carrying the prior map +
        // expected sizes (DL-Run 2 — from_wizard_state_full).
        let mut expected = BTreeMap::new();
        expected.insert(0usize, 2048u64);
        let p2 = DownloadProgress::from_wizard_state_full(
            &st,
            &p.asset_bytes,
            &[skipped("CACHED", Some(4096))],
            &expected,
        );
        assert_eq!(
            p2.rows[0].per_byte,
            Some((512, Some(2048))),
            "the byte map survives the per-frame row rebuild"
        );
        assert_eq!(
            p2.rows[0].expected_size,
            Some(2048),
            "the share-code expected size is merged onto the row"
        );
        assert_eq!(p2.rows[1].per_byte, None, "asset 1 had no byte delta yet");
        assert_eq!(p2.skipped.len(), 1, "skipped mods carried through");
        // The Downloading row renders a real 512/2048 = .25 fraction.
        assert!((p2.rows[0].bar_fraction() - 0.25).abs() < 0.001);
    }

    #[test]
    fn from_wizard_state_full_classifies_lifecycle_and_carries_skipped() {
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
        st.step2.update_selected_downloaded_sources = vec![
            "EET -> C:/a/EET.zip".to_string(),
            "cdtweaks -> C:/a/cdt.zip".to_string(),
        ];
        st.step2.update_selected_extracted_sources = vec!["EET -> C:/m/EET".to_string()];
        st.step2.update_selected_download_running = true;

        let sk = vec![skipped("ALREADY_HERE", Some(7777))];
        let p =
            DownloadProgress::from_wizard_state_full(&st, &BTreeMap::new(), &sk, &BTreeMap::new());
        assert_eq!(p.rows[0].status, ModDownloadStatus::Staged); // downloaded+extracted
        assert_eq!(p.rows[1].status, ModDownloadStatus::Extracting); // downloaded only
        assert_eq!(p.rows[2].status, ModDownloadStatus::Downloading); // running
        assert_eq!(p.rows[3].status, ModDownloadStatus::Downloading);
        assert_eq!(p.skipped.len(), 1);
        assert_eq!(p.total(), 5, "4 rows + 1 skipped");
    }

    // ─────────────────────── copy / outcome ───────────────────────

    #[test]
    fn install_copy_is_spec_4_3_verbatim() {
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
    fn render_outcome_chassis_stay_until_all_staged() {
        // Chassis `render` (no pipeline): Stay while any row not staged,
        // Advance only when all_staged.
        let mut st = WizardState::default();
        let asset = crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".into(),
            tp_file: "A/A.TP2".into(),
            label: "A".into(),
            source_id: "gh".into(),
            tag: "v1".into(),
            asset_name: "A.zip".into(),
            asset_url: "http://x/A".into(),
            installed_source_ref: None,
        };
        st.step2.update_selected_update_assets = vec![asset];
        // Not downloaded ⇒ Queued ⇒ not all_staged.
        let p = DownloadProgress::from_wizard_state(&st);
        assert!(!p.all_staged());
    }

    #[test]
    fn install_phase_verbs() {
        assert_eq!(InstallPhase::Downloading.verb(), "Downloading");
        assert_eq!(InstallPhase::Extracting.verb(), "Extracting");
        assert_eq!(InstallPhase::default(), InstallPhase::Downloading);
    }
}
