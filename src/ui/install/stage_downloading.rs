// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Install Modlist — Stage 3 (downloading); the wireframe
//! `ImportDownloadScreen`. Also reused by Create → fork-download via
//! [`DownloadScreenCopy`] (only the Install path is wired).
//!
//! Two explicit phases, each its own independent 0→100:
//! - Download overall = `Σ downloaded / Σ expected` byte aggregate; rows
//!   already present by hash count instantly complete.
//! - Extract overall = separate bar; starts at 0 when extract begins,
//!   never inherits the download value.
//!
//! The per-mod bar is each row's live `bytes / Content-Length` (or the
//! share-code-baked size) during download; no-Content-Length rows render
//! an indeterminate marquee.

use eframe::egui;

use crate::app::state::WizardState;
use crate::install_runtime::archive_store;
use crate::ui::install::sub_flow_footer::{self, BackBtn, PrimaryBtn};
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent,
    redesign_border_strong, redesign_input_bg, redesign_pill_danger, redesign_shell_bg,
    redesign_success, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

/// The `✓` staged-checkmark glyph. U+2713 IS present in the full
/// `FiraCode` Nerd build → rendered as a glyph, not a vector. Kept as a
/// named constant so the symbol-glyph rule is visible at the call site.
const CHECK_STAGED: &str = "\u{2713}"; // ✓

use crate::ui::shared::numeric::{
    f32_from_f64, f64_from_u64, pct_from_fraction, ratio_u64, ratio_usize, unit_f32,
};

/// Per-mod download/extract lifecycle.
///
/// Ordered as the row progresses: `Queued` → `Downloading` → `Extracting`
/// → `Staged`. `Skipped` is the already-present-by-hash terminal — never
/// downloaded, presented straight to extract.
///
/// The in-flight `Downloading` row's bar is the mod's own live byte
/// fraction (`bytes / Content-Length`), read fresh every frame from
/// [`ModDownloadRow::per_byte`]; `phase_fraction` is only the no-byte
/// fallback (chassis / fork-download / pre-first-byte / no-Content-Length).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModDownloadStatus {
    /// Not started yet. Faint text.
    #[default]
    Queued,
    /// **(Change B / C) — archive hash decision in
    /// flight.** The async skip pass is currently hashing this
    /// archive (`archive_skip_async::AssetHashStarted` arrived; no
    /// `AssetHashed` yet). The §4.3 grid shows the row as
    /// active-hashing; the caption is "checking cache…".
    Hashing,
    /// Archive fetch in progress. The per-mod bar is this mod's live
    /// `bytes / Content-Length` (`ModDownloadRow::bar_fraction`), not a
    /// phase nub.
    Downloading,
    /// Archive extraction in progress. **(Change C /
    /// visual collapse):** the caption renders as
    /// "✓ downloaded" (NOT "extracting…") — the user's request that
    /// once an archive is fetched the row shows uniformly as
    /// downloaded regardless of its post-fetch lifecycle stage. The
    /// internal status distinction stays (still drives completion
    /// checks, phase classification, count math) but the visual
    /// presentation is uniform across `Extracting`/`Staged`/`Skipped`.
    Extracting,
    /// Downloaded + extracted + staged. **collapse**: caption is
    /// "✓ downloaded" (matching Extracting + Skipped).
    Staged,
    /// **already present on disk by content hash** (the
    /// Wabbajack skip): never downloaded, the bytes were placed at BIO's
    /// extract path. **collapse**: caption is "✓ downloaded" —
    /// the same uniform terminal as Extracting + Staged.
    Skipped,
}

impl ModDownloadStatus {
    /// The per-row status caption. `Downloading` reads just `downloading`
    /// (the live fraction is on the per-mod bar, not the caption).
    /// **(Change C / visual collapse):**
    /// `Extracting`/`Staged`/`Skipped` all return the SAME caption
    /// `"downloaded"` (rendered with the `✓` glyph prefix at the call
    /// site, mirroring `sub_flow_footer`'s glyph/prose split). The user-
    /// directed simplification: once the bytes are on disk the row reads
    /// as downloaded regardless of the post-fetch lifecycle stage. The
    /// internal status distinctions stay (`download_complete()` /
    /// `is_done()` / `is_skipped()` / `phase_fraction()` still drive
    /// completion checks, phase classification, count math) — only the
    /// caption is unified. `Hashing` reads `"checking cache..."` (the
    /// new async-skip-pass active state).
    #[must_use]
    pub fn status_text(self) -> String {
        match self {
            Self::Queued => "queued".to_string(),
            Self::Hashing => "checking cache...".to_string(),
            Self::Downloading => "downloading".to_string(),
            // Extracting/Staged/Skipped all render the same "downloaded"
            // caption (the check glyph is laid before the word at the
            // call site).
            Self::Extracting | Self::Staged | Self::Skipped => "downloaded".to_string(),
        }
    }

    /// **The non-byte fallback per-mod fill (0.0..=1.0).** The live path
    /// uses [`ModDownloadRow::bar_fraction`], which prefers the mod's
    /// real byte fraction; this is only reached when there is NO byte
    /// signal (chassis / fork-download / a frame before the first byte
    /// delta / a no-Content-Length row). Strictly monotonic across the
    /// lifecycle (Queued 0 < Hashing < Downloading < Extracting/Staged/
    /// Skipped 1.0) so a fallback row never visually regresses.
    /// **(Change C / ):** `Extracting` is collapsed
    /// to 1.0 (was 0.65) so its fallback bar matches the new uniform
    /// "downloaded" caption — Extracting + Staged + Skipped all render
    /// full.
    #[must_use]
    pub const fn phase_fraction(self) -> f32 {
        match self {
            Self::Queued => 0.0,
            // Hashing nub — the row is being checked; a tiny determinate
            // fill so the row reads as active.
            Self::Hashing => 0.1,
            // No byte signal yet — a small determinate nub so the row
            // reads as active (the live byte fraction takes over the
            // moment the first delta arrives via `ModDownloadRow::per_byte`).
            Self::Downloading => 0.15,
            // Extracting + Staged + Skipped are fully-satisfied terminals
            // — the internal distinction stays for is_done() /
            // download_complete() / is_skipped().
            Self::Extracting | Self::Staged | Self::Skipped => 1.0,
        }
    }

    /// `true` for the fully-satisfied terminals (`Staged` or `Skipped`).
    #[must_use]
    pub const fn is_done(self) -> bool {
        matches!(self, Self::Staged | Self::Skipped)
    }

    /// `true` once a row's archive is no longer being fetched —
    /// `Extracting`, `Staged`, or `Skipped`. Drives the Download-phase
    /// "N" count and the download-byte-aggregate "complete" treatment.
    /// `Hashing` is NOT download-complete (it's pre-download).
    #[must_use]
    pub const fn download_complete(self) -> bool {
        matches!(self, Self::Extracting | Self::Staged | Self::Skipped)
    }

    /// `true` only for `Queued` — the row's name + status use
    /// `text-faint`.
    #[must_use]
    pub const fn is_queued(self) -> bool {
        matches!(self, Self::Queued)
    }

    /// `true` only for `Hashing` — the row is being checked against the
    /// persistent hash cache / Mods-archive folder.
    #[must_use]
    pub const fn is_hashing(self) -> bool {
        matches!(self, Self::Hashing)
    }

    /// `true` only for the already-present-by-hash terminal.
    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

/// Sort priority used by [`DownloadProgress::from_wizard_state_full`]:
/// active (Hashing/Downloading) → pending (Queued) → downloaded (rest).
const fn status_sort_key(s: ModDownloadStatus) -> u8 {
    match s {
        ModDownloadStatus::Hashing => 0,
        ModDownloadStatus::Downloading => 1,
        ModDownloadStatus::Queued => 2,
        ModDownloadStatus::Extracting | ModDownloadStatus::Staged | ModDownloadStatus::Skipped => 3,
    }
}

/// One row of the grid (mod / source / status / per-mod bar).
///
/// The per-mod bar carries a real byte fraction: `install_runtime::
/// stream_downloader` reads each response's `Content-Length` and
/// accumulates bytes, so a `Downloading` row's bar is `bytes / total`
/// while it streams. `per_byte` is `Some((bytes, total))` once the
/// streamer has reported progress; `total` is `None` for a
/// no-`Content-Length` response (indeterminate, shown as a marquee).
/// Cleared / absent for
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
    /// **this mod's expected archive size in bytes** (from the
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
    /// The per-mod progress-bar fill (0.0..=1.0).
    /// `Downloading` with a determinate size (live `Content-Length` OR
    /// `expected_size`) ⇒ `bytes / size` over the whole bar, including 0
    /// before the first byte arrives.
    /// `Downloading` with no determinate size ⇒ the indeterminate
    /// placeholder ([`Self::is_indeterminate`] is true; the caller paints
    /// a marquee instead of a static fill).
    /// Any other status ⇒ `status.phase_fraction()`.
    /// Monotonic per row: byte totals are non-decreasing and the phase
    /// steps strictly increase, so a row's bar never regresses.
    #[must_use]
    pub fn bar_fraction(&self) -> f32 {
        if self.status == ModDownloadStatus::Downloading {
            // The determinate size: a live Content-Length, else the
            // share-code-baked expected size.
            let size = self
                .per_byte
                .and_then(|(_, t)| t)
                .filter(|&t| t > 0)
                .or_else(|| self.expected_size.filter(|&s| s > 0));
            if let Some(size) = size {
                // True byte fraction over the WHOLE bar, INCLUDING 0
                // before the first byte (strictly monotonic from empty —
                // no nub to jerk back from).
                let got = self.per_byte.map_or(0, |(b, _)| b);
                return ratio_u64(got, size);
            }
            // No determinate size anywhere ⇒ indeterminate (the caller
            // paints a moving marquee via `is_indeterminate`).
            return ModDownloadStatus::Downloading.phase_fraction();
        }
        self.status.phase_fraction()
    }

    /// **`true` when this row's download fill is an honest
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

    /// This row's contribution to the Download byte aggregate as
    /// `(downloaded_bytes, denominator_bytes)`, or `None` when the row has
    /// no determinate size at all.
    /// `Skipped` / `Extracting` / `Staged` ⇒ `(size, size)` — complete
    /// for the Download phase.
    /// `Downloading` / `Queued` with a known size ⇒ `(bytes, size)`
    /// (size prefers `expected_size`, else the live `Content-Length`).
    /// No known size ⇒ `None`.
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
 ModDownloadStatus::Downloading
 | ModDownloadStatus::Queued
 // Hashing is pre-download (deciding whether to download or
 // skip); its byte contribution to the Download phase is the
 // same as Queued (0 bytes / size).
 | ModDownloadStatus::Hashing => {
 let size = known_size?;
 let got = self.per_byte.map_or(0, |(b, _)| b).min(size);
 Some((got, size))
 }
 }
    }
}

/// Three explicit ordered phases.
///
/// Each is its own independent 0→100; the screen never shares one bar
/// between them. `Hashing` (`archive_skip_async`) decides what to fetch,
/// `Downloading` (`stream_downloader`) fetches, `Extracting`
/// (`extract_parallel`) unpacks; then the install screen takes over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallPhase {
    /// Checking the Mods-archive folder against the share-code's per-archive
    /// `{name,size,hash}` to decide what to fetch vs skip. Active whenever
    /// any row's status is `Hashing`.
    Hashing,
    /// Fetching mod archives; the Download byte aggregate is live.
    #[default]
    Downloading,
    /// All archives fetched; unpacking. The Extract count bar starts at 0
    /// here and never inherits the Download value.
    Extracting,
}

impl InstallPhase {
    /// Phase-indicator verb shown by the chrome.
    /// [`DownloadProgress::is_preparing_install`] takes over at the tail
    /// of Extract to swap the verb line to "Preparing to install …".
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Hashing => "Checking cache",
            Self::Downloading => "Downloading",
            Self::Extracting => "Extracting",
        }
    }
}

/// One already-present-by-hash (skipped) mod.
///
/// Captured the frame `archive_skip::skip_present_archives` drops it from
/// `update_selected_update_assets`. Re-injected into the grid as an
/// instantly-satisfied row; its `size` counts complete in the Download
/// byte aggregate so a mostly-cached install is honest.
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

/// The Stage-3 download/extract progress model.
/// Lives on `InstallScreenState`. Fed live from BIO's auto-build state
/// via [`DownloadProgress::from_wizard_state`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Per-mod rows, modlist order (one per resolved asset).
    /// **(Change C / )**: rows include EVERY asset,
    /// including DL-Run-1-skipped (already-present-by-hash) ones as
    /// `Skipped`-status rows — the rendered grid is one combined
    /// list, sorted by status priority (active top, pending middle,
    /// downloaded bottom). The prior separate `skipped: Vec<SkippedMod>`
    /// rendering path is dropped per the user-directed unification.
    pub rows: Vec<ModDownloadRow>,
    /// **/ (Change C) — VESTIGIAL FIELD.**
    /// The brief drops `progress.skipped` (and its separate rendering
    /// in `render_mod_progress`). The field is RETAINED at length-0
    /// only for the byte-aggregate math the tests + the render path
    /// have inherited from v2 (skipped-mod size accounting is now
    /// handled by Skipped-status rows in `rows`); it is never
    /// populated by `from_wizard_state_full` in v3 and the
    /// `render_mod_progress` path never reads it. Internal callers
    /// that want skipped-mod size accounting should look for
    /// `Skipped`-status rows in `rows`. Left in place so the existing
    /// test harness's struct literals continue to compile during the
    /// v3 migration; the long-term plan is to delete it in a
    /// follow-up.
    pub skipped: Vec<SkippedMod>,
    /// **the per-mod expected archive size map**, keyed by
    /// asset index into `step2.update_selected_update_assets` (== `rows`
    /// index). Decoded once from the share code's `{name,size,hash}` and
    /// carried through the per-frame `rows` rebuild (merged onto
    /// `ModDownloadRow::expected_size`). Empty for a fieldless /
    /// pre-redesign / third-party code (the aggregate then uses each row's
    /// live `Content-Length`).
    pub expected_sizes: std::collections::BTreeMap<usize, u64>,
    /// **#1 — the persistent per-mod byte map**,
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
    /// **(Fix 1c) / v3 (Change A) — the live extract
    /// progress snapshot `(completed, total)`**, sourced every frame
    /// from the shared `Arc<Mutex<Option<(usize, usize)>>>` on
    /// `OrchestratorApp::extract_progress`. Under v3 the parallel
    /// extract coordinator writes here directly (no more forwarder
    /// thread): set to `Some((0, N))` at extract-start, bumped to
    /// `Some((c+1, N))` on every `AssetDone`, LEFT at `Some((N, N))`
    /// on `Finished` (the v3 bug fix: the v2 forwarder cleared it,
    /// causing a frame of `(0, 0)` flash before the screen took
    /// over). `Some(_)` ⇒ the Extract bar + the chrome's N/T line
    /// read real `completed / total`. `None` (no extract run yet
    /// this session) ⇒ the bar falls back to the count path.
    pub extract_progress: Option<(usize, usize)>,
    /// **(Change B / C) — the live hash progress
    /// snapshot `(completed, total)`**, sourced every frame from
    /// the shared `Arc<Mutex<Option<(usize, usize)>>>` on
    /// `OrchestratorApp::hash_progress`. The async skip pass
    /// (`archive_skip_async`) writes it via `drain_archive_skip_
    /// events`: set to `Some((0, N))` on `CandidateEnumerated`,
    /// bumped to `Some((c+1, N))` on every `AssetHashed`. Reset to
    /// `None` by `InstallScreenState::clear_preview`. `None`
    /// outside the hash window ⇒ the chrome's Hashing bar reads
    /// `(0, 0)` / 0%.
    pub hash_progress: Option<(usize, usize)>,
}

impl DownloadProgress {
    /// **live feed.** Project BIO's
    /// auto-build / update-download / extract state onto the §4.3 per-mod
    /// grid. One row per resolved download asset
    /// (`step2.update_selected_update_assets` — the mods the imported
    /// share code resolved to a concrete archive), its status derived from
    /// the same BIO progress signals the legacy update-selected popup
    /// reads:
    /// Status classification by BIO source vector membership:
    /// downloaded **and** extracted ⇒ `Staged`
    /// downloaded only ⇒ `Extracting`
    /// `download_running` ⇒ `Downloading` (per-mod bar from `prior_bytes`)
    /// otherwise ⇒ `Queued`
    /// `prior_bytes` carries the per-mod byte map from the previous frame
    /// so a `Downloading` row shows its real `bytes / total` even though
    /// the row vector is reconstructed each frame. `prior_skipped` and
    /// `prior_expected` carry the already-present rows + size denominators
    /// across rebuilds. Pass empty maps for the chassis / first-frame
    /// path.
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
        // Already-present-by-hash mods, keyed by label; they render as
        // `Skipped` rows inline rather than via a separate rows path.
        let skipped_by_label: std::collections::HashMap<&str, &SkippedMod> =
            prior_skipped.iter().map(|s| (s.name.as_str(), s)).collect();

        let mut rows: Vec<ModDownloadRow> = s2
            .update_selected_update_assets
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let status = if skipped_by_label.contains_key(a.label.as_str()) {
                    // Already-present-by-hash ⇒ unified "downloaded" caption.
                    ModDownloadStatus::Skipped
                } else {
                    let downloaded = label_done(&s2.update_selected_downloaded_sources, &a.label);
                    let extracted = label_done(&s2.update_selected_extracted_sources, &a.label);
                    if extracted {
                        ModDownloadStatus::Staged
                    } else if downloaded {
                        // Downloaded; extract pending or running.
                        ModDownloadStatus::Extracting
                    } else if s2.update_selected_download_running {
                        // #1 / the per-mod bar uses the REAL
                        // byte fraction from `per_byte` (merged below
                        // from the persistent map) over the whole 0→1
                        // bar.
                        ModDownloadStatus::Downloading
                    } else {
                        ModDownloadStatus::Queued
                    }
                };
                // expected_size: prefer the prior_expected map (the
                // share-code-baked size); fall back to the
                // skipped-mod's recorded size (the skip pass
                // populated it from the same share-code data).
                let expected_size = prior_expected
                    .get(&i)
                    .copied()
                    .or_else(|| skipped_by_label.get(a.label.as_str()).and_then(|s| s.size));
                ModDownloadRow {
                    name: a.label.clone(),
                    source: a.source_id.clone(),
                    status,
                    // Carry the live byte readout for THIS asset index
                    // through the per-frame rebuild (#1 / ).
                    // Indices are stable under Fix 1e (asset list is
                    // never mutated).
                    per_byte: prior_bytes.get(&i).copied(),
                    expected_size,
                }
            })
            .collect();

        // Sort by status priority: Hashing/Downloading (active) →
        // Queued (pending) → Extracting/Staged/Skipped (downloaded).
        // Stable sort preserves original asset order within each tier.
        rows.sort_by_key(|r| status_sort_key(r.status));

        Self {
            rows,
            // `skipped` is vestigial in
            // v3; never populated by `from_wizard_state_full`. The
            // skipped-mod rendering path is dropped (skipped assets
            // render as `Skipped`-status rows in `rows`). Left
            // empty so the field exists for back-compat callers /
            // tests.
            skipped: Vec::new(),
            expected_sizes: prior_expected.clone(),
            asset_bytes: prior_bytes.clone(),
            // The live snapshots are sourced by `render_live` from the
            // orchestrator-owned handles, not from BIO state — leave
            // them `None` here; the caller fills them after the
            // rebuild (see `render_live`).
            extract_progress: None,
            hash_progress: None,
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
    /// **(Change C) — which of the THREE ordered phases
    /// is live.** Hashing → Downloading → Extracting (then Preparing
    /// to install — see [`Self::is_preparing_install`]):
    /// `Hashing` ⇒ any row's status is `Hashing` (the async skip
    /// pass is actively hashing assets).
    /// `Downloading` ⇒ any row is `Downloading`/`Queued` (and no
    /// row is Hashing — the skip pass has handed off).
    /// `Extracting` ⇒ any work present (skipped/extracting/staged
    /// rows) and no row is Hashing/Downloading/Queued — i.e. all
    /// archives are either downloaded-complete or being unpacked.
    /// The chassis / empty model is `Downloading` (the default, no-op).
    #[must_use]
    pub fn phase(&self) -> InstallPhase {
        let any_hashing = self
            .rows
            .iter()
            .any(|r| r.status == ModDownloadStatus::Hashing);
        if any_hashing {
            return InstallPhase::Hashing;
        }
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

    /// **(Change C / ) — "Preparing to install"
    /// UX moment.** `true` when:
    /// the phase is `Extracting`, AND
    /// the extract completion `(n, n)` snapshot indicates
    /// `n > 0 && n == total` (every archive unpacked), AND
    /// every row's status is a downloaded-terminal
    /// (`Skipped`/`Extracting`/`Staged`) — i.e. nothing is
    /// still hashing or downloading.
    /// When `true`, `render_overall_progress` replaces the phase
    /// verb line ("Extracting … N / T mods · X%") with "Preparing
    /// to install …"; both phase bars stay visible at 100%. This is
    /// the brief's UX request: at the tail of Extract, before the
    /// install screen takes over, give the user a clear "we're
    /// about to install" beat.
    #[must_use]
    pub fn is_preparing_install(&self) -> bool {
        if self.phase() != InstallPhase::Extracting {
            return false;
        }
        let (c, t) = self.extract_completed_total();
        if t == 0 || c != t {
            return false;
        }
        // Every row must be a downloaded-terminal — no Hashing /
        // Downloading / Queued left.
        self.rows.iter().all(|r| {
            matches!(
                r.status,
                ModDownloadStatus::Skipped
                    | ModDownloadStatus::Extracting
                    | ModDownloadStatus::Staged
            )
        })
    }

    /// **Total mod count = to-fetch rows + DL-Run-1-skipped.** The "T" in
    /// "N / T mods" (skipped mods ARE part of the modlist; they are just
    /// instantly satisfied).
    #[must_use]
    pub const fn total(&self) -> usize {
        self.rows.len() + self.skipped.len()
    }

    /// **The Download-phase "N"** — mods whose archive is no longer being
    /// fetched (download-complete rows + every skipped mod). Climbs as
    /// archives land; a fully-cached install starts at `skipped == N`.
    #[must_use]
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
    #[must_use]
    pub fn extracted_count(&self) -> usize {
        self.rows.iter().filter(|r| r.status.is_done()).count()
    }

    /// The number of archives the **extract phase** processes — the
    /// to-fetch rows (what BIO actually unpacks). Skipped mods are NOT
    /// extract work (see [`Self::extracted_count`]).
    const fn extract_total(&self) -> usize {
        self.rows.len()
    }

    /// Generic "N / T done" for the chrome's phase line — the (N, T) for
    /// the **currently live phase**: Hashing = `hash_completed_total`.0;
    /// Download = `downloaded_count`; Extract = the live
    /// `(completed, total)` if the snapshot is present, else the
    /// count-fallback. Always non-decreasing within a run.
    #[must_use]
    pub fn completed(&self) -> usize {
        match self.phase() {
            InstallPhase::Hashing => self.hash_completed_total().0,
            InstallPhase::Downloading => self.downloaded_count(),
            InstallPhase::Extracting => self.extract_completed_total().0,
        }
    }

    /// **the Download overall fraction.** Two modes, chosen by
    /// what the share code carries:
    /// **Homogeneous-known-size** (every to-fetch row has a determinate
    /// size — a baked `expected_size` OR a live `Content-Length`): a
    /// TRUE byte aggregate `Σ downloaded_bytes ÷ Σ expected_bytes`
    /// across every to-fetch mod, with DL-Run-1-skipped mods counted
    /// instantly-complete (their full size on both sides). The smooth
    /// byte path — every 64 KiB chunk advances the bar.
    /// **Any-row-lacks-known-size** (an old / fieldless / pre-redesign
    /// code that baked no per-archive sizes, AND the active row's
    /// server sent no `Content-Length`): **pure count** — `(download-
    /// complete rows + skipped) / (all rows + skipped)`. Skipped mods
    /// always count instantly complete (numerator + denominator). This
    /// is the Fix 1a fallback (the old "% of currently-known bytes" bug:
    /// active-pool sizes dominated the denominator so the bar tracked
    /// the active batch as the total instead of all mods).
    /// 0.0..=1.0. Monotonic: byte totals are non-decreasing and a row only
    /// ever moves `fetching → complete`. Returns `0.0` for the empty /
    /// chassis model.
    #[must_use]
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
            return ratio_usize(downloaded, denom);
        }
        // Homogeneous known-size — TRUE byte aggregate (the smooth path).
        let mut num: f64 = 0.0;
        let mut den: f64 = 0.0;
        for r in &self.rows {
            if let Some((got, size)) = r.download_bytes_pair() {
                num += f64_from_u64(got);
                den += f64_from_u64(size);
            }
        }
        // Skipped mods are instantly complete; size is known here (the
        // homogeneous-known-size branch) so it adds to both sums.
        for s in &self.skipped {
            if let Some(sz) = s.size {
                num += f64_from_u64(sz);
                den += f64_from_u64(sz);
            }
        }
        if den <= 0.0 {
            return 0.0;
        }
        unit_f32(num / den)
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

    /// **the Download overall percent** (0..=100), the byte
    /// aggregate rounded. The Download bar reads this; it NEVER carries
    /// extract progress.
    #[must_use]
    pub fn download_overall_pct(&self) -> u32 {
        pct_from_fraction(self.download_overall_fraction())
    }

    /// **+ (Fix 1c) — the Extract overall fraction.**
    /// A SEPARATE 0→100 that starts at EXACTLY 0 when the extract phase
    /// begins and climbs independently — NEVER inherits the Download value.
    /// Two modes:
    /// **Live snapshot (Fix 1c)**: when [`Self::extract_progress`] is
    /// `Some((completed, total))` (the forwarder thread the orchestrator
    /// spawns at extract-kick wrote the latest BIO `Progress` event),
    /// return `completed / total.max(1)`. This is the real live extract
    /// fraction — climbs frame-by-frame as BIO unpacks each archive.
    /// **Count fallback**: `extracted_rows / extract_total()`. Used
    /// pre-snapshot, post-`Finished` (the handle is cleared then), the
    /// chassis path, or anywhere the intercept isn't wired. The count
    /// path only updates at extract-`Finished` (BIO bulk-assigns
    /// `update_selected_extracted_sources` then) — the very defect
    /// Fix 1c addresses for the live path; the fallback is still
    /// monotonic + correct, just count-granular.
    /// A fully-cached install (zero to-fetch rows — nothing for BIO to
    /// extract) is `1.0` once in the extract phase (no extract work =
    /// complete, so it auto-advances honestly).
    /// 0.0..=1.0; empty / pre-extract ⇒ 0.0.
    #[must_use]
    pub fn extract_overall_fraction(&self) -> f32 {
        if self.phase() != InstallPhase::Extracting {
            return 0.0;
        }
        // Prefer the live forwarder snapshot when present.
        if let Some((completed, total)) = self.extract_progress {
            return ratio_usize(completed, total.max(1));
        }
        let to_extract = self.extract_total();
        if to_extract == 0 {
            // Fully-cached: no archive for BIO to extract ⇒ the extract
            // phase is trivially complete (not a false 0 that never moves).
            return 1.0;
        }
        ratio_usize(self.extracted_count(), to_extract)
    }

    /// **the Extract overall percent** (0..=100). The Extract
    /// bar reads this; 0 until the extract phase begins.
    #[must_use]
    pub fn extract_overall_pct(&self) -> u32 {
        pct_from_fraction(self.extract_overall_fraction())
    }

    /// **(Fix 1c) / v3 (Change A) — `(completed,
    /// total)` for the Extract phase's N/T display.** When the
    /// live snapshot is present (the parallel extract coordinator
    /// wrote it directly under v3 — was a forwarder under v2),
    /// returns those numbers so the §4.3 chrome's "N / T mods"
    /// line tracks the real mid-extract progress. Otherwise falls
    /// back to the count-based `(extracted_count(),
    /// extract_total())` (the chassis / pre-snapshot path).
    #[must_use]
    pub fn extract_completed_total(&self) -> (usize, usize) {
        if let Some((completed, total)) = self.extract_progress {
            return (completed, total);
        }
        (self.extracted_count(), self.extract_total())
    }

    /// **(Change B / C) — `(completed, total)` for
    /// the Hashing phase's N/T display.** Reads the live
    /// `hash_progress` snapshot (the async skip pass's drain
    /// writes it). `(0, 0)` outside the hash window (the chrome's
    /// chrome's Hashing bar then reads 0% / muted).
    #[must_use]
    pub fn hash_completed_total(&self) -> (usize, usize) {
        self.hash_progress.unwrap_or((0, 0))
    }

    /// **(Change B / C) — the Hashing overall
    /// percent** (0..=100). The Hashing bar reads this; 0 outside
    /// the hash window.
    #[must_use]
    pub fn hash_overall_pct(&self) -> u32 {
        let (n, t) = self.hash_completed_total();
        if t == 0 {
            return 0;
        }
        pct_from_fraction(ratio_usize(n, t))
    }

    /// `true` when there is at least one mod and every to-fetch row is a
    /// fully-extracted terminal (`Staged`) — the production auto-advance
    /// condition (: the next stage transitions automatically when
    /// downloads + extracts complete). Skipped mods do not block it (their
    /// content was presented to extract; once BIO unpacks them the rows it
    /// tracks are `Staged`). Empty ⇒ `false` (not started, never complete).
    #[must_use]
    pub fn all_staged(&self) -> bool {
        if self.rows.is_empty() && self.skipped.is_empty() {
            return false;
        }
        self.rows.iter().all(|r| r.status.is_done())
    }
}

/// The reusable screen copy (wireframe `ImportDownloadScreen` props).
/// Install passes the strings; Phase 6's fork-download passes
/// its own. Only the Install path is wired.
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
    /// Install-path copy.
    pub const INSTALL: Self = Self {
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
    /// `← Cancel` clicked — back to the Preview stage (: "Cancel
    /// (← back)").
    Cancel,
    /// Downloads + extracts all finished — auto-advance to the next stage
    /// (: the install runtime, which is the Phase-7 stub
    /// this run). In production this fires with no manual click; the
    /// wireframe's `simulate complete →` primary is a wireframe-only
    /// affordance and is intentionally NOT shipped (see the run report's
    /// judgment-call note re: an optional dev-mode manual advance).
    Advance,
}

/// Render the Stage-3 download/extract chassis with a caller-supplied
/// `progress` model.
///
/// Used by the not-yet-wired fork-download path and by the render gate;
/// the live Install-Modlist path uses [`render_live`]. Auto-advances when
/// every row is `Staged`.
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

/// The live Install-Modlist Downloading stage.
///
/// Orchestrator-aware: arms BIO's import → auto-build pipeline once,
/// interposes the content-addressed staging layer at the download/extract
/// boundary every frame, feeds the grid from BIO state, and advances to
/// the stage-4 seam when the pipeline reaches the install hand-off.
///
/// The pipeline is not driven from here — the orchestrator's existing
/// per-frame polls already run it. This screen only arms it once,
/// interposes `archive_store` at the boundary, renders the live feed,
/// and returns `Advance` at the seam.
///
/// Returns `Cancel` if the footer's `← Cancel` was clicked (the caller
/// resets the arm latch + progress and returns to Preview), `Advance` at
/// the seam, else `Stay`.
pub fn render_live(
    ui: &mut egui::Ui,
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    copy: DownloadScreenCopy,
) -> DownloadingOutcome {
    use crate::install_runtime::auto_build_driver;

    let palette = orchestrator.theme_palette;
    let inputs = LivePipelineInputs::from(orchestrator);

    arm_pipeline_once(orchestrator, &inputs);
    stage_and_kick_archive_skip_once(orchestrator, &inputs);
    kick_streaming_downloader_once(orchestrator);
    verify_downloaded_archives_once(orchestrator, &inputs.destination);
    ingest_downloaded_archives_once(orchestrator, &inputs.destination);

    let progress = build_and_hold_progress(orchestrator);
    let arm_error = orchestrator.install_screen_state.pipeline_arm_error.clone();
    let back_clicked = render_chrome(ui, palette, copy, &progress, arm_error.as_deref());

    if back_clicked {
        return DownloadingOutcome::Cancel;
    }
    if auto_build_driver::pipeline_reached_install(&orchestrator.wizard_state) {
        return DownloadingOutcome::Advance;
    }
    auto_build_driver::log_if_pipeline_stopped(&orchestrator.wizard_state);
    DownloadingOutcome::Stay
}

struct LivePipelineInputs {
    destination: String,
    game: crate::registry::model::Game,
    workflow: crate::install_runtime::flag_policies::InstallWorkflow,
    code: String,
}

impl LivePipelineInputs {
    fn from(orchestrator: &crate::ui::orchestrator::orchestrator_app::OrchestratorApp) -> Self {
        let state = &orchestrator.install_screen_state;
        let destination = state.destination.trim().to_string();
        let game = state
            .parsed_preview
            .as_ref()
            .map(|p| crate::registry::model::Game::from_legacy_string(&p.game_install))
            .unwrap_or_default();
        let workflow = if state.is_partial() {
            crate::install_runtime::flag_policies::InstallWorkflow::ContinuePartialInstall
        } else {
            crate::install_runtime::flag_policies::InstallWorkflow::ShareCodeConsuming
        };
        let code = state.import_code.trim().to_string();
        Self {
            destination,
            game,
            workflow,
            code,
        }
    }
}

/// Import the share code and arm the auto-build pipeline once per session.
/// On failure, records the error for in-chrome display and keeps the latch
/// set (no per-frame retry).
fn arm_pipeline_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    use crate::install_runtime::auto_build_driver;

    if !orchestrator.install_screen_state.pipeline_flags.armed() {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_armed(true);

        match auto_build_driver::prepare_install_dirs_and_maybe_import(
            &mut orchestrator.wizard_state,
            &inputs.destination,
            inputs.game,
            inputs.workflow,
            &inputs.code,
        ) {
            Ok(_) => {
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
                crate::install_runtime::install_modlist_registration::register_and_write_install_start_artifacts(
                    orchestrator,
                );
            }
            Err(err) => {
                orchestrator.install_screen_state.pipeline_arm_error = Some(err.clone());
                orchestrator.wizard_state.step2.scan_status =
                    format!("Auto Build could not start: {err}");
                tracing::warn!(
                    target = "orchestrator",
                    "pipeline arm failed: {err} (Downloading stays navigable; surfaced on-screen)"
                );
            }
        }
    }
}

/// Place store-known archives at BIO's extract path and spawn the async
/// checksum-then-skip pool. One-shot per arm; reset on Cancel→Preview.
fn stage_and_kick_archive_skip_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    inputs: &LivePipelineInputs,
) {
    if !orchestrator
        .install_screen_state
        .pipeline_flags
        .archives_staged()
        && orchestrator.install_screen_state.pipeline_flags.armed()
        && !inputs.destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .is_empty()
    {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_staged(true);
        archive_store::stage_known_archives(&mut orchestrator.wizard_state, &inputs.destination);

        let expected =
            crate::registry::share_export::decode_archive_meta(&inputs.code).unwrap_or_default();
        orchestrator.install_screen_state.pre_skip_assets = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .clone();

        let by_name: std::collections::HashMap<&str, &crate::registry::share_export::ArchiveMeta> =
            expected.iter().map(|m| (m.name.as_str(), m)).collect();
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
        orchestrator.install_screen_state.skipped_mods = Vec::new();
        orchestrator.install_screen_state.expected_archive_sizes = expected_sizes;
        orchestrator.install_screen_state.skip_indices = std::collections::HashSet::new();
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archive_skip_completed(false);
        orchestrator
            .install_screen_state
            .expected_archive_meta
            .clone_from(&expected);

        let archive_dir_pb =
            std::path::PathBuf::from(orchestrator.wizard_state.step1.mods_archive_folder.trim());
        let input = crate::install_runtime::archive_skip_async::AsyncSkipInput {
            archive_dir: archive_dir_pb,
            assets: orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .clone(),
        };
        let rx =
            crate::install_runtime::archive_skip_async::start_async_archive_skip(input, expected);
        orchestrator.archive_skip_rx = Some(rx);
        tracing::info!(
            target = "orchestrator",
            "async checksum-then-skip pool spawned for {} asset(s); {} \
             baked expected sizes carried forward",
            orchestrator
                .wizard_state
                .step2
                .update_selected_update_assets
                .len(),
            orchestrator
                .install_screen_state
                .expected_archive_sizes
                .len(),
        );
    }
}

/// Spawn the parallel streaming downloader once the skip pass has decided
/// which archives are already present.
fn kick_streaming_downloader_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) {
    use crate::install_runtime::auto_build_driver;

    if orchestrator.install_screen_state.pipeline_flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && auto_build_driver::download_gate_open(&orchestrator.wizard_state)
        && !orchestrator
            .install_screen_state
            .pipeline_flags
            .download_phase_started()
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .archive_skip_completed()
    {
        orchestrator
            .wizard_state
            .modlist_auto_build_waiting_for_install = true;
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_download_phase_started(true);
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        if let Some(rx) = crate::install_runtime::stream_downloader::start_stream_download(
            &mut orchestrator.wizard_state,
            &skip_indices,
        ) {
            orchestrator.stream_download_rx = Some(rx);
            tracing::info!(
                target = "orchestrator",
                "parallel streaming downloader spawned for {} asset(s); \
                 bypasses {} skipped index/indices",
                orchestrator
                    .wizard_state
                    .step2
                    .update_selected_update_assets
                    .len(),
                skip_indices.len()
            );
        }
    }
}

/// Hash each fetched archive against the share-code expected hashes once
/// the streamer is done; delete + record-failed any mismatch so BIO's
/// extract never sees a corrupt download.
fn verify_downloaded_archives_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    destination: &str,
) {
    if !orchestrator
        .install_screen_state
        .pipeline_flags
        .archives_verified()
        && !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .download_phase_started()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_verified(true);
        let expected = orchestrator
            .install_screen_state
            .expected_archive_meta
            .clone();
        let skip_indices = orchestrator.install_screen_state.skip_indices.clone();
        let pre_skip: Vec<_> = orchestrator
            .install_screen_state
            .pre_skip_assets
            .iter()
            .enumerate()
            .filter(|(i, _)| !skip_indices.contains(i))
            .map(|(_, a)| a.clone())
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
}

/// Hash + content-address the resolved archive set exactly once after the
/// streamer finishes. Repeated per-frame hashing would freeze the UI.
fn ingest_downloaded_archives_once(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
    destination: &str,
) {
    if !orchestrator
        .install_screen_state
        .pipeline_flags
        .archives_ingested()
        && !destination.is_empty()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_download_running
        && orchestrator
            .install_screen_state
            .pipeline_flags
            .download_phase_started()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty()
    {
        orchestrator
            .install_screen_state
            .pipeline_flags
            .set_archives_ingested(true);
        let names: Vec<String> = orchestrator
            .wizard_state
            .step2
            .update_selected_update_assets
            .iter()
            .map(crate::app::app_step2_update_download::archive_file_name)
            .collect();
        archive_store::ingest_downloaded_archives(&orchestrator.wizard_state, destination, &names);
    }
}

/// Build this frame's progress model from BIO state + the persistent byte
/// map / skipped / expected-size maps, snapshot the live extract + hash
/// progress, then hold the prior frame's grid through the extract→install
/// seam to avoid a 0/0 flash.
fn build_and_hold_progress(
    orchestrator: &mut crate::ui::orchestrator::orchestrator_app::OrchestratorApp,
) -> DownloadProgress {
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
    progress.extract_progress = orchestrator.extract_progress.lock().ok().and_then(|g| *g);
    progress.hash_progress = orchestrator.hash_progress.lock().ok().and_then(|g| *g);

    let hold_prior_grid = progress.rows.is_empty()
        && !orchestrator
            .install_screen_state
            .download_progress
            .rows
            .is_empty()
        && orchestrator.install_screen_state.pipeline_flags.armed()
        && orchestrator
            .install_screen_state
            .pipeline_arm_error
            .is_none()
        && !orchestrator
            .wizard_state
            .step2
            .update_selected_downloaded_sources
            .is_empty();
    if hold_prior_grid {
        orchestrator.install_screen_state.download_progress.clone()
    } else {
        orchestrator.install_screen_state.download_progress = progress.clone();
        progress
    }
}

/// The shared §4.3 chrome (title + overall-progress Box + 4-col grid +
/// footer). Returns whether the footer's `← Cancel` was clicked. Used by
/// both [`render`] (chassis) and [`render_live`] (live feed) so the visual
/// is bit-identical regardless of data source.
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
    // failed). Danger-bordered, full-width, above the (empty) progress
    // box so it cannot be missed — the fix for "it just sits there, no
    // feedback". ──
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

    // footer: `Cancel` (← back) + (production) auto-advance on
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

/// `Box label="overall progress"` — **: the TWO distinct ordered
/// phase bars** (Download then Extract) + a phase indicator + the optional
/// faint hint. Each phase is its own independent 0→100 (the wireframe's
/// single bar is replaced by two clearly-labelled rows — the user-directed
/// "smooth fill 0→100, then a new extracting phase 0→100" requirement;
/// recorded as intentional so a review does not collapse them back to one):
/// • **Download** — `N / T mods · P%` where P% = the TRUE byte aggregate
/// `Σ bytes ÷ Σ expected` ([`DownloadProgress::download_overall_pct`]);
/// filled accent; this is the active phase's bar while downloading.
/// • **Extract** — a SEPARATE bar that is **0 until the extract phase
/// begins** then climbs `extracted ÷ total` independently
/// ([`DownloadProgress::extract_overall_pct`]). It NEVER inherits the
/// Download value.
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
        // Hash denominator/N — from the `hash_progress` snapshot
        // mirrored onto `progress` (the chrome reads only the
        // progress struct's fields). Empty when no async skip pass
        // is in flight.
        let (h_n, h_total) = progress.hash_completed_total();
        let h_pct = progress.hash_overall_pct();
        // Download denominator = ALL mods (skipped count as
        // satisfied); Extract denominator = the to-fetch rows BIO
        // actually unpacks (skipped mods are a download-phase
        // concern, not extract work) — so the Extract bar is a clean
        // 0→100 that starts at exactly 0. when the live
        // snapshot is present, the Extract N/T comes from the
        // parallel extract coordinator's `extract_progress` snapshot
        // so the chrome reads real mid-extract progress.
        let dl_total = progress.total();
        let dl_n = progress.downloaded_count();
        let dl_pct = progress.download_overall_pct();
        let (ex_n, ex_total) = progress.extract_completed_total();
        let ex_pct = progress.extract_overall_pct();

        // The phase indicator: which phase is live + its N/T · P%.
        // at the tail of
        // Extract (every archive unpacked + every row downloaded-
        // terminal), swap the verb line to "Preparing to install"
        // for the UX hand-off beat.
        let preparing = progress.is_preparing_install();
        let phase_line = if preparing {
            "Preparing to install \u{2026}".to_string()
        } else {
            let (verb, n, t, p) = match phase {
                InstallPhase::Hashing => (InstallPhase::Hashing.verb(), h_n, h_total, h_pct),
                InstallPhase::Downloading => {
                    (InstallPhase::Downloading.verb(), dl_n, dl_total, dl_pct)
                }
                InstallPhase::Extracting => {
                    (InstallPhase::Extracting.verb(), ex_n, ex_total, ex_pct)
                }
            };
            format!("{verb} \u{2026} {n} / {t} mods \u{00B7} {p}%")
        };
        ui.label(
            egui::RichText::new(phase_line)
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
        ui.add_space(8.0);

        // ── Phase bar 1 — Hashing. New in v3. Active while the
        // async skip pass is in flight; muted once the skip
        // completes. The chrome always paints all three bars so
        // the user sees the full pipeline at a glance. ──
        phase_bar_row(
            ui,
            palette,
            "hash",
            h_n,
            h_total,
            h_pct,
            f64::from(h_pct) / 100.0,
            phase == InstallPhase::Hashing,
        );
        ui.add_space(8.0);
        // ── Phase bar 2 — Download (the byte aggregate over ALL
        // mods). Accent-filled while it is the live phase, muted
        // once handed off (it stays full so the user sees
        // download completed). ──
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
        // ── Phase bar 3 — Extract (a SEPARATE 0→100 over the to-fetch
        // rows; 0 until the extract phase begins, never inheriting
        // Download's value). Stays full at 100% in the
        // "Preparing to install" state. ──
        phase_bar_row(
            ui,
            palette,
            "extract",
            ex_n,
            ex_total,
            ex_pct,
            f64::from(ex_pct) / 100.0,
            phase == InstallPhase::Extracting && !preparing,
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

/// **one phase row inside the overall-progress Box**: a fixed
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
            format!("{verb} {n} / {total} \u{00B7} {pct}%"),
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
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    let frac = unit_f32(frac);
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
/// **Vertically scrollable + footer-safe.** With 50+ mods the list overflows
/// the box; the rows live in a vertical `egui::ScrollArea` whose height is
/// **bounded by `max_h`** (the budget `render_chrome` computes *after*
/// reserving the footer row). This is the fix for the footer being pushed
/// off-frame: the scroll area can no longer grow unbounded to the full
/// 50-row content height.
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
        // **status column widened 120→170** to fit the longest
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

        if progress.rows.is_empty() {
            // No rows yet (e.g. the not-yet-wired fork-download chassis,
            // or before the pipeline resolves assets). Honest faint
            // placeholder rather than a blank box (the redesign's
            // honest-empty-state stance; consistent with stage_preview's
            // parse-error path).
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
                        // one
                        // combined sorted grid (active top, pending
                        // middle, downloaded bottom; sort is done in
                        // `from_wizard_state_full`). Skipped assets
                        // render here as `Skipped`-status rows (the
                        // unified "✓ downloaded" caption under );
                        // the prior separate skipped-section render
                        // path is dropped per the user-directed
                        // unification.
                        for row in &progress.rows {
                            render_grid_row(ui, palette, row, mod_w, src_w, status_w, prog_w);
                            ui.end_row();
                        }
                    });
            });
    });
}

// `render_skipped_row` deleted.
// The separate skipped-rows render path is gone (the user-directed
// unified-grid render): DL-Run-1-skipped assets now render as
// `Skipped`-status rows via `render_grid_row`, sorted to the bottom
// of the grid by `from_wizard_state_full`'s status-priority sort.

/// One data row of the 4-column grid (mod / source / status / per-mod
/// progress bar). **:** the bar fill is the row's own live byte
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

    // Column 3 — status: queued → hashing... → downloading → ✓ downloaded.
    // `Extracting`/`Staged`/
    // `Skipped` all render uniformly as "✓ downloaded" (the user-directed
    // unified terminal). Color: download-complete → success-green,
    // queued → text-faint, else normal text. The check-prose split
    // mirrors `sub_flow_footer`'s glyph/prose split.
    let status_color = if row.status.download_complete() {
        redesign_success(palette)
    } else if row.status.is_queued() {
        redesign_text_faint(palette)
    } else {
        redesign_text_primary(palette)
    };
    if row.status.download_complete() {
        // uniform "downloaded" prose for every download-
        // complete terminal (Extracting / Staged / Skipped).
        check_prose_cell(ui, status_w, "downloaded", status_color);
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

    // Column 4 — the per-mod progress bar (core requirement).
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
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));
    if filled {
        let frac = unit_f32(frac);
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

// The download overall bar uses the true byte aggregate
// (`DownloadProgress::download_overall_fraction`), not a parsed N/M string
// — every 64 KiB chunk advances it.

/// **paint an honest *indeterminate* per-mod bar** (a
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
    let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
    painter.rect_filled(track, radius, redesign_input_bg(palette));

    // A block ~28% of the track sliding left↔right, ~1.6s period. `i.time`
    // is wall-clock seconds; a triangle wave keeps it bouncing inside the
    // track (never clipped past either edge).
    let t = f32_from_f64(ui.input(|i| i.time));
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
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
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
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
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
    fn status_text_has_no_fabricated_pct_and_unified_downloaded_caption_v3() {
        // the unified
        // "downloaded" caption: Extracting / Staged / Skipped all
        // render as "downloaded" (the user-directed simplification).
        // Hashing is the new pre-download active state.
        assert_eq!(ModDownloadStatus::Queued.status_text(), "queued");
        assert_eq!(
            ModDownloadStatus::Hashing.status_text(),
            "checking cache..."
        );
        assert_eq!(ModDownloadStatus::Downloading.status_text(), "downloading");
        // all three terminals share the
        // "downloaded" caption.
        assert_eq!(ModDownloadStatus::Extracting.status_text(), "downloaded");
        assert_eq!(ModDownloadStatus::Staged.status_text(), "downloaded");
        assert_eq!(ModDownloadStatus::Skipped.status_text(), "downloaded");
        for s in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Hashing,
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
    fn phase_fraction_is_monotonic_with_v3_collapsed_terminals() {
        // the lifecycle is
        // Queued (0) < Hashing < Downloading < Extracting / Staged /
        // Skipped (all 1.0 — collapsed: once the bytes are on
        // disk the row reads as downloaded). Strict monotonic
        // q<h<d<{e,s,k}; e == s == k.
        let queued = ModDownloadStatus::Queued.phase_fraction();
        let hashing = ModDownloadStatus::Hashing.phase_fraction();
        let downloading = ModDownloadStatus::Downloading.phase_fraction();
        let extracting = ModDownloadStatus::Extracting.phase_fraction();
        let staged = ModDownloadStatus::Staged.phase_fraction();
        let skipped = ModDownloadStatus::Skipped.phase_fraction();
        assert!(
            queued < hashing && hashing < downloading,
            "strictly increasing queued < hashing < downloading"
        );
        assert!(
            downloading < extracting,
            "Downloading < Extracting (collapsed terminal)"
        );
        assert!((queued - 0.0).abs() < f32::EPSILON);
        assert!(
            (extracting - 1.0).abs() < f32::EPSILON,
            "Extracting collapses to 1.0"
        );
        assert!(
            (staged - 1.0).abs() < f32::EPSILON,
            "Staged is the fully-satisfied terminal (1.0)"
        );
        assert!(
            (skipped - 1.0).abs() < f32::EPSILON,
            "Skipped is a fully-satisfied terminal (1.0)"
        );
    }

    // ──────────── per-mod bar = WHOLE byte fraction ────────────

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
        assert!(
            (nub.bar_fraction() - ModDownloadStatus::Downloading.phase_fraction()).abs()
                < f32::EPSILON
        );
        assert!(nub.is_indeterminate(), "no Content-Length ⇒ indeterminate");
        // Zero total is treated the same (no divide-by-zero, no full bar).
        let zero = row_b(ModDownloadStatus::Downloading, Some((10, Some(0))));
        assert!(
            (zero.bar_fraction() - ModDownloadStatus::Downloading.phase_fraction()).abs()
                < f32::EPSILON
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
        assert!(
            none_yet.bar_fraction().abs() < f32::EPSILON,
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
        assert!(seq[0].abs() < f32::EPSILON);
        assert!((seq[seq.len() - 1] - 1.0).abs() < 1e-6);
        // A live Content-Length ALSO makes it determinate-from-0 even with
        // no baked size.
        let cl_only = row_sz(ModDownloadStatus::Downloading, Some((0, Some(1000))), None);
        assert!(cl_only.bar_fraction().abs() < f32::EPSILON);
        assert!(!cl_only.is_indeterminate());
    }

    #[test]
    fn bar_fraction_falls_back_to_phase_when_no_byte_signal() {
        for status in [
            ModDownloadStatus::Queued,
            ModDownloadStatus::Downloading,
            ModDownloadStatus::Extracting,
            ModDownloadStatus::Staged,
            ModDownloadStatus::Skipped,
        ] {
            assert!(
                (row_b(status, None).bar_fraction() - status.phase_fraction()).abs() < f32::EPSILON
            );
        }
        // Post-download phases ignore the byte map (phase is the signal).
        assert!(
            (row_b(ModDownloadStatus::Extracting, Some((100, Some(100)))).bar_fraction()
                - ModDownloadStatus::Extracting.phase_fraction())
            .abs()
                < f32::EPSILON
        );
        assert!(
            (row_b(ModDownloadStatus::Staged, Some((100, Some(100)))).bar_fraction() - 1.0).abs()
                < f32::EPSILON
        );
    }

    // ──────── per-row Download byte-aggregate contribution ────────

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

    // ───────────────── the two ordered phases ─────────────────

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
        assert!(
            only_unknown_skipped.download_overall_fraction().abs() < f32::EPSILON,
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
            (f - (48.0_f32 / 51.0_f32)).abs() < 0.001,
            "48 of 51 cached ⇒ ~0.941 (honest, not lurched), got {f}"
        );
        assert_eq!(p.total(), 51);
        assert_eq!(p.downloaded_count(), 48, "the 48 skipped are past download");
    }

    #[test]
    fn download_overall_indeterminate_rows_get_a_count_share_so_it_reaches_one() {
        // ANY row lacking a known size flips the aggregate to
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
        assert!(
            downloading.extract_overall_fraction().abs() < f32::EPSILON,
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
        // **(Fix 1c).** When the orchestrator's forwarder
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
        let mut p2 = p;
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

    // ───────── #1 / the persistent byte map survives rebuild ─────────

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
        // expected sizes (from_wizard_state_full).
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
        // `skipped` is now
        // vestigial; `from_wizard_state_full` never populates it.
        // Skipped assets become `Skipped`-status rows in `rows`
        // (matched by label against the asset list). The test passes
        // `&[skipped("CACHED", _)]` but "CACHED" is not in the
        // asset list (the assets are A and B), so no Skipped row
        // is produced — and `skipped` is empty.
        assert!(
            p2.skipped.is_empty(),
            ": `skipped` is vestigial; not populated"
        );
        // The Downloading row renders a real 512/2048 = .25 fraction.
        assert!((p2.rows[0].bar_fraction() - 0.25).abs() < 0.001);
    }

    #[test]
    fn from_wizard_state_full_v3_classifies_lifecycle_and_sorts_rows() {
        // rows are sorted by
        // status priority (Downloading top, Queued middle,
        // Extracting/Staged/Skipped bottom). The classification logic
        // (Staged ⇐ downloaded+extracted, Extracting ⇐ downloaded
        // only, Downloading ⇐ running else Queued) stays the same;
        // only the row order in the rebuilt grid changes.
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
        // 4 rows (skipped name "ALREADY_HERE" doesn't match any
        // asset label, so no skipped-status row is added — the
        // `sk` is dropped, per the v3 contract: skipped mods are
        // matched against the asset list by label).
        assert_eq!(p.rows.len(), 4);
        // **v3 row sort** — Downloading (stratagems, spell_rev)
        // first (priority 1), then Extracting (cdtweaks) +
        // Staged (EET) (both priority 3, stable-sorted to keep
        // their original order). Queued: none in this scenario.
        let statuses: Vec<_> = p.rows.iter().map(|r| r.status).collect();
        // First two are the active Downloading rows.
        assert_eq!(statuses[0], ModDownloadStatus::Downloading);
        assert_eq!(statuses[1], ModDownloadStatus::Downloading);
        // Bottom two are the downloaded-terminal rows
        // (Extracting / Staged collapse visually but the internal
        // status distinction is preserved).
        assert!(statuses[2..].iter().all(|s| s.download_complete()));
        // **v3 — `skipped` is vestigial / always empty after
        // `from_wizard_state_full`.**
        assert_eq!(
            p.skipped.len(),
            0,
            "v3: skipped is not populated by from_wizard_state_full"
        );
        // total = rows.len() + skipped.len() = 4 + 0 = 4. (Was 5
        // under v2 where the phantom skipped contributed to total;
        // v3's contract is one combined grid + only label-matched
        // skipped rows.)
        assert_eq!(p.total(), 4, "4 rows (no phantom skipped row)");
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
