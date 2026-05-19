// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::archive_skip` — **the Wabbajack-installer
// checksum-then-skip core** (Download-Overhaul Run 1; SPEC §13.12a / §13.3 /
// §13.12 #2).
//
// **The bug this fixes.** On a real install the redesign re-downloaded
// every archive even when the exact bytes were already in the global
// Mods-archive folder. Source-traced: BIO's `download_one_asset`
// unconditionally GETs+overwrites; `Step2UpdateAsset` carries no hash/size;
// the only pre-existing "skip" is BIO's extract-side
// `archive_dir.join(archive_file_name(asset)).exists()` which extracts
// *whatever file has that name, unverified*. SPEC §13.12a *specifies*
// content-addressing (hash-on-write, skip-if-present, same-name/different-
// hash coexist) — it was never effectively wired. `archive_store` added the
// content-addressed store but its `stage_known_archives` only reuses an
// archive when THIS modlist's per-install lock already recorded a hash
// (a *prior attempt* of the same modlist) — it never skips an archive that
// is *already on disk under the same content* on a first install.
//
// **The Wabbajack model the user approved.** The author/exporter (who has
// the files) hashes each archive and bakes `{size, hash}` into the shipped
// share code (`registry::share_export` — the
// [`build_archive_meta_for_assets`] / `pack_meta` / `bake_archive_meta_into
// _code` side). The installer (this module):
//
//   1. **size pre-filter** — enumerate the global Mods-archive folder; for
//      each on-disk file, only hash it if its byte length matches some
//      wanted archive's recorded size (a cheap reject — a multi-GB archive
//      whose size differs is never hashed).
//   2. **hash through a persistent cache** — keyed by `lower(path)` +
//      mtime; a multi-GB archive hashes **once ever** (a later run with an
//      unchanged file is a pure cache hit; the entry is invalidated the
//      moment the file's mtime changes).
//   3. **build `HashMap<expected_hash → on-disk PathBuf>`**.
//   4. **skip** — for every wanted asset whose recorded `expected_hash` is
//      already present on disk: drop it from
//      `update_selected_update_assets` (so the parallel `stream_downloader`
//      — which downloads exactly that list — **never fetches it**) AND
//      present those bytes at BIO's deterministic
//      `archive_dir.join(archive_file_name(asset))` path (hardlink; copy
//      fallback) so BIO's **reused-unchanged** extract picks it up via its
//      `.exists()` gate.
//
// An asset with **no** recorded expected hash (a fieldless / pre-redesign /
// third-party code, or one the exporter could not hash) is **left in the
// list** — the established always-download fallback, never an error.
//
// **Post-download verify (the Wabbajack mismatch rule).** After the
// streamer writes an archive, if an expected hash is known,
// [`verify_downloaded_archives`] hashes the file: `actual != expected` ⇒
// **delete the bad file + record the asset failed** (it is NOT cached, NOT
// presented to extract); only a **matching** hash is registered in the
// persistent cache. A hash-mismatched archive is never silently accepted.
//
// **Content-addressed identity (coexist).** Identity is the content hash,
// not the BIO name — two share codes wanting the same BIO-named archive at
// **different** hashes both resolve correctly: the skip only fires for the
// hash THIS code expects; a different on-disk hash under the same name is
// not a match, so that archive is downloaded (and `archive_store` then
// stores it under its hash-encoded name so both versions coexist).
//
// **Zero BIO source.** This module only: mutates the orchestrator-owned
// `state.step2.update_selected_update_assets`; reads/writes its own JSON
// cache sidecar in the global Mods-archive folder; reuses BIO's
// `app_step2_update_download::archive_file_name` **read-only** (the SAME fn
// `archive_store` / `stream_downloader` already reuse); and reuses
// `archive_store::hash_file` (the ONE stable seedless FNV-1a-128 — one
// hashing path, zero drift; the brief's premise-check outcome). No
// `src/core/` file is touched.
//
// SPEC: §13.12a (content-addressed: hash-on-write, skip-if-present,
//       coexist + the BIO-name link for extract), §13.3 (the share code
//       carries per-archive `{size,hash}`), §13.12 #2, §1 (CRITICAL
//       DIRECTIVE — net-new, zero BIO source).

// rationale: the cache is a simple serde map; `#[must_use]` on trivial
// accessors + `Self`/`const fn` churn add noise without behavior value
// (Cat 3). The hash-cache + skip pass is intrinsic to the Wabbajack model
// (not a smell).
#![allow(
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::too_long_first_doc_paragraph
)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::{Step2UpdateAsset, WizardState};
use crate::install_runtime::archive_store::{self, ARCHIVE_INDEX_FILENAME, INSTALL_LOCK_FILENAME};
use crate::registry::share_export::ArchiveMeta;

/// The persistent **path+mtime → content-hash** cache sidecar filename
/// (written inside the global Mods-archive folder, next to
/// `archive_store`'s index/lock). Lets a multi-GB archive be hashed **once
/// ever**: a later install whose on-disk file is unchanged is a pure cache
/// hit (no re-hash); the entry is invalidated the instant the file's size
/// **or** mtime changes (a re-download / replacement is re-hashed).
pub const HASH_CACHE_FILENAME: &str = ".bio-archive-hashcache.json";

/// One cached file's content-hash, pinned to the exact bytes that produced
/// it via `(size, mtime)`. A change to either invalidates the entry (the
/// hash is recomputed) — the Wabbajack installer's persistent-hash-cache
/// invalidation rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HashCacheEntry {
    /// File byte length at hash time (also the cheap size pre-filter key).
    size: u64,
    /// File mtime (unix nanos) at hash time. `None` if the platform/file
    /// did not expose one — then only `size` guards the entry (still
    /// correct: a same-size different-content replacement is rare and the
    /// post-download verify is the hard backstop; a redundant re-hash is
    /// the worst case).
    mtime_nanos: Option<u128>,
    /// The content hash (`archive_store::hash_file` — FNV-1a-128, 32 hex).
    hash: String,
}

/// The persistent hash cache — `lower(absolute-ish path) → entry`. Keyed by
/// the **lowercased path string** (Windows is case-insensitive; lowercasing
/// makes the key stable regardless of how the path was spelled). A
/// `BTreeMap` keeps the on-disk JSON deterministic/diffable. Missing /
/// unreadable / corrupt ⇒ an empty cache (worst case: one redundant
/// re-hash — never a wrong skip; the post-download verify is the hard
/// integrity backstop regardless).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HashCache {
    entries: BTreeMap<String, HashCacheEntry>,
}

impl HashCache {
    fn path(archive_dir: &Path) -> PathBuf {
        archive_dir.join(HASH_CACHE_FILENAME)
    }

    /// Load the cache from the global Mods-archive folder. A
    /// missing/unreadable/corrupt file degrades to empty (the worst case is
    /// a one-time redundant re-hash; never a wrong skip — same lighter
    /// policy `archive_store`'s reconstructable sidecars use).
    pub fn load(archive_dir: &Path) -> Self {
        match std::fs::read_to_string(Self::path(archive_dir)) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
                warn!(
                    target = "orchestrator",
                    "archive hash cache unreadable ({err}); continuing empty \
                     (worst case: a one-time re-hash)"
                );
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Persist atomically (temp-file-then-rename — the same discipline as
    /// `archive_store`'s sidecars / `RegistryStore::save`). A write failure
    /// is logged, not fatal — the archives themselves are on disk; only the
    /// re-hash accelerator is stale.
    pub fn save(&self, archive_dir: &Path) {
        let p = Self::path(archive_dir);
        let tmp = p.with_extension("json.tmp");
        let json = match serde_json::to_string_pretty(self) {
            Ok(j) => j,
            Err(err) => {
                warn!(target = "orchestrator", "serialize hash cache: {err}");
                return;
            }
        };
        if let Err(err) = std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &p)) {
            warn!(
                target = "orchestrator",
                "persist hash cache {}: {err} (non-fatal — re-hash accelerator only)",
                p.display()
            );
        }
    }

    /// `true` if nothing is cached (no path→hash entries). Observational —
    /// used by the runtime trace to assert a hash-mismatched download never
    /// poisons the cache, and harmless in production.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Lowercased path-string cache key (Windows case-insensitivity safe).
    fn key(path: &Path) -> String {
        path.to_string_lossy().to_lowercase()
    }

    /// The cached hash for `path` **iff** the cache entry's `(size, mtime)`
    /// still matches the file's current `(size, mtime)` — i.e. the bytes
    /// have not changed since it was hashed. A size **or** mtime difference
    /// ⇒ `None` (stale ⇒ the caller re-hashes). This is the cache-hit /
    /// invalidation rule (a multi-GB archive hashes once ever; a changed
    /// file is re-hashed).
    fn fresh_hash(&self, path: &Path, size: u64, mtime_nanos: Option<u128>) -> Option<&str> {
        let e = self.entries.get(&Self::key(path))?;
        if e.size != size {
            return None;
        }
        // If both sides have an mtime, it must match. If either side lacks
        // one, fall back to size-only (documented on `mtime_nanos`).
        if let (Some(a), Some(b)) = (e.mtime_nanos, mtime_nanos) {
            if a != b {
                return None;
            }
        }
        Some(&e.hash)
    }

    /// Record `path`'s content `hash`, pinned to its current `(size,
    /// mtime)`. Overwrites any stale entry for the same path. Returns
    /// `true` if this changed the cache (so the caller can skip a redundant
    /// save).
    fn record(&mut self, path: &Path, size: u64, mtime_nanos: Option<u128>, hash: &str) -> bool {
        let key = Self::key(path);
        let new = HashCacheEntry {
            size,
            mtime_nanos,
            hash: hash.to_string(),
        };
        match self.entries.get(&key) {
            Some(existing) if *existing == new => false,
            _ => {
                self.entries.insert(key, new);
                true
            }
        }
    }
}

/// File `(size, mtime_nanos)` — the size pre-filter input + the cache
/// invalidation key. `None` mtime if unavailable (rare; size-only then).
fn file_size_mtime(path: &Path) -> Option<(u64, Option<u128>)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos());
    Some((size, mtime))
}

/// Outcome of [`skip_present_archives`] — counts only, purely
/// observational (the live Downloading screen / the run report / the
/// runtime trace want to know how much the checksum-then-skip saved).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkipSummary {
    /// Wanted archives whose expected `{size,hash}` was already present on
    /// disk ⇒ **dropped from the download list** (NOT fetched) + presented
    /// at BIO's extract path. The Wabbajack "already have it" count.
    pub skipped_present: usize,
    /// Wanted archives with **no** recorded expected hash (fieldless /
    /// pre-redesign / un-hashable code) ⇒ left for a normal download
    /// (today's always-download fallback).
    pub no_expected_hash: usize,
    /// Wanted archives whose expected hash was **not** found on disk ⇒ left
    /// for a normal download (the genuine fetch set).
    pub missing_on_disk: usize,
    /// On-disk files whose size matched a wanted size and so were hashed
    /// this pass (the size pre-filter let everything else through
    /// un-hashed — the metric proving the cheap reject works).
    pub hashed_candidates: usize,
    /// Of `hashed_candidates`, how many were a **persistent-cache hit** (no
    /// re-hash — proves a multi-GB archive hashes once ever).
    pub cache_hits: usize,
}

/// **Before the streamer downloads: checksum-then-skip (the Wabbajack
/// installer pass).** `expected` is the per-archive `{name,size,hash}`
/// decoded from the share code (`share_export::decode_archive_meta`); empty
/// ⇒ "no expected hashes known" ⇒ a pure no-op (today's always-download for
/// everything — the backward-compatible fallback).
///
/// Steps (see the module header): size pre-filter → hash size-matching
/// on-disk candidates through the persistent path+mtime cache → build
/// `HashMap<hash → PathBuf>` → for every wanted asset whose expected hash
/// is present, drop it from `state.step2.update_selected_update_assets`
/// (so the parallel `stream_downloader` never fetches it) and present those
/// bytes at BIO's deterministic `archive_dir.join(archive_file_name(asset))`
/// path (hardlink; copy fallback) so BIO's reused-unchanged extract
/// `.exists()` gate is satisfied.
///
/// Returns a [`SkipSummary`] (observational). Zero BIO edit: only the
/// orchestrator-owned asset list + the archive dir + the cache sidecar are
/// touched; `archive_file_name` is reused read-only.
pub fn skip_present_archives(state: &mut WizardState, expected: &[ArchiveMeta]) -> SkipSummary {
    let mut summary = SkipSummary::default();
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return summary;
    }
    if expected.is_empty() {
        // No expected hashes known (fieldless / pre-redesign / third-party
        // code) ⇒ today's behavior: download everything. Pure no-op.
        summary.no_expected_hash = state.step2.update_selected_update_assets.len();
        return summary;
    }

    // `expected_hash` → set of expected sizes for that hash (sizes are the
    // cheap pre-filter; the hash is the identity). `name → expected hash`
    // is what we match each resolved asset against.
    let by_name: HashMap<&str, &ArchiveMeta> =
        expected.iter().map(|m| (m.name.as_str(), m)).collect();
    let wanted_sizes: std::collections::HashSet<u64> = expected.iter().map(|m| m.size).collect();
    let wanted_hashes: std::collections::HashSet<&str> =
        expected.iter().map(|m| m.hash.as_str()).collect();

    // ── Build `present_by_hash: HashMap<hash → PathBuf>` by scanning the
    //    global Mods-archive folder with the size pre-filter + the
    //    persistent hash cache. ──
    let mut cache = HashCache::load(&archive_dir);
    let mut cache_dirty = false;
    let mut present_by_hash: HashMap<String, PathBuf> = HashMap::new();

    if let Ok(read_dir) = std::fs::read_dir(&archive_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            // Skip our own sidecars + any directory.
            let Some((size, mtime)) = file_size_mtime(&path) else {
                continue;
            };
            if is_sidecar(&path) {
                continue;
            }
            // **Size pre-filter** — only hash a file whose length matches
            // some wanted archive's recorded size (the cheap reject; a
            // multi-GB archive of a different size is never hashed).
            if !wanted_sizes.contains(&size) {
                continue;
            }
            // **Persistent path+mtime cache** — a multi-GB archive hashes
            // once ever; a later run with an unchanged file is a cache hit.
            let hash = if let Some(h) = cache.fresh_hash(&path, size, mtime) {
                summary.cache_hits += 1;
                h.to_string()
            } else {
                match archive_store::hash_file(&path) {
                    Ok(h) => {
                        if cache.record(&path, size, mtime, &h) {
                            cache_dirty = true;
                        }
                        h
                    }
                    Err(err) => {
                        warn!(
                            target = "orchestrator",
                            "hash candidate {}: {err} (skipping it as a skip \
                             source — it will just be re-downloaded if wanted)",
                            path.display()
                        );
                        continue;
                    }
                }
            };
            summary.hashed_candidates += 1;
            // Only retain a present file if its hash is one some wanted
            // archive expects (identity by content — coexist: a same-named
            // different-hash on-disk file is simply not a match here).
            if wanted_hashes.contains(hash.as_str()) {
                present_by_hash.entry(hash).or_insert(path);
            }
        }
    }
    if cache_dirty {
        cache.save(&archive_dir);
    }

    // ── Skip pass: drop every wanted asset whose expected hash is present
    //    on disk + present those bytes at BIO's deterministic extract
    //    path. Assets with no expected hash, or whose expected hash is not
    //    on disk, are KEPT for a normal download. ──
    let assets = std::mem::take(&mut state.step2.update_selected_update_assets);
    let mut kept = Vec::with_capacity(assets.len());
    for asset in assets {
        let name = archive_file_name(&asset);
        let Some(meta) = by_name.get(name.as_str()) else {
            // The exporter baked no hash for this archive (or it is a
            // newly-resolved one not in the code) ⇒ always-download
            // fallback (never an error).
            summary.no_expected_hash += 1;
            kept.push(asset);
            continue;
        };
        let Some(present_path) = present_by_hash.get(&meta.hash) else {
            // The expected content is not on disk ⇒ genuine fetch.
            summary.missing_on_disk += 1;
            kept.push(asset);
            continue;
        };
        // **The expected bytes are already on disk** — present them at
        // BIO's deterministic extract path and DROP the asset so the
        // parallel streamer never fetches it (the Wabbajack skip).
        let deterministic = archive_dir.join(&name);
        if present_path == &deterministic {
            // Already exactly where extract looks (same file) — nothing to
            // place; just drop the asset.
            summary.skipped_present += 1;
            continue;
        }
        match link_or_copy(present_path, &deterministic) {
            Ok(()) => {
                summary.skipped_present += 1;
                // Asset intentionally NOT pushed to `kept` ⇒ dropped from
                // the download list ⇒ never fetched.
            }
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "present skip-source {} → {}: {err} (falling back to a \
                     normal download for this archive)",
                    present_path.display(),
                    deterministic.display()
                );
                summary.missing_on_disk += 1;
                kept.push(asset);
            }
        }
    }
    state.step2.update_selected_update_assets = kept;
    summary
}

/// Outcome of [`verify_downloaded_archives`] — counts only (observational).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifySummary {
    /// Downloaded archives whose bytes matched the expected hash ⇒ accepted
    /// + registered in the persistent cache (so the next install is a skip).
    pub verified: usize,
    /// Downloaded archives whose bytes did **not** match the expected hash
    /// ⇒ **deleted + the asset recorded failed** (NOT cached, NOT presented
    /// to extract — the Wabbajack mismatch rule; never silently accepted).
    pub mismatched: usize,
    /// Downloaded archives with no recorded expected hash ⇒ accepted as-is
    /// (today's behavior — nothing to verify against). Not cached here
    /// (`archive_store::ingest` still content-addresses them on the
    /// existing post-download pass).
    pub unverifiable: usize,
}

/// **After the streamer writes the archives: verify + mismatch (the
/// Wabbajack post-download integrity rule).** For every wanted asset that
/// has a recorded expected hash, hash the just-written
/// `archive_dir.join(archive_file_name(asset))`:
///
///   - `actual == expected` ⇒ accept; register `(path, size, mtime) →
///     hash` in the persistent cache so the **next** install skips it.
///   - `actual != expected` ⇒ **delete the bad file** and append a
///     BIO-shaped `"{label}: hash mismatch …"` to
///     `state.step2.update_selected_download_failed_sources` (so BIO's
///     unchanged `auto_build_blocker_before_install` stops the auto-build
///     for that source and extract — which gates on `.exists()` — never
///     sees it). It is **not** cached.
///   - no expected hash ⇒ left as-is (today's behavior; `archive_store`'s
///     existing post-download ingest still content-addresses it).
///
/// `assets` is the **pre-skip** resolved set (so a verify covers what the
/// streamer actually fetched). Idempotent. Zero BIO edit — read-only on
/// BIO's archive dir + the orchestrator-owned failed-sources vec + the
/// cache sidecar; `archive_file_name` reused read-only.
pub fn verify_downloaded_archives(
    state: &mut WizardState,
    expected: &[ArchiveMeta],
    assets: &[Step2UpdateAsset],
) -> VerifySummary {
    let mut summary = VerifySummary::default();
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() || assets.is_empty() {
        return summary;
    }
    let by_name: HashMap<&str, &ArchiveMeta> =
        expected.iter().map(|m| (m.name.as_str(), m)).collect();

    let mut cache = HashCache::load(&archive_dir);
    let mut cache_dirty = false;
    let mut failures: Vec<String> = Vec::new();

    for asset in assets {
        let name = archive_file_name(asset);
        let Some(meta) = by_name.get(name.as_str()) else {
            summary.unverifiable += 1;
            continue; // nothing to verify against (today's behavior)
        };
        let path = archive_dir.join(&name);
        let Some((size, mtime)) = file_size_mtime(&path) else {
            // Not on disk — either it was skipped (already verified on a
            // prior install / staged) or its download failed (BIO's own
            // failed-source tracking already recorded that). Nothing to do.
            continue;
        };
        let actual = match archive_store::hash_file(&path) {
            Ok(h) => h,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "verify-hash {}: {err} — treating as a mismatch (delete + \
                     fail; never silently accept an unverifiable download)",
                    path.display()
                );
                String::new() // forces the mismatch branch below
            }
        };
        if actual == meta.hash && !actual.is_empty() {
            // Match — accept + cache so the NEXT install skips it.
            if cache.record(&path, size, mtime, &actual) {
                cache_dirty = true;
            }
            summary.verified += 1;
        } else {
            // **Mismatch — delete the bad file + record the asset failed.**
            // Never cache it; never present it to extract.
            if let Err(err) = std::fs::remove_file(&path) {
                warn!(
                    target = "orchestrator",
                    "delete hash-mismatched archive {}: {err} (still recorded \
                     failed so extract — which gates on .exists() — and the \
                     auto-build blocker skip it)",
                    path.display()
                );
            }
            // BIO-shaped failed-source string (`"{label}: {err}"` — matches
            // `app_step2_update_download.rs:142` so BIO's unchanged
            // auto-build blocker treats it identically to a fetch failure).
            failures.push(format!(
                "{}: hash mismatch (expected {}, got {}) — deleted, not installed",
                asset.label,
                meta.hash,
                if actual.is_empty() {
                    "unreadable"
                } else {
                    &actual
                }
            ));
            summary.mismatched += 1;
        }
    }

    if cache_dirty {
        cache.save(&archive_dir);
    }
    if !failures.is_empty() {
        state
            .step2
            .update_selected_download_failed_sources
            .extend(failures);
    }
    summary
}

/// `true` if `path` is one of the orchestrator's own JSON sidecars in the
/// global Mods-archive folder (the hash cache, the content-addressed
/// index, or a per-install lock that somehow leaked here) — never a skip
/// candidate.
fn is_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(HASH_CACHE_FILENAME) | Some(ARCHIVE_INDEX_FILENAME) | Some(INSTALL_LOCK_FILENAME)
    ) || path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))
}

/// Present `src`'s bytes at `dst` (BIO's deterministic extract path) —
/// **hardlink preferred, copy fallback** (the brief's directive). A
/// hardlink is instant + zero extra disk for a multi-GB archive; a copy is
/// the portable fallback when hardlinking is unsupported (cross-volume / a
/// filesystem without hardlinks) or `dst` already exists. `dst` is removed
/// first if present so a stale/wrong-content file under that name cannot
/// shadow the correct content (the unverified-`.exists()`-extract hazard
/// this whole run fixes).
fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Remove any existing file at the deterministic name first — it may be
    // a different (stale / wrong-hash) archive; we are placing the
    // content-verified bytes there.
    if dst.exists() {
        let _ = std::fs::remove_file(dst);
    }
    match std::fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => std::fs::copy(src, dst).map(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
        // DATA-LOSS-safe: a unique temp dir; this module never binds the
        // real `%APPDATA%\bio\` (it operates only on an arbitrary
        // Mods-archive dir).
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_archive_skip_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn asset(tp: &str, src: &str, tag: &str, name: &str) -> Step2UpdateAsset {
        Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp.to_string(),
            label: tp.to_string(),
            source_id: src.to_string(),
            tag: tag.to_string(),
            asset_name: name.to_string(),
            asset_url: format!("https://example/{name}"),
            installed_source_ref: None,
        }
    }

    fn meta_for(archive_dir: &Path, asset: &Step2UpdateAsset) -> ArchiveMeta {
        let name = archive_file_name(asset);
        let path = archive_dir.join(&name);
        ArchiveMeta {
            name,
            size: std::fs::metadata(&path).unwrap().len(),
            hash: archive_store::hash_file(&path).unwrap(),
        }
    }

    #[test]
    fn present_archive_is_skipped_not_downloaded_and_placed_for_extract() {
        // SPEC §13.12a — the core bug fix: an archive whose expected
        // {size,hash} is already present on disk is DROPPED from the
        // download list (the streamer never fetches it) and PLACED at
        // BIO's deterministic extract path.
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("AMOD/AMOD.TP2", "github", "v1", "A.zip");
        let name = archive_file_name(&a);
        // The archive is already on disk under the content-addressed store
        // name (NOT yet at the deterministic name — the realistic case:
        // `archive_store` stored it hash-encoded on a prior modlist).
        let stored = archive_dir.join(archive_store::stored_filename(&name, "x"));
        std::fs::write(&stored, b"SHARED-ARCHIVE-BYTES").unwrap();
        let expected = ArchiveMeta {
            name: name.clone(),
            size: "SHARED-ARCHIVE-BYTES".len() as u64,
            hash: archive_store::hash_file(&stored).unwrap(),
        };

        state.step2.update_selected_update_assets = vec![a.clone()];
        let s = skip_present_archives(&mut state, &[expected]);

        assert_eq!(s.skipped_present, 1, "the present archive was skipped");
        assert!(
            state.step2.update_selected_update_assets.is_empty(),
            "the asset was DROPPED ⇒ the streamer never downloads it"
        );
        let deterministic = archive_dir.join(&name);
        assert!(
            deterministic.exists(),
            "the present bytes were placed at BIO's deterministic extract path"
        );
        assert_eq!(
            std::fs::read(&deterministic).unwrap(),
            b"SHARED-ARCHIVE-BYTES",
            "extract gets exactly the expected content"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn same_name_different_hash_is_not_skipped_and_coexists() {
        // SPEC §13.12a coexist: a wanted archive whose expected hash
        // differs from the on-disk same-named one IS downloaded (different
        // version) — the skip only fires for the hash THIS code expects.
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD.zip");
        let name = archive_file_name(&a);
        // On disk: version 1.
        let on_disk = archive_dir.join(archive_store::stored_filename(&name, "v1"));
        std::fs::write(&on_disk, b"VERSION-1-CONTENT").unwrap();
        // Wanted: version 2 (a DIFFERENT hash + size).
        let want_v2_bytes = b"VERSION-2-DIFFERENT-CONTENT";
        let expected = ArchiveMeta {
            name: name.clone(),
            size: want_v2_bytes.len() as u64,
            // A hash that is NOT the on-disk v1 hash.
            hash: {
                let p = archive_dir.join("scratch_v2");
                std::fs::write(&p, want_v2_bytes).unwrap();
                let h = archive_store::hash_file(&p).unwrap();
                std::fs::remove_file(&p).unwrap();
                h
            },
        };

        state.step2.update_selected_update_assets = vec![a.clone()];
        let s = skip_present_archives(&mut state, &[expected]);

        assert_eq!(s.skipped_present, 0, "different hash ⇒ NOT skipped");
        assert_eq!(s.missing_on_disk, 1, "the wanted version is not on disk");
        assert_eq!(
            state.step2.update_selected_update_assets.len(),
            1,
            "the asset is KEPT ⇒ the streamer downloads v2 (coexist)"
        );
        assert!(
            on_disk.exists(),
            "the on-disk v1 is untouched (both versions coexist)"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn no_expected_hash_falls_back_to_always_download() {
        // A fieldless / pre-redesign code ⇒ empty `expected` ⇒ a pure
        // no-op: every asset is kept (today's always-download), never an
        // error.
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        state.step2.update_selected_update_assets = vec![a];

        let s = skip_present_archives(&mut state, &[]);
        assert_eq!(s.skipped_present, 0);
        assert_eq!(s.no_expected_hash, 1);
        assert_eq!(
            state.step2.update_selected_update_assets.len(),
            1,
            "no expected hashes ⇒ everything downloaded as today (no error)"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn size_prefilter_skips_hashing_non_matching_files_and_cache_hits_second_run() {
        // The size pre-filter: a big on-disk file whose size matches no
        // wanted archive is never hashed. And a second skip pass over an
        // unchanged file is a persistent-cache HIT (no re-hash) — a
        // multi-GB archive hashes once ever.
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        let name = archive_file_name(&a);
        let bytes = b"WANTED-ARCHIVE-CONTENT";
        std::fs::write(archive_dir.join(&name), bytes).unwrap();
        // An unrelated big file of a DIFFERENT size — must never be hashed
        // (the size pre-filter rejects it cheaply).
        std::fs::write(archive_dir.join("unrelated-huge.bin"), vec![9u8; 4096]).unwrap();
        let expected = meta_for(&archive_dir, &a);

        // First pass: hashes only the size-matching candidate (1), no
        // cache hit yet.
        state.step2.update_selected_update_assets = vec![a.clone()];
        let s1 = skip_present_archives(&mut state, std::slice::from_ref(&expected));
        assert_eq!(
            s1.hashed_candidates, 1,
            "only the size-matching file was hashed (the 4096-byte unrelated \
             file was rejected by the size pre-filter, never hashed)"
        );
        assert_eq!(s1.cache_hits, 0, "nothing cached before the first pass");
        assert_eq!(s1.skipped_present, 1);
        assert!(
            archive_dir.join(HASH_CACHE_FILENAME).exists(),
            "the persistent hash cache was written"
        );

        // Second pass over the SAME unchanged file ⇒ a pure cache hit (no
        // re-hash — the multi-GB-archive-hashes-once-ever guarantee).
        state.step2.update_selected_update_assets = vec![a.clone()];
        let s2 = skip_present_archives(&mut state, std::slice::from_ref(&expected));
        assert_eq!(
            s2.cache_hits, 1,
            "the unchanged file was a persistent-cache hit (NOT re-hashed)"
        );
        assert_eq!(s2.skipped_present, 1);
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn cache_invalidates_on_mtime_or_size_change() {
        // The cache invalidation rule: a changed file is re-hashed (the
        // cached hash is NOT trusted across a content change).
        let archive_dir = td();
        let p = archive_dir.join("a.bin");
        std::fs::write(&p, b"original").unwrap();
        let (size1, mtime1) = file_size_mtime(&p).unwrap();
        let mut cache = HashCache::default();
        cache.record(&p, size1, mtime1, "ORIGINALHASH");
        assert_eq!(
            cache.fresh_hash(&p, size1, mtime1),
            Some("ORIGINALHASH"),
            "unchanged ⇒ cache hit"
        );
        // Size change ⇒ stale.
        assert_eq!(
            cache.fresh_hash(&p, size1 + 1, mtime1),
            None,
            "a size change invalidates the entry"
        );
        // mtime change ⇒ stale.
        assert_eq!(
            cache.fresh_hash(&p, size1, Some(mtime1.unwrap_or(0) + 1)),
            None,
            "an mtime change invalidates the entry (re-hash)"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn verify_accepts_match_and_caches_it() {
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        let name = archive_file_name(&a);
        std::fs::write(archive_dir.join(&name), b"GOOD-DOWNLOAD").unwrap();
        let expected = meta_for(&archive_dir, &a);

        let v =
            verify_downloaded_archives(&mut state, std::slice::from_ref(&expected), &[a.clone()]);
        assert_eq!(v.verified, 1);
        assert_eq!(v.mismatched, 0);
        assert!(
            state
                .step2
                .update_selected_download_failed_sources
                .is_empty(),
            "a matching download is not recorded failed"
        );
        // It was cached ⇒ a subsequent skip pass is a cache hit + skip.
        let cache = HashCache::load(&archive_dir);
        assert_eq!(
            cache.fresh_hash(
                &archive_dir.join(&name),
                std::fs::metadata(archive_dir.join(&name)).unwrap().len(),
                file_size_mtime(&archive_dir.join(&name)).unwrap().1
            ),
            Some(expected.hash.as_str()),
            "a verified download is registered in the persistent cache"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn verify_deletes_mismatch_and_records_failed_not_cached() {
        // The Wabbajack mismatch rule: a downloaded archive whose bytes
        // don't match the expected hash is DELETED + the asset recorded
        // failed (NOT cached, NOT presented to extract).
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("BAD/BAD.TP2", "gh", "v9", "BAD.zip");
        let name = archive_file_name(&a);
        // The streamer wrote CORRUPT bytes (wrong content for the expected
        // hash).
        std::fs::write(archive_dir.join(&name), b"CORRUPT-OR-TAMPERED").unwrap();
        let expected = ArchiveMeta {
            name: name.clone(),
            // size matches but the hash is of DIFFERENT content.
            size: "CORRUPT-OR-TAMPERED".len() as u64,
            hash: "00000000000000000000000000000000".to_string(),
        };

        let v = verify_downloaded_archives(&mut state, &[expected], &[a.clone()]);
        assert_eq!(v.mismatched, 1);
        assert_eq!(v.verified, 0);
        assert!(
            !archive_dir.join(&name).exists(),
            "the hash-mismatched archive was DELETED (never accepted)"
        );
        assert_eq!(
            state.step2.update_selected_download_failed_sources.len(),
            1,
            "the asset was recorded failed (BIO-shaped) so extract + the \
             auto-build blocker skip it"
        );
        assert!(
            state.step2.update_selected_download_failed_sources[0]
                .starts_with(&format!("{}: ", a.label)),
            "failed entry is the BIO-shaped \"{{label}}: {{err}}\""
        );
        // It was NOT cached (a mismatch must never poison the cache).
        assert!(
            HashCache::load(&archive_dir).entries.is_empty(),
            "a hash-mismatched archive is never registered in the cache"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn verify_leaves_unverifiable_alone() {
        // No expected hash for the asset ⇒ today's behavior (accept as-is,
        // archive_store's existing ingest still content-addresses it).
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        std::fs::write(archive_dir.join(archive_file_name(&a)), b"x").unwrap();

        let v = verify_downloaded_archives(&mut state, &[], &[a.clone()]);
        assert_eq!(v.unverifiable, 1);
        assert_eq!(v.mismatched, 0);
        assert!(
            archive_dir.join(archive_file_name(&a)).exists(),
            "an unverifiable download is left as-is (today's behavior)"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn corrupt_hash_cache_degrades_to_empty_not_panic() {
        let archive_dir = td();
        std::fs::write(archive_dir.join(HASH_CACHE_FILENAME), b"{ not json").unwrap();
        assert!(
            HashCache::load(&archive_dir).entries.is_empty(),
            "a corrupt cache degrades to empty (worst case: one re-hash)"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn sidecars_are_never_skip_candidates() {
        let d = td();
        assert!(is_sidecar(&d.join(HASH_CACHE_FILENAME)));
        assert!(is_sidecar(&d.join(ARCHIVE_INDEX_FILENAME)));
        assert!(is_sidecar(&d.join(INSTALL_LOCK_FILENAME)));
        assert!(is_sidecar(&d.join("something.json.tmp")));
        assert!(!is_sidecar(&d.join("real-archive.zip")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
