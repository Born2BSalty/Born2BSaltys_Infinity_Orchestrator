// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::WizardState;

pub const INSTALL_LOCK_FILENAME: &str = ".bio-install-lock.json";

pub const ARCHIVE_INDEX_FILENAME: &str = ".bio-archive-index.json";

#[derive(Debug, Clone, Copy)]
struct Fnv1a128 {
    state: u128,
}

impl Fnv1a128 {
    const OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;

    const fn new() -> Self {
        Self {
            state: Self::OFFSET,
        }
    }

    fn update(&mut self, chunk: &[u8]) {
        let mut h = self.state;
        for &b in chunk {
            h ^= u128::from(b);
            h = h.wrapping_mul(Self::PRIME);
        }
        self.state = h;
    }

    fn finish_hex(self) -> String {
        format!("{:032x}", self.state)
    }
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Fnv1a128::new();
    let mut buf = vec![0u8; 64 * 1024].into_boxed_slice();
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

#[must_use]
pub fn stored_filename(name: &str, hash: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{hash}.{ext}"),
        _ => format!("{name}.{hash}"),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchiveIndex {
    pub archives: BTreeMap<String, Vec<String>>,
}

impl ArchiveIndex {
    fn path(archive_dir: &Path) -> PathBuf {
        archive_dir.join(ARCHIVE_INDEX_FILENAME)
    }

    fn load(archive_dir: &Path) -> Self {
        let p = Self::path(archive_dir);
        std::fs::read_to_string(&p).map_or_else(
            |_| Self::default(),
            |text| {
                serde_json::from_str(&text).unwrap_or_else(|err| {
                    warn!(
                        target = "orchestrator",
                        "archive index {} unreadable ({err}); continuing with an empty index \
                     (worst case: a redundant re-download)",
                        p.display()
                    );
                    Self::default()
                })
            },
        )
    }

    fn save(&self, archive_dir: &Path) {
        let p = Self::path(archive_dir);
        let tmp = p.with_extension("json.tmp");
        let json = match serde_json::to_string_pretty(self) {
            Ok(j) => j,
            Err(err) => {
                warn!(target = "orchestrator", "serialize archive index: {err}");
                return;
            }
        };
        if let Err(err) = std::fs::write(&tmp, json).and_then(|()| std::fs::rename(&tmp, &p)) {
            warn!(
                target = "orchestrator",
                "persist archive index {}: {err} (non-fatal — dedupe cache only)",
                p.display()
            );
        }
    }

    fn record(&mut self, name: &str, hash: &str) -> bool {
        let hashes = self.archives.entry(name.to_string()).or_default();
        if hashes.iter().any(|h| h == hash) {
            false
        } else {
            hashes.push(hash.to_string());
            hashes.sort();
            true
        }
    }

    fn has(&self, name: &str, hash: &str) -> bool {
        self.archives
            .get(name)
            .is_some_and(|hs| hs.iter().any(|h| h == hash))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallArchiveLock {
    pub resolved: BTreeMap<String, String>,
}

impl InstallArchiveLock {
    fn path(destination: &Path) -> PathBuf {
        destination.join(INSTALL_LOCK_FILENAME)
    }

    #[must_use]
    pub fn load(destination: &str) -> Self {
        let p = Self::path(Path::new(destination.trim()));
        std::fs::read_to_string(&p).map_or_else(
            |_| Self::default(),
            |text| serde_json::from_str(&text).unwrap_or_default(),
        )
    }

    fn save(&self, destination: &Path) {
        let p = Self::path(destination);
        let tmp = p.with_extension("json.tmp");
        let json = match serde_json::to_string_pretty(self) {
            Ok(j) => j,
            Err(err) => {
                warn!(target = "orchestrator", "serialize install lock: {err}");
                return;
            }
        };
        if let Err(err) = std::fs::create_dir_all(p.parent().unwrap_or_else(|| Path::new(".")))
            .and_then(|()| std::fs::write(&tmp, json))
            .and_then(|()| std::fs::rename(&tmp, &p))
        {
            warn!(
                target = "orchestrator",
                "persist install lock {}: {err} (non-fatal — reproduce accelerator only)",
                p.display()
            );
        }
    }

    pub fn hash_for(&self, name: &str) -> Option<&str> {
        self.resolved.get(name).map(String::as_str)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngestSummary {
    pub stored: usize,

    pub deduped: usize,

    pub missing: usize,
}

/// Place `src` at `dst` as a hardlink when possible; fall back to a full
/// byte-copy only when hardlinking is not available (cross-volume,
/// non-NTFS, antivirus, permissions, etc.). Two filesystem names share
/// one set of bytes on the hardlink path, so re-staging a known archive
/// into BIO's deterministic extract path costs zero extra disk for the
/// common single-volume case.
///
/// A hardlink failure surfaces via `tracing::warn` before falling back so
/// any future skew on a different filesystem / environment is diagnostic,
/// not silent.
fn link_or_copy(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if dst.exists() {
        let _ = std::fs::remove_file(dst);
    }
    match std::fs::hard_link(src, dst) {
        Ok(()) => Ok(()),
        Err(err) => {
            warn!(
                target = "orchestrator",
                "hardlink {} -> {} failed: {err}; falling back to byte-copy \
                 (no dedupe — disk usage doubles)",
                src.display(),
                dst.display()
            );
            std::fs::copy(src, dst).map(|_| ())
        }
    }
}

pub fn stage_known_archives(state: &mut WizardState, destination: &str) -> usize {
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return 0;
    }
    let lock = InstallArchiveLock::load(destination);
    if lock.resolved.is_empty() {
        return 0;
    }
    let index = ArchiveIndex::load(&archive_dir);

    let mut satisfied = 0usize;
    let assets = std::mem::take(&mut state.step2.update_selected_update_assets);
    let mut kept = Vec::with_capacity(assets.len());
    for asset in assets {
        let name = archive_file_name(&asset);
        let Some(hash) = lock.hash_for(&name) else {
            kept.push(asset);
            continue;
        };
        if !index.has(&name, hash) {
            kept.push(asset);
            continue;
        }
        let stored = archive_dir.join(stored_filename(&name, hash));
        let deterministic = archive_dir.join(&name);
        match link_or_copy(&stored, &deterministic) {
            Ok(()) => {
                satisfied += 1;
            }
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "stage stored archive {} \u{2192} {}: {err} (falling back to download)",
                    stored.display(),
                    deterministic.display()
                );
                kept.push(asset);
            }
        }
    }
    state.step2.update_selected_update_assets = kept;
    satisfied
}

pub fn ingest_downloaded_archives(
    state: &WizardState,
    destination: &str,
    logical_names: &[String],
) -> IngestSummary {
    let mut summary = IngestSummary::default();
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() || logical_names.is_empty() {
        return summary;
    }
    let mut index = ArchiveIndex::load(&archive_dir);
    let mut lock = InstallArchiveLock::load(destination);
    let mut index_dirty = false;
    let mut lock_dirty = false;

    for name in logical_names {
        let deterministic = archive_dir.join(name);
        if !deterministic.exists() {
            summary.missing += 1;
            continue;
        }
        let hash = match hash_file(&deterministic) {
            Ok(h) => h,
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "hash archive {}: {err} (skipping content-addressing for it)",
                    deterministic.display()
                );
                summary.missing += 1;
                continue;
            }
        };

        if lock.resolved.get(name).map(String::as_str) != Some(hash.as_str()) {
            lock.resolved.insert(name.clone(), hash.clone());
            lock_dirty = true;
        }

        let stored = archive_dir.join(stored_filename(name, &hash));
        if stored.exists() && index.has(name, &hash) {
            summary.deduped += 1;
            continue;
        }

        if let Err(err) = link_or_copy(&deterministic, &stored) {
            warn!(
                target = "orchestrator",
                "store content-addressed copy {} \u{2192} {}: {err} \
                 (the deterministic copy is intact; dedupe just won't help next time)",
                deterministic.display(),
                stored.display()
            );
            continue;
        }
        if index.record(name, &hash) {
            index_dirty = true;
        }
        summary.stored += 1;
    }

    if index_dirty {
        index.save(&archive_dir);
    }
    if lock_dirty {
        lock.save(Path::new(destination.trim()));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Step2UpdateAsset;

    fn td() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_archive_store_test_{}_{}",
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

    #[test]
    fn fnv1a128_is_deterministic_and_distinguishes_content() {
        let dir = td();
        let a = dir.join("a.bin");
        let b = dir.join("b.bin");
        std::fs::write(&a, b"modlist-A v1.3 bytes").unwrap();
        std::fs::write(&b, b"modlist-B v1.4 bytes").unwrap();
        let ha1 = hash_file(&a).unwrap();
        let ha2 = hash_file(&a).unwrap();
        let hb = hash_file(&b).unwrap();
        assert_eq!(ha1, ha2, "same bytes ⇒ identical hash on every call");
        assert_ne!(ha1, hb, "different bytes ⇒ different hash");
        assert_eq!(ha1.len(), 32, "128-bit ⇒ 32 hex chars");

        let empty = dir.join("e.bin");
        std::fs::write(&empty, b"").unwrap();
        assert_eq!(
            hash_file(&empty).unwrap(),
            "6c62272e07bb014262b821756295c58d"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stored_filename_encodes_hash_so_versions_coexist() {
        assert_eq!(
            stored_filename("eet__gh__v13.zip", "deadbeef"),
            "eet__gh__v13.zip".replace(".zip", ".deadbeef.zip")
        );

        assert_ne!(
            stored_filename("m.zip", "aaaa"),
            stored_filename("m.zip", "bbbb")
        );

        assert_eq!(stored_filename("noext", "ff"), "noext.ff");
    }

    #[test]
    fn ingest_stores_then_dedupes_then_coexists() {
        let archive_dir = td();
        let dest = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let dest_s = dest.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        let name = archive_file_name(&a);

        std::fs::write(archive_dir.join(&name), b"VERSION-1-CONTENT").unwrap();
        let s1 = ingest_downloaded_archives(&state, &dest_s, std::slice::from_ref(&name));
        assert_eq!(
            (s1.stored, s1.deduped, s1.missing),
            (1, 0, 0),
            "first ⇒ stored"
        );
        let h1 = hash_file(&archive_dir.join(&name)).unwrap();
        assert!(archive_dir.join(stored_filename(&name, &h1)).exists());

        assert_eq!(
            InstallArchiveLock::load(&dest_s).hash_for(&name),
            Some(h1.as_str())
        );

        let s2 = ingest_downloaded_archives(&state, &dest_s, std::slice::from_ref(&name));
        assert_eq!(
            (s2.stored, s2.deduped),
            (0, 1),
            "identical re-download ⇒ deduped"
        );

        std::fs::write(archive_dir.join(&name), b"VERSION-2-DIFFERENT").unwrap();
        let s3 = ingest_downloaded_archives(&state, &dest_s, std::slice::from_ref(&name));
        assert_eq!(
            (s3.stored, s3.deduped),
            (1, 0),
            "different content ⇒ stored (coexist)"
        );
        let h2 = hash_file(&archive_dir.join(&name)).unwrap();
        assert_ne!(h1, h2);
        assert!(
            archive_dir.join(stored_filename(&name, &h1)).exists(),
            "v1 still there"
        );
        assert!(
            archive_dir.join(stored_filename(&name, &h2)).exists(),
            "v2 also there"
        );
        let idx = ArchiveIndex::load(&archive_dir);
        assert_eq!(
            idx.archives.get(&name).map(Vec::len),
            Some(2),
            "both hashes indexed"
        );

        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn stage_known_archives_removes_asset_and_no_redownload() {
        let archive_dir = td();
        let dest_a = td();
        let dest_b = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        let name = archive_file_name(&a);

        std::fs::write(archive_dir.join(&name), b"SHARED-CONTENT").unwrap();
        ingest_downloaded_archives(
            &state,
            &dest_a.to_string_lossy(),
            std::slice::from_ref(&name),
        );
        let h = hash_file(&archive_dir.join(&name)).unwrap();

        std::fs::remove_file(archive_dir.join(&name)).unwrap();

        let mut lock_b = InstallArchiveLock::default();
        lock_b.resolved.insert(name.clone(), h);
        lock_b.save(Path::new(dest_b.to_string_lossy().trim()));

        state.step2.update_selected_update_assets = vec![a];
        let satisfied = stage_known_archives(&mut state, &dest_b.to_string_lossy());

        assert_eq!(satisfied, 1, "the asset was satisfied from the store");
        assert!(
            state.step2.update_selected_update_assets.is_empty(),
            "the asset was dropped ⇒ BIO will NOT re-download it (cross-modlist dedupe)"
        );
        assert!(
            archive_dir.join(&name).exists(),
            "the stored copy was placed at BIO's deterministic extract path"
        );
        assert_eq!(
            std::fs::read(archive_dir.join(&name)).unwrap(),
            b"SHARED-CONTENT",
            "extract gets the exact content modlist B resolved"
        );

        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest_a);
        let _ = std::fs::remove_dir_all(&dest_b);
    }

    #[test]
    fn stage_keeps_asset_when_nothing_recorded() {
        let archive_dir = td();
        let dest = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        state.step2.update_selected_update_assets = vec![a];
        let satisfied = stage_known_archives(&mut state, &dest.to_string_lossy());
        assert_eq!(satisfied, 0);
        assert_eq!(
            state.step2.update_selected_update_assets.len(),
            1,
            "nothing recorded ⇒ asset kept for a normal download"
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest);
    }

    /// Returns true when `a` and `b` share the same inode (hardlink) on
    /// the active filesystem. Detection is by behavior, not API: a
    /// rewrite of `a` propagates to `b` only if they point at the same
    /// bytes on disk. Stable on every platform — no `file_index` /
    /// unstable feature.
    #[cfg(windows)]
    fn share_inode(a: &Path, b: &Path) -> bool {
        let original = std::fs::read(a).unwrap();
        let probe = b"__BIO_TEST_HARDLINK_PROBE__";
        std::fs::write(a, probe).unwrap();
        let b_after = std::fs::read(b).unwrap_or_default();
        std::fs::write(a, &original).unwrap();
        b_after == probe
    }

    #[test]
    fn link_or_copy_produces_content_equivalent_dst() {
        let dir = td();
        let src = dir.join("src.bin");
        let dst = dir.join("dst.bin");
        std::fs::write(&src, b"the-payload-bytes").unwrap();
        link_or_copy(&src, &dst).unwrap();
        assert_eq!(std::fs::read(&src).unwrap(), std::fs::read(&dst).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn link_or_copy_hardlinks_on_same_volume_so_one_inode_two_names() {
        let dir = td();
        let src = dir.join("src.bin");
        let dst = dir.join("dst.bin");
        std::fs::write(&src, b"the-payload-bytes").unwrap();
        link_or_copy(&src, &dst).unwrap();
        assert!(
            share_inode(&src, &dst),
            "same NTFS volume ⇒ hardlink ⇒ src + dst share one set of bytes \
             (truncating src reports zero length on dst)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ingest_does_not_duplicate_bytes_on_first_store_hardlink_path() {
        let archive_dir = td();
        let dest = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let dest_s = dest.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        let name = archive_file_name(&a);

        std::fs::write(archive_dir.join(&name), b"DEDUPED-CONTENT-BYTES").unwrap();
        let s = ingest_downloaded_archives(&state, &dest_s, std::slice::from_ref(&name));
        assert_eq!(s.stored, 1);
        let h = hash_file(&archive_dir.join(&name)).unwrap();
        let stored = archive_dir.join(stored_filename(&name, &h));
        let deterministic = archive_dir.join(&name);
        assert!(stored.exists());
        assert!(deterministic.exists());
        assert_eq!(
            std::fs::read(&stored).unwrap(),
            std::fs::read(&deterministic).unwrap()
        );
        #[cfg(windows)]
        {
            assert!(
                share_inode(&deterministic, &stored),
                "ingest hardlinks the content-addressed copy ⇒ no full duplicate \
                 (truncating the deterministic file should also empty the stored one)"
            );
        }
        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn stage_known_archives_hardlinks_into_deterministic_path() {
        let archive_dir = td();
        let dest_a = td();
        let dest_b = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        let name = archive_file_name(&a);

        std::fs::write(archive_dir.join(&name), b"SHARED-CONTENT").unwrap();
        ingest_downloaded_archives(
            &state,
            &dest_a.to_string_lossy(),
            std::slice::from_ref(&name),
        );
        let h = hash_file(&archive_dir.join(&name)).unwrap();

        std::fs::remove_file(archive_dir.join(&name)).unwrap();

        let mut lock_b = InstallArchiveLock::default();
        lock_b.resolved.insert(name.clone(), h.clone());
        lock_b.save(Path::new(dest_b.to_string_lossy().trim()));

        state.step2.update_selected_update_assets = vec![a];
        let satisfied = stage_known_archives(&mut state, &dest_b.to_string_lossy());
        assert_eq!(satisfied, 1);

        let stored = archive_dir.join(stored_filename(&name, &h));
        let deterministic = archive_dir.join(&name);
        assert!(stored.exists());
        assert!(deterministic.exists());
        #[cfg(windows)]
        {
            assert!(
                share_inode(&stored, &deterministic),
                "stage hardlinks the stored copy at BIO's extract path ⇒ \
                 no full duplicate"
            );
        }
        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest_a);
        let _ = std::fs::remove_dir_all(&dest_b);
    }

    #[test]
    fn corrupt_index_and_lock_degrade_to_empty_not_panic() {
        let archive_dir = td();
        let dest = td();
        std::fs::write(archive_dir.join(ARCHIVE_INDEX_FILENAME), b"{ not json").unwrap();
        std::fs::write(
            Path::new(dest.to_string_lossy().trim()).join(INSTALL_LOCK_FILENAME),
            b"garbage",
        )
        .unwrap();

        assert!(ArchiveIndex::load(&archive_dir).archives.is_empty());
        assert!(
            InstallArchiveLock::load(&dest.to_string_lossy())
                .resolved
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&dest);
    }
}
