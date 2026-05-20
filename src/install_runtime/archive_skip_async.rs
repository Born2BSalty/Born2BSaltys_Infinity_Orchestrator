// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Bounded parallel hashing pool for archive-skip decisions.
//!
//! Runs on a spawned coordinator thread so the UI stays responsive while
//! candidate archives in the Mods-archive folder are hashed against the
//! share-code's expected hashes. Two worker pools (size [`HASH_POOL_SIZE`])
//! run in sequence: phase 1 hashes the size-matching on-disk candidates
//! through the persistent cache, phase 2 consults the resulting
//! present-by-hash map per asset and emits per-asset events to the
//! orchestrator's drain.

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

/// Bounded parallel hashing pool size. Matches
/// [`crate::install_runtime::extract_parallel::EXTRACT_POOL_SIZE`] so the
/// disk-pressure shape is the same across phases.
pub const HASH_POOL_SIZE: usize = 10;

/// Events the coordinator thread sends to the orchestrator's per-frame
/// drain.
pub enum ArchiveSkipEvent {
    /// The asset list is known. Sent exactly once at coordinator start
    /// so the chrome's Hashing phase indicator can show `0 / N`
    /// immediately.
    CandidateEnumerated { total: usize },
    /// A worker is about to hash the asset at `index`; the grid flips
    /// that row's status to `Hashing`.
    AssetHashStarted { index: usize },
    /// Asset `index`'s hash decision is final. `was_skipped` ⇒ the
    /// expected hash is on disk and the bytes were placed at
    /// `dest_display` (the deterministic path); the drain pre-populates
    /// `update_selected_downloaded_sources` with `"{label} -> {dest}"`.
    /// `!was_skipped` ⇒ the asset is left in the to-fetch set.
    AssetHashed {
        index: usize,
        was_skipped: bool,
        label: String,
        dest_display: Option<String>,
    },
    /// Every asset processed. Carries the observational [`SkipSummary`]
    /// + the asset indices the streamer must silently bypass.
    Finished {
        summary: SkipSummary,
        skipped_indices: Vec<usize>,
    },
}

/// Snapshot of the pieces the async pass needs, moved into the spawned
/// coordinator thread so no `&mut WizardState` is held across it.
pub struct AsyncSkipInput {
    /// The Mods-archive folder. Empty ⇒ the coordinator early-returns
    /// with a no-op `Finished`.
    pub archive_dir: PathBuf,
    /// The resolved asset set; read-only input, left intact.
    pub assets: Vec<Step2UpdateAsset>,
}

// Local cache types. We deliberately do NOT touch the sync
// `archive_skip::HashCache` (its entry type + hash-lookup are private);
// the on-disk JSON format IS the public contract so we load/save it
// directly.
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
        let Ok(text) = std::fs::read_to_string(&p) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|err| {
            warn!(
                target = "orchestrator",
                "archive hash cache unreadable ({err}); continuing empty \
                 (worst case: a one-time re-hash)"
            );
            Self::default()
        })
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
        if let (Some(a), Some(b)) = (e.mtime_nanos, mtime)
            && a != b
        {
            return None;
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

/// `(size, mtime_nanos)` for `path`. `None` if not a file; the inner
/// `mtime` is `None` when unavailable (size-only invalidation then).
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

/// `true` if `path` is one of the orchestrator's own JSON sidecars in
/// the Mods-archive folder — never a skip candidate.
fn is_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(
            HASH_CACHE_FILENAME
                | archive_store::ARCHIVE_INDEX_FILENAME
                | archive_store::INSTALL_LOCK_FILENAME
        )
    ) || path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))
}

/// Present `src`'s bytes at `dst` — hardlink preferred, copy fallback.
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

/// `(path, size, mtime_nanos)` for one size-matching on-disk candidate.
type CandidateMeta = (PathBuf, u64, Option<u128>);

/// A `(path, size, mtime, hash)` tuple a worker wants to record in the
/// persistent cache after a fresh hash.
type CacheWrite = (PathBuf, u64, Option<u128>, String);

/// Per-asset result the asset-side workers push into a shared vec; the
/// coordinator orders them by `index` to build the summary.
type AssetRecord = (usize, bool, Step2UpdateAsset);

/// Spawn the bounded parallel checksum-then-skip pool.
///
/// Returns a receiver the orchestrator's per-frame drain consumes.
/// The coordinator runs on a spawned thread (the caller returns
/// immediately) and progresses through:
/// 1. Emit `CandidateEnumerated { total: assets.len() }`.
/// 2. Early-return no-op `Finished` if `archive_dir` or `assets` is
///    empty.
/// 3. If `expected` is empty: emit one `AssetHashStarted` +
///    `AssetHashed { was_skipped: false }` per asset and `Finished`
///    with `no_expected_hash == assets.len()`.
/// 4. Otherwise hash size-matching on-disk candidates through the
///    persistent cache in parallel ([`HASH_POOL_SIZE`] workers,
///    splitting by file). Per-thread cache writes merge once after
///    workers join.
/// 5. Run a second parallel pool over assets: each worker consults
///    `present_by_hash` for the asset's expected hash and emits
///    `AssetHashStarted` then `AssetHashed`.
/// 6. Send `Finished` with the ordered [`SkipSummary`] +
///    `skipped_indices`.
#[must_use]
pub fn start_async_archive_skip(
    state_snapshot: AsyncSkipInput,
    expected: Vec<ArchiveMeta>,
) -> Receiver<ArchiveSkipEvent> {
    let (tx, rx) = mpsc::channel::<ArchiveSkipEvent>();
    thread::spawn(move || {
        run_async_archive_skip(state_snapshot, &expected, &tx);
    });
    rx
}

/// Coordinator body: runs on the spawned thread.
fn run_async_archive_skip(
    input: AsyncSkipInput,
    expected: &[ArchiveMeta],
    tx: &Sender<ArchiveSkipEvent>,
) {
    let AsyncSkipInput {
        archive_dir,
        assets,
    } = input;
    let total_assets = assets.len();

    let _ = tx.send(ArchiveSkipEvent::CandidateEnumerated {
        total: total_assets,
    });

    if archive_dir.as_os_str().is_empty() || total_assets == 0 {
        let summary = SkipSummary {
            no_expected_hash: if expected.is_empty() { total_assets } else { 0 },
            ..Default::default()
        };
        let _ = tx.send(ArchiveSkipEvent::Finished {
            summary,
            skipped_indices: Vec::new(),
        });
        return;
    }

    if expected.is_empty() {
        emit_always_download(&assets, tx);
        return;
    }

    let by_name: HashMap<String, ArchiveMeta> = expected
        .iter()
        .map(|m| (m.name.clone(), m.clone()))
        .collect();
    let wanted_sizes: std::collections::HashSet<u64> = expected.iter().map(|m| m.size).collect();
    let wanted_hashes: std::collections::HashSet<String> =
        expected.iter().map(|m| m.hash.clone()).collect();

    let candidate_paths = enumerate_candidates(&archive_dir, &wanted_sizes);
    let candidate_count = candidate_paths.len();
    let (present_by_hash, cache_writes, cache_hits) =
        hash_candidates_parallel(&archive_dir, candidate_paths, &wanted_hashes);
    let recs = decide_per_asset_parallel(assets, by_name.clone(), present_by_hash, archive_dir, tx);
    let (summary, skipped_indices) = build_summary(&recs, &by_name, candidate_count, cache_hits);

    drop(cache_writes);

    let _ = tx.send(ArchiveSkipEvent::Finished {
        summary,
        skipped_indices,
    });
}

/// No-expected-hash fallback: emit `AssetHashStarted` + `AssetHashed`
/// per asset and a `Finished` with `no_expected_hash == assets.len()`.
fn emit_always_download(assets: &[Step2UpdateAsset], tx: &Sender<ArchiveSkipEvent>) {
    let summary = SkipSummary {
        no_expected_hash: assets.len(),
        ..Default::default()
    };
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
}

/// Cheap size-pre-filter pass: read `archive_dir` once and keep entries
/// whose size matches one of `wanted_sizes`.
fn enumerate_candidates(
    archive_dir: &Path,
    wanted_sizes: &std::collections::HashSet<u64>,
) -> Vec<CandidateMeta> {
    let mut out: Vec<CandidateMeta> = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(archive_dir) else {
        return out;
    };
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
        out.push((path, size, mtime));
    }
    out
}

/// Phase 1: hash size-matching candidates through the persistent cache
/// in parallel. Returns the resulting present-by-hash map, the
/// per-thread cache-write list (merged + saved as a side effect), and
/// the cache-hit count.
fn hash_candidates_parallel(
    archive_dir: &Path,
    candidate_paths: Vec<CandidateMeta>,
    wanted_hashes: &std::collections::HashSet<String>,
) -> (HashMap<String, PathBuf>, Vec<CacheWrite>, usize) {
    let cache_at_entry = AsyncCacheFile::load(archive_dir);
    let candidate_paths = Arc::new(candidate_paths);
    let cache_at_entry = Arc::new(cache_at_entry);
    let wanted_hashes_arc = Arc::new(wanted_hashes.clone());
    let next_candidate = Arc::new(AtomicUsize::new(0));
    let present_by_hash: Arc<Mutex<HashMap<String, PathBuf>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let cache_writes: Arc<Mutex<Vec<CacheWrite>>> = Arc::new(Mutex::new(Vec::new()));
    let cache_hits = Arc::new(AtomicUsize::new(0));

    let worker_count = HASH_POOL_SIZE.min(candidate_paths.len().max(1));
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let candidate_paths = Arc::clone(&candidate_paths);
        let cache_at_entry = Arc::clone(&cache_at_entry);
        let wanted_hashes = Arc::clone(&wanted_hashes_arc);
        let next_candidate = Arc::clone(&next_candidate);
        let present_by_hash = Arc::clone(&present_by_hash);
        let cache_writes = Arc::clone(&cache_writes);
        let cache_hits = Arc::clone(&cache_hits);
        handles.push(thread::spawn(move || {
            hash_candidates_worker(
                &candidate_paths,
                &cache_at_entry,
                &wanted_hashes,
                &next_candidate,
                &present_by_hash,
                &cache_writes,
                &cache_hits,
            );
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    let writes = cache_writes.lock().expect("cache_writes mutex").clone();
    let mut cache = (*cache_at_entry).clone();
    let mut cache_dirty = false;
    for (path, size, mtime, hash) in &writes {
        if cache.record(path, *size, *mtime, hash) {
            cache_dirty = true;
        }
    }
    if cache_dirty {
        cache.save(archive_dir);
    }

    let present = present_by_hash
        .lock()
        .expect("present_by_hash mutex")
        .clone();
    let hits = cache_hits.load(Ordering::Relaxed);
    (present, writes, hits)
}

fn hash_candidates_worker(
    candidate_paths: &[CandidateMeta],
    cache_at_entry: &AsyncCacheFile,
    wanted_hashes: &std::collections::HashSet<String>,
    next_candidate: &AtomicUsize,
    present_by_hash: &Mutex<HashMap<String, PathBuf>>,
    cache_writes: &Mutex<Vec<CacheWrite>>,
    cache_hits: &AtomicUsize,
) {
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
                .or_insert_with(|| path.clone());
        }
    }
}

/// Phase 2: per-asset skip decisions in parallel. Each worker hashes is
/// already done; here we just consult `present_by_hash` and emit the
/// per-asset events as decisions are made.
fn decide_per_asset_parallel(
    assets: Vec<Step2UpdateAsset>,
    by_name: HashMap<String, ArchiveMeta>,
    present_by_hash: HashMap<String, PathBuf>,
    archive_dir: PathBuf,
    tx: &Sender<ArchiveSkipEvent>,
) -> Vec<AssetRecord> {
    let total_assets = assets.len();
    let assets = Arc::new(assets);
    let by_name = Arc::new(by_name);
    let present_by_hash = Arc::new(present_by_hash);
    let archive_dir = Arc::new(archive_dir);
    let next_asset = Arc::new(AtomicUsize::new(0));
    let asset_results: Arc<Mutex<Vec<AssetRecord>>> =
        Arc::new(Mutex::new(Vec::with_capacity(total_assets)));

    let worker_count = HASH_POOL_SIZE.min(total_assets);
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let assets = Arc::clone(&assets);
        let by_name = Arc::clone(&by_name);
        let present_by_hash = Arc::clone(&present_by_hash);
        let archive_dir = Arc::clone(&archive_dir);
        let next_asset = Arc::clone(&next_asset);
        let asset_results = Arc::clone(&asset_results);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            decide_per_asset_worker(
                &assets,
                &by_name,
                &present_by_hash,
                &archive_dir,
                &next_asset,
                &asset_results,
                &tx,
            );
        }));
    }
    for h in handles {
        let _ = h.join();
    }

    let mut recs = asset_results.lock().expect("asset_results mutex").clone();
    recs.sort_by_key(|(i, _, _)| *i);
    recs
}

fn decide_per_asset_worker(
    assets: &[Step2UpdateAsset],
    by_name: &HashMap<String, ArchiveMeta>,
    present_by_hash: &HashMap<String, PathBuf>,
    archive_dir: &Path,
    next_asset: &AtomicUsize,
    asset_results: &Mutex<Vec<AssetRecord>>,
    tx: &Sender<ArchiveSkipEvent>,
) {
    loop {
        let i = next_asset.fetch_add(1, Ordering::SeqCst);
        if i >= assets.len() {
            break;
        }
        let asset = &assets[i];
        let _ = tx.send(ArchiveSkipEvent::AssetHashStarted { index: i });
        let name = archive_file_name(asset);
        let (was_skipped, dest_display) = by_name
            .get(&name)
            .and_then(|meta| present_by_hash.get(&meta.hash))
            .map_or((false, None), |present_path| {
                let deterministic = archive_dir.join(&name);
                let placed = present_path == &deterministic
                    || match link_or_copy(present_path, &deterministic) {
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
                    };
                if placed {
                    (true, Some(deterministic.display().to_string()))
                } else {
                    (false, None)
                }
            });
        asset_results
            .lock()
            .expect("asset_results mutex")
            .push((i, was_skipped, asset.clone()));
        let _ = tx.send(ArchiveSkipEvent::AssetHashed {
            index: i,
            was_skipped,
            label: asset.label.clone(),
            dest_display,
        });
    }
}

/// Tally the per-asset records into the observational `SkipSummary` and
/// the streamer's `skipped_indices` (ordered by asset index).
fn build_summary(
    recs: &[AssetRecord],
    by_name: &HashMap<String, ArchiveMeta>,
    candidate_count: usize,
    cache_hits: usize,
) -> (SkipSummary, Vec<usize>) {
    let mut summary = SkipSummary::default();
    let mut skipped_indices: Vec<usize> = Vec::new();
    for (i, was_skipped, asset) in recs {
        let name = archive_file_name(asset);
        let has_expected = by_name.contains_key(&name);
        if *was_skipped {
            summary.skipped_present += 1;
            summary.skipped_assets.push(asset.clone());
            skipped_indices.push(*i);
        } else if has_expected {
            summary.missing_on_disk += 1;
        } else {
            summary.no_expected_hash += 1;
        }
    }
    summary.hashed_candidates = candidate_count;
    summary.cache_hits = cache_hits;
    (summary, skipped_indices)
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
        rx: &Receiver<ArchiveSkipEvent>,
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
        let (events, finished) = collect_events(&rx);
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
        let (events, finished) = collect_events(&rx);

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
            assets: vec![a],
        };
        let rx = start_async_archive_skip(input, vec![expected]);
        let (events, finished) = collect_events(&rx);

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
            assets: vec![a, b],
        };
        let rx = start_async_archive_skip(input, expected);
        let (_events, finished) = collect_events(&rx);
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
