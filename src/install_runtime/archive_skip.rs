// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::{Step2UpdateAsset, WizardState};
use crate::install_runtime::archive_store::{self, ARCHIVE_INDEX_FILENAME, INSTALL_LOCK_FILENAME};
use crate::registry::share_export::ArchiveMeta;

pub const HASH_CACHE_FILENAME: &str = ".bio-archive-hashcache.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HashCacheEntry {
    size: u64,

    mtime_nanos: Option<u128>,

    hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HashCache {
    entries: BTreeMap<String, HashCacheEntry>,
}

impl HashCache {
    fn path(archive_dir: &Path) -> PathBuf {
        archive_dir.join(HASH_CACHE_FILENAME)
    }

    #[must_use]
    pub fn load(archive_dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(archive_dir)).map_or_else(
            |_| Self::default(),
            |text| {
                serde_json::from_str(&text).unwrap_or_else(|err| {
                    warn!(
                        target = "orchestrator",
                        "archive hash cache unreadable ({err}); continuing empty \
                     (worst case: a one-time re-hash)"
                    );
                    Self::default()
                })
            },
        )
    }

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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().to_lowercase()
    }

    fn fresh_hash(&self, path: &Path, size: u64, mtime_nanos: Option<u128>) -> Option<&str> {
        let e = self.entries.get(&Self::key(path))?;
        if e.size != size {
            return None;
        }

        match (e.mtime_nanos, mtime_nanos) {
            (Some(a), Some(b)) if a != b => return None,
            _ => {}
        }
        Some(&e.hash)
    }

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkipSummary {
    pub skipped_present: usize,

    pub no_expected_hash: usize,

    pub missing_on_disk: usize,

    pub hashed_candidates: usize,

    pub cache_hits: usize,

    pub skipped_assets: Vec<Step2UpdateAsset>,
}

pub fn skip_present_archives(state: &mut WizardState, expected: &[ArchiveMeta]) -> SkipSummary {
    let mut summary = SkipSummary::default();
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return summary;
    }
    if expected.is_empty() {
        summary.no_expected_hash = state.step2.update_selected_update_assets.len();
        return summary;
    }

    let by_name: HashMap<&str, &ArchiveMeta> =
        expected.iter().map(|m| (m.name.as_str(), m)).collect();
    let present_by_hash = scan_present_archives(expected, &archive_dir, &mut summary);

    for asset in &state.step2.update_selected_update_assets {
        let name = archive_file_name(asset);
        let Some(meta) = by_name.get(name.as_str()) else {
            summary.no_expected_hash += 1;
            continue;
        };
        let Some(present_path) = present_by_hash.get(&meta.hash) else {
            summary.missing_on_disk += 1;
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
                        "present skip-source {} → {}: {err} (falling back to a \
                         normal download for this archive)",
                        present_path.display(),
                        deterministic.display()
                    );
                    false
                }
            }
        };
        if placed {
            summary.skipped_present += 1;
            summary.skipped_assets.push(asset.clone());

            state.step2.update_selected_downloaded_sources.push(format!(
                "{} -> {}",
                asset.label,
                deterministic.display()
            ));
        } else {
            summary.missing_on_disk += 1;
        }
    }
    summary
}

fn scan_present_archives(
    expected: &[ArchiveMeta],
    archive_dir: &Path,
    summary: &mut SkipSummary,
) -> HashMap<String, PathBuf> {
    let wanted_sizes: std::collections::HashSet<u64> = expected.iter().map(|m| m.size).collect();
    let wanted_hashes: std::collections::HashSet<&str> =
        expected.iter().map(|m| m.hash.as_str()).collect();
    let mut cache = HashCache::load(archive_dir);
    let mut cache_dirty = false;
    let mut present_by_hash = HashMap::new();
    if let Ok(read_dir) = std::fs::read_dir(archive_dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let Some((size, mtime)) = file_size_mtime(&path) else {
                continue;
            };
            if is_sidecar(&path) || !wanted_sizes.contains(&size) {
                continue;
            }
            let hash = match cached_or_hashed(&mut cache, &path, size, mtime, &mut cache_dirty) {
                Some((hash, cache_hit)) => {
                    if cache_hit {
                        summary.cache_hits += 1;
                    }
                    hash
                }
                None => continue,
            };
            summary.hashed_candidates += 1;
            if wanted_hashes.contains(hash.as_str()) {
                present_by_hash.entry(hash).or_insert(path);
            }
        }
    }
    if cache_dirty {
        cache.save(archive_dir);
    }
    present_by_hash
}

fn cached_or_hashed(
    cache: &mut HashCache,
    path: &Path,
    size: u64,
    mtime: Option<u128>,
    cache_dirty: &mut bool,
) -> Option<(String, bool)> {
    if let Some(hash) = cache.fresh_hash(path, size, mtime) {
        return Some((hash.to_string(), true));
    }
    match archive_store::hash_file(path) {
        Ok(hash) => {
            if cache.record(path, size, mtime, &hash) {
                *cache_dirty = true;
            }
            Some((hash, false))
        }
        Err(err) => {
            warn!(
                target = "orchestrator",
                "hash candidate {}: {err} (skipping it as a skip source)",
                path.display()
            );
            None
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerifySummary {
    pub verified: usize,

    pub mismatched: usize,

    pub unverifiable: usize,
}

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
            continue;
        };
        let path = archive_dir.join(&name);
        let Some((size, mtime)) = file_size_mtime(&path) else {
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
                String::new()
            }
        };
        if actual == meta.hash && !actual.is_empty() {
            if cache.record(&path, size, mtime, &actual) {
                cache_dirty = true;
            }
            summary.verified += 1;
        } else {
            if let Err(err) = std::fs::remove_file(&path) {
                warn!(
                    target = "orchestrator",
                    "delete hash-mismatched archive {}: {err} (still recorded \
                     failed so extract — which gates on .exists() — and the \
                     auto-build blocker skip it)",
                    path.display()
                );
            }

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

fn is_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some(HASH_CACHE_FILENAME | ARCHIVE_INDEX_FILENAME | INSTALL_LOCK_FILENAME)
    ) || path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
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
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("AMOD/AMOD.TP2", "github", "v1", "A.zip");
        let name = archive_file_name(&a);

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
        assert_eq!(
            state.step2.update_selected_update_assets.len(),
            1,
            "**Fix 1e** — the asset STAYS in the list so BIO's \
             build_extract_jobs finds it (the streamer bypasses by index)"
        );
        assert_eq!(
            s.skipped_assets.len(),
            1,
            "skipped_assets carries the exact asset (no diff needed by the caller)"
        );
        assert_eq!(s.skipped_assets[0].label, a.label);
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

        assert_eq!(
            state.step2.update_selected_downloaded_sources.len(),
            1,
            "Fix 1e — downloaded-sources pre-populated with the BIO-shaped entry"
        );
        assert_eq!(
            state.step2.update_selected_downloaded_sources[0],
            format!("{} -> {}", a.label, deterministic.display()),
            "pre-populated entry is BIO-verbatim shape"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn same_name_different_hash_is_not_skipped_and_coexists() {
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD.zip");
        let name = archive_file_name(&a);

        let on_disk = archive_dir.join(archive_store::stored_filename(&name, "v1"));
        std::fs::write(&on_disk, b"VERSION-1-CONTENT").unwrap();

        let want_v2_bytes = b"VERSION-2-DIFFERENT-CONTENT";
        let expected = ArchiveMeta {
            name,
            size: want_v2_bytes.len() as u64,

            hash: {
                let p = archive_dir.join("scratch_v2");
                std::fs::write(&p, want_v2_bytes).unwrap();
                let h = archive_store::hash_file(&p).unwrap();
                std::fs::remove_file(&p).unwrap();
                h
            },
        };

        state.step2.update_selected_update_assets = vec![a];
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
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        let name = archive_file_name(&a);
        let bytes = b"WANTED-ARCHIVE-CONTENT";
        std::fs::write(archive_dir.join(&name), bytes).unwrap();

        std::fs::write(archive_dir.join("unrelated-huge.bin"), vec![9u8; 4096]).unwrap();
        let expected = meta_for(&archive_dir, &a);

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

        state.step2.update_selected_update_assets = vec![a];
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

        assert_eq!(
            cache.fresh_hash(&p, size1 + 1, mtime1),
            None,
            "a size change invalidates the entry"
        );

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

        let v = verify_downloaded_archives(
            &mut state,
            std::slice::from_ref(&expected),
            std::slice::from_ref(&a),
        );
        assert_eq!(v.verified, 1);
        assert_eq!(v.mismatched, 0);
        assert!(
            state
                .step2
                .update_selected_download_failed_sources
                .is_empty(),
            "a matching download is not recorded failed"
        );

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
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("BAD/BAD.TP2", "gh", "v9", "BAD.zip");
        let name = archive_file_name(&a);

        std::fs::write(archive_dir.join(&name), b"CORRUPT-OR-TAMPERED").unwrap();
        let expected = ArchiveMeta {
            name: name.clone(),

            size: "CORRUPT-OR-TAMPERED".len() as u64,
            hash: "00000000000000000000000000000000".to_string(),
        };

        let v = verify_downloaded_archives(&mut state, &[expected], std::slice::from_ref(&a));
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

        assert!(
            HashCache::load(&archive_dir).entries.is_empty(),
            "a hash-mismatched archive is never registered in the cache"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn verify_leaves_unverifiable_alone() {
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("A/A.TP2", "gh", "v1", "A.zip");
        std::fs::write(archive_dir.join(archive_file_name(&a)), b"x").unwrap();

        let v = verify_downloaded_archives(&mut state, &[], std::slice::from_ref(&a));
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
    fn fix_1e_all_cached_keeps_every_asset_for_extract_and_prepopulates_each() {
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("AMOD/AMOD.TP2", "github", "v1", "A.zip");
        let b = asset("BMOD/BMOD.TP2", "weasel", "v2", "B.zip");
        let c = asset("CMOD/CMOD.TP2", "morpheus", "v3", "C.zip");

        let bytes_a = b"A-ARCHIVE-BYTES".to_vec();
        let bytes_b = b"B-DIFFERENT-BYTES".to_vec();
        let bytes_c = b"C-THIRD-BYTES-LONGER-X".to_vec();
        std::fs::write(archive_dir.join(archive_file_name(&a)), &bytes_a).unwrap();
        std::fs::write(archive_dir.join(archive_file_name(&b)), &bytes_b).unwrap();
        std::fs::write(archive_dir.join(archive_file_name(&c)), &bytes_c).unwrap();
        let expected = vec![
            meta_for(&archive_dir, &a),
            meta_for(&archive_dir, &b),
            meta_for(&archive_dir, &c),
        ];
        state.step2.update_selected_update_assets = vec![a.clone(), b.clone(), c.clone()];

        let s = skip_present_archives(&mut state, &expected);

        assert_eq!(s.skipped_present, 3, "all 3 archives skipped");
        assert_eq!(
            state.step2.update_selected_update_assets.len(),
            3,
            "Fix 1e — all assets STAY in the list for BIO's extract"
        );
        assert_eq!(s.skipped_assets.len(), 3, "all 3 in skipped_assets");

        assert_eq!(state.step2.update_selected_downloaded_sources.len(), 3);
        for asset in [&a, &b, &c] {
            let dest = archive_dir.join(archive_file_name(asset));
            let expected_entry = format!("{} -> {}", asset.label, dest.display());
            assert!(
                state
                    .step2
                    .update_selected_downloaded_sources
                    .contains(&expected_entry),
                "pre-populated entry missing for {}: {expected_entry}",
                asset.label
            );
        }
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
