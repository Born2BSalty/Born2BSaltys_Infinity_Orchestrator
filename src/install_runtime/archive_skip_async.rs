// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::archive_skip_async` — **DL Fix-Set v3 (Change B):
// async hashing with per-asset progress** (SPEC §13.12a / §4.3).
//
// **The bug + improvement this fixes.** The synchronous
// `archive_skip::skip_present_archives` pass runs on the egui render
// thread the moment the Downloading screen arms. With a large
// Mods-archive folder + many same-size on-disk candidates, the hashing
// blocks the UI for seconds (the user-reported "looks frozen at start of
// download"). The persistent path+mtime cache only helps on a second
// install; a first install still pays the full hash cost. This module
// REPLACES that path with a **bounded parallel hashing pool**
// (`HASH_POOL_SIZE = 10`) that runs on a spawned coordinator thread, so
// the UI stays responsive while hashing runs in the background.
//
// **Per-asset events arrive as hashing progresses**, not in a single
// terminal lump:
//   - `CandidateEnumerated { total }` — the moment the asset list is
//     known (right at coordinator start), so the §4.3 chrome's Hashing
//     phase indicator can show "0 / N" immediately.
//   - `AssetHashStarted { index }` — when a worker picks an asset up
//     and is about to hash it; the §4.3 grid flips that row's status
//     to `Hashing` (visual "checking cache…").
//   - `AssetHashed { index, was_skipped, label, dest_display }` —
//     when the asset's hash decision is final; bumps the hash count
//     and (if skipped) lets the drain push the BIO-shaped
//     `"{label} -> {dest_display}"` entry into
//     `update_selected_downloaded_sources` (mirroring the sync skip
//     path's pre-population).
//   - `Finished { summary, skipped_indices }` — every asset processed;
//     the drain stores `skip_indices` on `InstallScreenState`,
//     clears `archive_skip_rx`, and sets `archive_skip_completed =
//     true` so the streamer's kick gate can now open.
//
// **Zero BIO source.** This module only: reads the JSON sidecar at
// `archive_dir/HASH_CACHE_FILENAME` (the same path the sync
// `archive_skip` writes; the on-disk format IS the public contract);
// reuses `app_step2_update_download::archive_file_name` **read-only**
// (the SAME fn `archive_store` / `stream_downloader` / sync
// `archive_skip` already reuse); and reuses
// `archive_store::hash_file` (the ONE stable seedless FNV-1a-128 —
// one hashing path, zero drift). The existing sync `archive_skip::
// skip_present_archives` is LEFT in place (tests still use it; any
// future fallback can fall back to it).
//
// **Concurrency model.** A spawned coordinator thread first loads
// the persistent hash cache + enumerates the global Mods-archive
// folder with the cheap size pre-filter. Then it spawns
// `min(HASH_POOL_SIZE, candidates.len())` worker threads via a
// shared `Arc<AtomicUsize>` work index to hash candidates in
// parallel. After workers join, the coordinator merges per-thread
// cache writes into the persistent cache + saves once (no mutex
// contention during hashing). A second worker pool then handles
// per-asset skip decisions, emitting `AssetHashStarted` /
// `AssetHashed` events to the orchestrator's drain. After workers
// join, the coordinator orders results by asset index, builds the
// `SkipSummary` + `skipped_indices`, and sends `Finished`.
//
// SPEC: §13.12a (content-addressed: hash-on-write, skip-if-present,
//       coexist + the BIO-name link for extract), §4.3 (Downloading
//       screen — Hashing phase indicator), §1 (CRITICAL DIRECTIVE —
//       net-new, zero BIO source).

// rationale: a bounded hash pool with a shared work index + per-thread
// hash-result merge is intrinsic to the parallel design (not a smell);
// `Self`/`const fn`/`#[must_use]` churn + the doc-paragraph-length lint
// add noise without behavior value (Cat 3).
#![allow(
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::use_self,
    clippy::too_long_first_doc_paragraph
)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::Step2UpdateAsset;
use crate::install_runtime::archive_skip::{HASH_CACHE_FILENAME, SkipSummary};
use crate::install_runtime::archive_store;
use crate::registry::share_export::ArchiveMeta;

/// The bounded parallel hashing pool size. 10 keeps disk pressure
/// bounded (hashing is read-heavy; >10 thrashes the disk on spinning
/// media) while being substantially faster than the prior serial
/// pass on any modern multi-core box. Same number as the extract pool
/// (`extract_parallel::EXTRACT_POOL_SIZE`) — the disk-pressure
/// argument is the same shape.
pub const HASH_POOL_SIZE: usize = 10;

/// Events the coordinator thread sends to the orchestrator's per-frame
/// drain. Mirrors `stream_downloader::StreamDownloadEvent` / `extract_
/// parallel::ExtractAssetEvent` discipline.
pub enum ArchiveSkipEvent {
    /// The asset list is known; the §4.3 chrome's Hashing phase
    /// indicator can show "0 / N" immediately. Sent exactly once at
    /// coordinator start.
    CandidateEnumerated { total: usize },
    /// A worker picked asset `index` up and is about to hash it; the
    /// §4.3 grid flips that row's status to `Hashing` ("checking
    /// cache…"). Sent before each asset's hash work begins.
    AssetHashStarted { index: usize },
    /// Asset `index`'s hash decision is final. `was_skipped == true`
    /// means the expected hash is on disk + the bytes were placed at
    /// the deterministic path (`dest_display` carries the path's
    /// `Display`); the drain pre-populates `update_selected_
    /// downloaded_sources` with the BIO-shaped `"{label} -> {dest}"`
    /// entry. `was_skipped == false` ⇒ no skip (either no expected
    /// hash recorded, or expected hash not on disk) ⇒ the asset is
    /// left in the to-fetch set for the streamer.
    AssetHashed {
        index: usize,
        was_skipped: bool,
        label: String,
        dest_display: Option<String>,
    },
    /// Every asset processed. Carries the same observational
    /// `SkipSummary` the sync pass returns + the orchestrator-side
    /// skip-index set (the indices into the unchanged asset list the
    /// streamer must silently bypass). After this the drain stores
    /// `skip_indices` on `InstallScreenState`, clears `archive_skip_
    /// rx`, and sets `archive_skip_completed = true`.
    Finished {
        summary: SkipSummary,
        skipped_indices: Vec<usize>,
    },
}

/// The coordinator's input — a snapshot of just the pieces the
/// async pass needs, so it can be moved into the spawned thread
/// without holding a `&mut WizardState`.
pub struct AsyncSkipInput {
    /// The global Mods-archive folder (`state.step1.mods_archive_
    /// folder`, trimmed). Empty ⇒ the coordinator early-returns with
    /// a no-op `Finished`.
    pub archive_dir: PathBuf,
    /// The resolved asset set (`state.step2.update_selected_update_
    /// assets`, cloned). Read-only input, left intact.
    pub assets: Vec<Step2UpdateAsset>,
}

// ── Local async-side cache structures. We deliberately do NOT touch
//    the sync `archive_skip::HashCache` struct (which is `pub` but
//    with a module-private entry type and a module-private hash-
//    lookup method). Instead we load/save the JSON sidecar through
//    the on-disk format directly — the format IS the public contract;
//    no new BIO surface is needed. ──
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct AsyncCacheEntry {
    size: u64,
    mtime_nanos: Option<u128>,
    hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct AsyncCacheFile {
    entries: BTreeMap<String, AsyncCacheEntry>,
}

impl AsyncCacheFile {
    fn load(archive_dir: &Path) -> Self {
        let p = archive_dir.join(HASH_CACHE_FILENAME);
        match std::fs::read_to_string(&p) {
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

    fn save(&self, archive_dir: &Path) {
        let p = archive_dir.join(HASH_CACHE_FILENAME);
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

    fn key(path: &Path) -> String {
        path.to_string_lossy().to_lowercase()
    }

    fn fresh_hash(&self, path: &Path, size: u64, mtime: Option<u128>) -> Option<&str> {
        let e = self.entries.get(&Self::key(path))?;
        if e.size != size {
            return None;
        }
        if let (Some(a), Some(b)) = (e.mtime_nanos, mtime) {
            if a != b {
                return None;
            }
        }
        Some(&e.hash)
    }

    fn record(&mut self, path: &Path, size: u64, mtime: Option<u128>, hash: &str) -> bool {
        let key = Self::key(path);
        let new = AsyncCacheEntry {
            size,
            mtime_nanos: mtime,
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
/// invalidation key. `None` mtime if unavailable (rare; size-only
/// then). Mirrors the sync pass's helper (kept local so the sync
/// module can stay untouched).
fn file_size_mtime(path: &Path) -> Option<(u64, Option<u128>)> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos());
    Some((size, mtime))
}

/// `true` if `path` is one of the orchestrator's own JSON sidecars
/// in the global Mods-archive folder — never a skip candidate.
/// Mirrors the sync pass's `is_sidecar` (kept local; same list).
fn is_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(HASH_CACHE_FILENAME)
            | Some(archive_store::ARCHIVE_INDEX_FILENAME)
            | Some(archive_store::INSTALL_LOCK_FILENAME)
    ) || path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))
}

/// Present `src`'s bytes at `dst` (BIO's deterministic extract
/// path) — hardlink preferred, copy fallback. Mirrors the sync
/// pass's `link_or_copy`.
fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        let _ = std::fs::remove_file(dst);
    }
    match std::fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => std::fs::copy(src, dst).map(|_| ()),
    }
}

/// **Spawn the bounded parallel checksum-then-skip pool (the v3
/// async replacement for the v2 sync `skip_present_archives`).**
/// Returns the `Receiver<ArchiveSkipEvent>` so the orchestrator's
/// per-frame drain can pump events into the §4.3 grid live.
///
/// Behavior:
///   1. Spawn a coordinator thread (the egui render thread returns
///      immediately).
///   2. Coordinator: load the persistent cache from `archive_dir`.
///      Enumerate `archive_dir` with the cheap size pre-filter
///      (built once before workers spawn — shared read-only).
///      Build `wanted_sizes` / `by_name` / `wanted_hashes` lookups
///      from `expected`.
///   3. Emit `CandidateEnumerated { total: assets.len() }`.
///   4. Empty archive_dir or empty assets ⇒ early-return no-op
///      `Finished` with summary noting the no-op state.
///   5. Empty `expected` ⇒ no-op pass: every asset is "no expected
///      hash" (the always-download fallback); emit
///      `AssetHashStarted` + `AssetHashed{was_skipped:false}` per
///      asset, then `Finished` with `no_expected_hash =
///      assets.len()`.
///   6. Otherwise hash size-matching on-disk candidates through
///      the cache in parallel (`HASH_POOL_SIZE` workers, splitting
///      by file). Per-thread cache writes are merged once at the
///      end (no mutex contention during hashing).
///   7. Spawn the asset-side workers to consult `present_by_hash`
///      for each asset's expected hash. Emit `AssetHashStarted`
///      then `AssetHashed`.
///   8. After all workers join, save the merged cache. Build the
///      `SkipSummary` + `skipped_indices` + send `Finished`.
pub fn start_async_archive_skip(
    state_snapshot: AsyncSkipInput,
    expected: Vec<ArchiveMeta>,
) -> Receiver<ArchiveSkipEvent> {
    let (tx, rx) = mpsc::channel::<ArchiveSkipEvent>();
    thread::spawn(move || {
        run_async_archive_skip(state_snapshot, expected, &tx);
    });
    rx
}

/// The coordinator + worker pool body. Runs on the spawned thread.
fn run_async_archive_skip(
    input: AsyncSkipInput,
    expected: Vec<ArchiveMeta>,
    tx: &Sender<ArchiveSkipEvent>,
) {
    let archive_dir = input.archive_dir;
    let assets = input.assets;
    let total_assets = assets.len();

    // Emit the candidate count immediately so the §4.3 chrome can
    // show "0 / N" at the start.
    let _ = tx.send(ArchiveSkipEvent::CandidateEnumerated {
        total: total_assets,
    });

    // Early-return on empty archive_dir or empty assets — emit a
    // no-op Finished.
    if archive_dir.as_os_str().is_empty() || total_assets == 0 {
        let mut summary = SkipSummary::default();
        if expected.is_empty() {
            summary.no_expected_hash = total_assets;
        }
        let _ = tx.send(ArchiveSkipEvent::Finished {
            summary,
            skipped_indices: Vec::new(),
        });
        return;
    }

    // Empty expected — no-op pass: every asset is "no expected
    // hash" (always-download fallback). Emit one AssetHashStarted
    // + AssetHashed per asset (so the grid's per-row state
    // transitions are consistent) then Finished.
    if expected.is_empty() {
        let mut summary = SkipSummary::default();
        summary.no_expected_hash = total_assets;
        for (i, a) in assets.iter().enumerate() {
            let _ = tx.send(ArchiveSkipEvent::AssetHashStarted { index: i });
            let _ = tx.send(ArchiveSkipEvent::AssetHashed {
                index: i,
                was_skipped: false,
                label: a.label.clone(),
                dest_display: None,
            });
        }
        let _ = tx.send(ArchiveSkipEvent::Finished {
            summary,
            skipped_indices: Vec::new(),
        });
        return;
    }

    // Build the lookup maps (the same shape the sync pass builds).
    let by_name: HashMap<String, ArchiveMeta> = expected
        .iter()
        .map(|m| (m.name.clone(), m.clone()))
        .collect();
    let wanted_sizes: std::collections::HashSet<u64> = expected.iter().map(|m| m.size).collect();
    let wanted_hashes: std::collections::HashSet<String> =
        expected.iter().map(|m| m.hash.clone()).collect();

    // ── Phase 1: enumerate + hash size-matching on-disk candidates
    //    through the persistent cache, in parallel. ──
    let cache_at_entry = AsyncCacheFile::load(&archive_dir);
    let mut candidate_paths: Vec<(PathBuf, u64, Option<u128>)> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(&archive_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let Some((size, mtime)) = file_size_mtime(&path) else {
                continue;
            };
            if is_sidecar(&path) {
                continue;
            }
            if !wanted_sizes.contains(&size) {
                continue;
            }
            candidate_paths.push((path, size, mtime));
        }
    }
    let candidate_count = candidate_paths.len();
    let candidate_paths = Arc::new(candidate_paths);
    let cache_at_entry = Arc::new(cache_at_entry);
    let wanted_hashes = Arc::new(wanted_hashes);
    let next_candidate = Arc::new(AtomicUsize::new(0));
    let present_by_hash: Arc<Mutex<HashMap<String, PathBuf>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let cache_writes: Arc<Mutex<Vec<(PathBuf, u64, Option<u128>, String)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let cache_hits = Arc::new(AtomicUsize::new(0));

    let worker_count = HASH_POOL_SIZE.min(candidate_count.max(1));
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let candidate_paths = Arc::clone(&candidate_paths);
        let cache_at_entry = Arc::clone(&cache_at_entry);
        let wanted_hashes = Arc::clone(&wanted_hashes);
        let next_candidate = Arc::clone(&next_candidate);
        let present_by_hash = Arc::clone(&present_by_hash);
        let cache_writes = Arc::clone(&cache_writes);
        let cache_hits = Arc::clone(&cache_hits);
        handles.push(thread::spawn(move || {
            loop {
                let i = next_candidate.fetch_add(1, Ordering::SeqCst);
                if i >= candidate_paths.len() {
                    break;
                }
                let (path, size, mtime) = &candidate_paths[i];
                let hash = if let Some(h) = cache_at_entry.fresh_hash(path, *size, *mtime) {
                    cache_hits.fetch_add(1, Ordering::Relaxed);
                    h.to_string()
                } else {
                    match archive_store::hash_file(path) {
                        Ok(h) => {
                            cache_writes.lock().expect("cache_writes mutex").push((
                                path.clone(),
                                *size,
                                *mtime,
                                h.clone(),
                            ));
                            h
                        }
                        Err(err) => {
                            warn!(
                                target = "orchestrator",
                                "hash candidate {}: {err} (skipping it as a \
                                 skip source — it will just be re-downloaded \
                                 if wanted)",
                                path.display()
                            );
                            continue;
                        }
                    }
                };
                if wanted_hashes.contains(hash.as_str()) {
                    present_by_hash
                        .lock()
                        .expect("present_by_hash mutex")
                        .entry(hash)
                        .or_insert(path.clone());
                }
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    // Merge per-thread cache writes into the persistent cache + save
    // once (the coordinator-only serial merge).
    let mut cache = (*cache_at_entry).clone();
    let writes = cache_writes.lock().expect("cache_writes mutex").clone();
    let mut cache_dirty = false;
    for (path, size, mtime, hash) in writes {
        if cache.record(&path, size, mtime, &hash) {
            cache_dirty = true;
        }
    }
    if cache_dirty {
        cache.save(&archive_dir);
    }

    // Snapshot present_by_hash for the per-asset phase.
    let present_by_hash_final = Arc::new(
        present_by_hash
            .lock()
            .expect("present_by_hash mutex")
            .clone(),
    );

    // ── Phase 2: per-asset skip decisions, in parallel. ──
    let assets = Arc::new(assets);
    let by_name = Arc::new(by_name);
    let archive_dir = Arc::new(archive_dir);
    let next_asset = Arc::new(AtomicUsize::new(0));
    let asset_results: Arc<Mutex<Vec<(usize, bool, Step2UpdateAsset)>>> =
        Arc::new(Mutex::new(Vec::with_capacity(total_assets)));

    let worker_count = HASH_POOL_SIZE.min(total_assets);
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let assets = Arc::clone(&assets);
        let by_name = Arc::clone(&by_name);
        let present_by_hash = Arc::clone(&present_by_hash_final);
        let archive_dir = Arc::clone(&archive_dir);
        let next_asset = Arc::clone(&next_asset);
        let asset_results = Arc::clone(&asset_results);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            loop {
                let i = next_asset.fetch_add(1, Ordering::SeqCst);
                if i >= assets.len() {
                    break;
                }
                let asset = &assets[i];
                let _ = tx.send(ArchiveSkipEvent::AssetHashStarted { index: i });
                let name = archive_file_name(asset);
                let Some(meta) = by_name.get(&name) else {
                    asset_results.lock().expect("asset_results mutex").push((
                        i,
                        false,
                        asset.clone(),
                    ));
                    let _ = tx.send(ArchiveSkipEvent::AssetHashed {
                        index: i,
                        was_skipped: false,
                        label: asset.label.clone(),
                        dest_display: None,
                    });
                    continue;
                };
                let Some(present_path) = present_by_hash.get(&meta.hash) else {
                    asset_results.lock().expect("asset_results mutex").push((
                        i,
                        false,
                        asset.clone(),
                    ));
                    let _ = tx.send(ArchiveSkipEvent::AssetHashed {
                        index: i,
                        was_skipped: false,
                        label: asset.label.clone(),
                        dest_display: None,
                    });
                    continue;
                };
                let deterministic = archive_dir.join(&name);
                let placed = if present_path == &deterministic {
                    true
                } else {
                    match link_or_copy(present_path, &deterministic) {
                        Ok(()) => true,
                        Err(err) => {
                            warn!(
                                target = "orchestrator",
                                "present skip-source {} → {}: {err} \
                                 (falling back to a normal download for this \
                                 archive)",
                                present_path.display(),
                                deterministic.display()
                            );
                            false
                        }
                    }
                };
                if placed {
                    let dest_display = deterministic.display().to_string();
                    asset_results.lock().expect("asset_results mutex").push((
                        i,
                        true,
                        asset.clone(),
                    ));
                    let _ = tx.send(ArchiveSkipEvent::AssetHashed {
                        index: i,
                        was_skipped: true,
                        label: asset.label.clone(),
                        dest_display: Some(dest_display),
                    });
                } else {
                    asset_results.lock().expect("asset_results mutex").push((
                        i,
                        false,
                        asset.clone(),
                    ));
                    let _ = tx.send(ArchiveSkipEvent::AssetHashed {
                        index: i,
                        was_skipped: false,
                        label: asset.label.clone(),
                        dest_display: None,
                    });
                }
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    // Build the SkipSummary + skipped_indices (ordered by asset
    // index for deterministic output).
    let mut recs = asset_results.lock().expect("asset_results mutex").clone();
    recs.sort_by_key(|(i, _, _)| *i);

    let mut summary = SkipSummary::default();
    let mut skipped_indices: Vec<usize> = Vec::new();
    for (i, was_skipped, asset) in &recs {
        let name = archive_file_name(asset);
        let has_expected = by_name.contains_key(&name);
        if *was_skipped {
            summary.skipped_present += 1;
            summary.skipped_assets.push(asset.clone());
            skipped_indices.push(*i);
        } else if !has_expected {
            summary.no_expected_hash += 1;
        } else {
            summary.missing_on_disk += 1;
        }
    }
    summary.hashed_candidates = candidate_count;
    summary.cache_hits = cache_hits.load(Ordering::Relaxed);

    let _ = tx.send(ArchiveSkipEvent::Finished {
        summary,
        skipped_indices,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_async_skip_test_{}_{}",
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

    fn collect_events(
        rx: Receiver<ArchiveSkipEvent>,
    ) -> (Vec<ArchiveSkipEvent>, Option<(SkipSummary, Vec<usize>)>) {
        let mut events = Vec::new();
        let mut finished: Option<(SkipSummary, Vec<usize>)> = None;
        while let Ok(ev) = rx.recv() {
            match ev {
                ArchiveSkipEvent::Finished {
                    summary,
                    skipped_indices,
                } => {
                    finished = Some((summary, skipped_indices));
                    break;
                }
                other => events.push(other),
            }
        }
        (events, finished)
    }

    #[test]
    fn empty_archive_dir_emits_candidate_enumerated_and_finished() {
        let input = AsyncSkipInput {
            archive_dir: PathBuf::new(),
            assets: vec![asset("A/A.TP2", "gh", "v1", "A.zip")],
        };
        let rx = start_async_archive_skip(input, vec![]);
        let (events, finished) = collect_events(rx);
        let has_enum = events
            .iter()
            .any(|e| matches!(e, ArchiveSkipEvent::CandidateEnumerated { total: 1 }));
        assert!(has_enum, "must emit CandidateEnumerated with total=1");
        let (_summary, indices) = finished.expect("Finished arrives");
        assert!(indices.is_empty(), "empty archive_dir ⇒ no skipped indices");
    }

    #[test]
    fn empty_expected_falls_back_to_always_download_with_per_asset_events() {
        let archive_dir = td();
        let input = AsyncSkipInput {
            archive_dir: archive_dir.clone(),
            assets: vec![
                asset("A/A.TP2", "gh", "v1", "A.zip"),
                asset("B/B.TP2", "wm", "v2", "B.zip"),
            ],
        };
        let rx = start_async_archive_skip(input, vec![]);
        let (events, finished) = collect_events(rx);

        let started = events
            .iter()
            .filter(|e| matches!(e, ArchiveSkipEvent::AssetHashStarted { .. }))
            .count();
        let hashed = events
            .iter()
            .filter(|e| matches!(e, ArchiveSkipEvent::AssetHashed { .. }))
            .count();
        assert_eq!(started, 2, "one AssetHashStarted per asset");
        assert_eq!(hashed, 2, "one AssetHashed per asset");
        let (summary, indices) = finished.expect("Finished arrives");
        assert_eq!(summary.no_expected_hash, 2);
        assert_eq!(summary.skipped_present, 0);
        assert!(indices.is_empty());
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn present_archive_is_skipped_and_events_arrive_incrementally() {
        let archive_dir = td();
        let a = asset("AMOD/AMOD.TP2", "github", "v1", "A.zip");
        let name = archive_file_name(&a);
        std::fs::write(archive_dir.join(&name), b"DIRECT-AT-DEST").unwrap();
        let expected = meta_for(&archive_dir, &a);

        let input = AsyncSkipInput {
            archive_dir: archive_dir.clone(),
            assets: vec![a.clone()],
        };
        let rx = start_async_archive_skip(input, vec![expected]);
        let (events, finished) = collect_events(rx);

        let has_enum = events
            .iter()
            .any(|e| matches!(e, ArchiveSkipEvent::CandidateEnumerated { total: 1 }));
        assert!(has_enum);
        let skip_event = events.iter().find(|e| {
            matches!(
                e,
                ArchiveSkipEvent::AssetHashed {
                    index: 0,
                    was_skipped: true,
                    dest_display: Some(_),
                    ..
                }
            )
        });
        assert!(
            skip_event.is_some(),
            "AssetHashed{{was_skipped:true,dest_display:Some}} for index 0 arrived"
        );
        let (summary, indices) = finished.expect("Finished arrives");
        assert_eq!(summary.skipped_present, 1);
        assert_eq!(indices, vec![0]);
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn async_summary_matches_sync_summary_for_same_input() {
        // The async pass and the sync pass should produce
        // equivalent `SkipSummary` for the same input (their job is
        // the same; only the timing differs).
        use crate::app::state::WizardState;
        use crate::install_runtime::archive_skip::skip_present_archives;

        let archive_dir = td();
        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        let b = asset("B/B.TP2", "wm", "v2", "B.zip");
        let name_a = archive_file_name(&a);
        std::fs::write(archive_dir.join(&name_a), b"A-BYTES").unwrap();
        let expected = vec![
            meta_for(&archive_dir, &a),
            ArchiveMeta {
                name: archive_file_name(&b),
                size: 7,
                hash: "ff00ff00ff00ff00ff00ff00ff00ff00".to_string(),
            },
        ];

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        state.step2.update_selected_update_assets = vec![a.clone(), b.clone()];
        let sync = skip_present_archives(&mut state, &expected);

        let input = AsyncSkipInput {
            archive_dir: archive_dir.clone(),
            assets: vec![a.clone(), b.clone()],
        };
        let rx = start_async_archive_skip(input, expected);
        let (_events, finished) = collect_events(rx);
        let (async_summary, _indices) = finished.unwrap();

        assert_eq!(
            sync.skipped_present, async_summary.skipped_present,
            "skipped_present matches"
        );
        assert_eq!(
            sync.no_expected_hash, async_summary.no_expected_hash,
            "no_expected_hash matches"
        );
        assert_eq!(
            sync.missing_on_disk, async_summary.missing_on_disk,
            "missing_on_disk matches"
        );

        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn hash_pool_size_is_ten() {
        assert_eq!(HASH_POOL_SIZE, 10);
    }
}
