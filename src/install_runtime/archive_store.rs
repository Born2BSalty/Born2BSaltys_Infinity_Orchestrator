// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::archive_store` — Phase 7 P7.T17 piece 2
// (SPEC §13.12a "Content-addressed archives (the global Mods-archive
// uniqueness rule)").
//
// The global Mods-archive folder (Settings → Paths §11.2) holds **all**
// downloaded archives for **all** modlists. Because it is global + shared,
// two modlists may need different versions of the same upstream-named
// archive (modlist A → mod v1.3, modlist B → mod v1.4, identical
// filename). Per SPEC §13.12a the store is therefore **content-addressed**:
//
//   - On download, hash the archive's content. Same logical name **and**
//     matching hash as an existing archive ⇒ reuse (cross-modlist dedupe,
//     no re-download). Same name, **different** hash ⇒ **both coexist**,
//     stored under a name that encodes the hash so they never collide.
//   - Each modlist records, in its lock/pinned data, the exact hash it
//     resolved — so extraction for a given install always targets *that*
//     archive, and re-install / reproduce-from-code uses the same content.
//   - This is a **net-new orchestrator staging layer that WRAPS** BIO's
//     existing download + extract. `bio::app::app_step2_update_download` /
//     `app_step2_update_extract` are reused **unchanged — no BIO
//     modification**.
//
// **How it interposes with zero BIO edit (the boundary mechanism).** BIO's
// `download_one_asset` always re-creates the archive at the *deterministic*
// path `archive_dir.join(archive_file_name(asset))` (`app_step2_update_
// download.rs:152-166`); BIO's extract reads from that **same**
// deterministic path and skips an asset whose archive is absent
// (`build_extract_jobs` → `archive_path.exists()`,
// `app_step2_update_extract_plan.rs:48-51`). The orchestrator therefore
// interposes at the *download/extract boundary* — around the
// reused-unchanged workers — in two pure steps:
//
//   1. **`stage_known_archives` (before download).** For every asset whose
//      content-addressed copy already exists in the store under THIS
//      modlist's resolved-or-recorded hash, drop that asset from
//      `update_selected_update_assets` (so BIO does **not** re-download
//      it) and copy the stored content-addressed file onto the
//      deterministic extract path (so BIO's extract picks it up via the
//      `archive_path.exists()` gate). → cross-modlist dedupe + no
//      re-download, with the engine untouched.
//   2. **`ingest_downloaded_archives` (after download).** Hash each
//      just-downloaded archive (at its deterministic path), copy it into
//      the content-addressed store under `<name>.<hash>.<ext>`, and record
//      `logical_name → hash` for THIS modlist in the per-install lock
//      sidecar (`<dest>/.bio-install-lock.json`) + the global store index
//      (`<archive_dir>/.bio-archive-index.json`). Same name+hash already
//      in the store ⇒ no duplicate copy (dedupe); different hash ⇒ a
//      distinct stored file (coexist).
//
// Both steps operate purely on the orchestrator-owned asset list + the
// archive dir + two net-new JSON sidecars. **No BIO source is touched**
// (the directive's hard constraint for this run) — the download/extract
// engine is composed around, never forked.
//
// **Hash function.** No `sha2`/`blake3`/etc. is a `Cargo.toml` dep and the
// run constraint forbids adding one; a `std::hash::DefaultHasher` is
// explicitly unsuitable (SipHash with a per-process random seed ⇒ not
// stable across runs, so a reproduce-from-code on a later launch would
// never match). Per the brief's authorized fallback ("a small std-based
// content hash is acceptable"), this uses a **deterministic, seedless
// 128-bit FNV-1a** over the file bytes — version-independent, stable
// across runs/processes, and far beyond collision-safe for the archive-
// identity scope (distinguishing versions of the same logical archive).
// It is an *integrity/identity* hash, not a security primitive (none is
// needed — the threat model is "two versions collide", not adversarial
// preimage).
//
// SPEC: §13.12a, §13.12 #5.

// rationale: the index/lock are simple serde maps; `#[must_use]` on
// trivial accessors + `Self`/`const fn` churn add noise (Cat 3).
#![allow(
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::use_self
)]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::app_step2_update_download::archive_file_name;
use crate::app::state::WizardState;

/// The per-install lock sidecar filename (written inside the modlist's
/// destination — a per-install dir the orchestrator owns, SPEC §13.12a:
/// "Each modlist records, in its lock/pinned data, the exact hash it
/// resolved"). Survives a crash next to the install (the same durability
/// rationale as `modlist-import-code.txt`, SPEC §13.13).
pub const INSTALL_LOCK_FILENAME: &str = ".bio-install-lock.json";

/// The global store-index sidecar filename (written inside the global
/// Mods-archive folder). Maps every logical archive name → the set of
/// content hashes stored for it (so "same name / different hash coexist"
/// is bookkept and a future modlist can dedupe against a prior one's
/// download).
pub const ARCHIVE_INDEX_FILENAME: &str = ".bio-archive-index.json";

/// Stable, seedless **128-bit FNV-1a** of `bytes` rendered as 32 lowercase
/// hex chars. Deterministic across runs/processes/Rust versions (unlike
/// `DefaultHasher`) — required so a reproduce-from-code on a later launch
/// resolves the *same* archive. Streaming-friendly (the caller feeds a
/// file in chunks via [`hash_file`]).
#[derive(Debug, Clone, Copy)]
struct Fnv1a128 {
    state: u128,
}

impl Fnv1a128 {
    // The canonical 128-bit FNV-1a offset basis + prime (FNV spec). Fixed
    // constants ⇒ the hash is identical on every machine/run.
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013B;

    fn new() -> Self {
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

/// Content hash of a file at `path` (32 hex chars). Streams in 64 KiB
/// chunks so a multi-hundred-MB mod archive never loads wholly into RAM.
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Fnv1a128::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

/// `<stem>.<hash>.<ext>` — the content-addressed stored filename for a
/// logical archive `name` (e.g. `eet__github__v13.zip`) at content
/// `hash`. Two versions of `eet__github__v13.zip` with different content
/// get two distinct stored files; an identical re-download maps to the
/// same one (dedupe). A name with no extension keeps the hash as a
/// trailing segment.
#[must_use]
pub fn stored_filename(name: &str, hash: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{hash}.{ext}"),
        _ => format!("{name}.{hash}"),
    }
}

/// The global store index — `logical_name → [hash, …]` (sorted maps for
/// a stable, diffable on-disk form). Lives at
/// `<archive_dir>/.bio-archive-index.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArchiveIndex {
    /// `logical archive name` → the set of content hashes stored for it.
    /// A `BTreeMap` + sorted `Vec` keeps the file deterministic.
    pub archives: BTreeMap<String, Vec<String>>,
}

impl ArchiveIndex {
    fn path(archive_dir: &Path) -> PathBuf {
        archive_dir.join(ARCHIVE_INDEX_FILENAME)
    }

    /// Load the index from `archive_dir`. A missing / unreadable / corrupt
    /// index is **not** fatal — it degrades to "nothing known yet" (the
    /// worst case is a redundant re-download, never a wrong archive),
    /// matching SPEC §13.14's lighter policy for reconstructable caches
    /// (this is a dedupe accelerator, not irreplaceable user data).
    fn load(archive_dir: &Path) -> Self {
        let p = Self::path(archive_dir);
        match std::fs::read_to_string(&p) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
                warn!(
                    target = "orchestrator",
                    "archive index {} unreadable ({err}); continuing with an empty index \
                     (worst case: a redundant re-download)",
                    p.display()
                );
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Persist atomically (temp-file-then-rename, the same discipline as
    /// `RegistryStore::save`). A write failure is logged, not fatal — the
    /// archives themselves are already on disk; only the dedupe
    /// accelerator is stale.
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

    /// Record that `name` has content `hash` stored. Idempotent (a repeat
    /// is a no-op). Returns `true` if this `(name, hash)` pair was new.
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

    /// `true` if a content-addressed copy of `name` at `hash` is recorded
    /// as stored.
    fn has(&self, name: &str, hash: &str) -> bool {
        self.archives
            .get(name)
            .is_some_and(|hs| hs.iter().any(|h| h == hash))
    }
}

/// The per-install lock — `logical_name → resolved content hash` for THIS
/// modlist (SPEC §13.12a "Each modlist records ... the exact hash it
/// resolved"). Written inside the modlist's destination so a re-install /
/// reproduce-from-code re-resolves the **same** content even if the global
/// index was pruned. Survives a crash next to the install.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InstallArchiveLock {
    /// `logical archive name` → the exact content hash this modlist
    /// resolved/used for it.
    pub resolved: BTreeMap<String, String>,
}

impl InstallArchiveLock {
    fn path(destination: &Path) -> PathBuf {
        destination.join(INSTALL_LOCK_FILENAME)
    }

    /// Load the per-install lock from the destination (missing/corrupt ⇒
    /// empty: a fresh install has resolved nothing yet; the worst case is
    /// a redundant re-download).
    pub fn load(destination: &str) -> Self {
        let p = Self::path(Path::new(destination.trim()));
        match std::fs::read_to_string(&p) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist atomically into the destination. A write failure is logged,
    /// not fatal — the install itself does not depend on the lock; it only
    /// accelerates a future reproduce.
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
        if let Err(err) = std::fs::create_dir_all(p.parent().unwrap_or(Path::new(".")))
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

    /// The recorded hash this modlist resolved for logical archive `name`,
    /// if any.
    pub fn hash_for(&self, name: &str) -> Option<&str> {
        self.resolved.get(name).map(String::as_str)
    }
}

/// Outcome of [`ingest_downloaded_archives`] — what the live Downloading
/// screen / the run report wants to know about the content-addressing
/// pass (counts only; purely observational).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IngestSummary {
    /// Archives newly stored in the content-addressed store this pass.
    pub stored: usize,
    /// Archives that were already in the store at the same content hash
    /// (cross-modlist / re-attempt dedupe — no duplicate copy made).
    pub deduped: usize,
    /// Archives present in the asset list but absent on disk after
    /// download (a failed/partial download — recorded so the live screen
    /// can reflect it; not fatal here, BIO's own failed-source tracking
    /// drives the auto-build blocker).
    pub missing: usize,
}

/// **Before download (boundary step 1).** For every selected update asset
/// whose content-addressed copy already exists in the store at the hash
/// THIS modlist resolved (recorded in the per-install lock from a prior
/// attempt/reproduce), copy the stored file onto BIO's deterministic
/// extract path and **remove the asset from
/// `update_selected_update_assets`** so BIO does not re-download it
/// (cross-modlist dedupe + no re-download — SPEC §13.12a). Returns the
/// number of assets satisfied from the store.
///
/// Assets with no recorded hash (first-ever resolution) are left in the
/// list for BIO to download normally — `ingest_downloaded_archives` then
/// records their hash so the *next* modlist/attempt can dedupe.
///
/// Zero BIO edit: this only mutates the orchestrator-owned asset list +
/// places a file at the path BIO's unchanged extract already reads.
pub fn stage_known_archives(state: &mut WizardState, destination: &str) -> usize {
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return 0;
    }
    let lock = InstallArchiveLock::load(destination);
    if lock.resolved.is_empty() {
        return 0; // nothing resolved yet for this modlist — let BIO fetch.
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
            // Recorded but not actually in the store (index pruned / file
            // deleted) — fall back to a normal download.
            kept.push(asset);
            continue;
        }
        let stored = archive_dir.join(stored_filename(&name, hash));
        let deterministic = archive_dir.join(&name);
        match std::fs::copy(&stored, &deterministic) {
            Ok(_) => {
                // BIO's extract will now find `deterministic` via its
                // `archive_path.exists()` gate; drop the asset so BIO does
                // not re-download it.
                satisfied += 1;
            }
            Err(err) => {
                warn!(
                    target = "orchestrator",
                    "stage stored archive {} → {}: {err} (falling back to download)",
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

/// **After download (boundary step 2).** Hash every archive BIO just wrote
/// at its deterministic path, copy each into the content-addressed store
/// (`<name>.<hash>.<ext>` — skipped if an identical content-addressed copy
/// already exists ⇒ dedupe / coexist), and record `name → hash` for THIS
/// modlist in the per-install lock + the global index (SPEC §13.12a:
/// "Each modlist records ... the exact hash it resolved"). Idempotent
/// across re-attempts of the same install.
///
/// Hashes the **resolved asset set** (`update_selected_update_assets` ∪
/// the assets `stage_known_archives` already satisfied — passing the full
/// pre-stage list is the caller's job; this function only needs the
/// archive dir + the names). Zero BIO edit — read-only on BIO's archive
/// dir + the two orchestrator sidecars.
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

        // Record this modlist's resolved hash for the logical name (the
        // SPEC §13.12a per-install lock).
        if lock.resolved.get(name).map(String::as_str) != Some(hash.as_str()) {
            lock.resolved.insert(name.clone(), hash.clone());
            lock_dirty = true;
        }

        let stored = archive_dir.join(stored_filename(name, &hash));
        if stored.exists() && index.has(name, &hash) {
            // Same name + same content already in the store — dedupe (a
            // re-attempt, or a different modlist that resolved the same
            // version). No duplicate copy.
            summary.deduped += 1;
            continue;
        }
        // Same name + different hash ⇒ `stored` is a *distinct* file
        // (the hash is in the name) ⇒ both versions coexist.
        if let Err(err) = std::fs::copy(&deterministic, &stored) {
            warn!(
                target = "orchestrator",
                "store content-addressed copy {} → {}: {err} \
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
        // DATA-LOSS-safe: a unique temp dir; this module never binds the
        // real `%APPDATA%\bio\` (it operates on an arbitrary archive dir
        // + destination — here throwaway temp dirs).
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
        // Stable across calls (the reproduce-from-code requirement) +
        // different content ⇒ different hash (the version-collision
        // requirement).
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
        // Known-answer: empty input hashes to the FNV-1a-128 offset basis
        // (pins the constants so a refactor can't silently change the
        // function and break reproduce).
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
        // Same name, two hashes ⇒ two distinct stored files (coexist).
        assert_ne!(
            stored_filename("m.zip", "aaaa"),
            stored_filename("m.zip", "bbbb")
        );
        // No extension ⇒ hash is a trailing segment.
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

        // Modlist A downloads v1 → BIO wrote the deterministic file.
        std::fs::write(archive_dir.join(&name), b"VERSION-1-CONTENT").unwrap();
        let s1 = ingest_downloaded_archives(&state, &dest_s, &[name.clone()]);
        assert_eq!(
            (s1.stored, s1.deduped, s1.missing),
            (1, 0, 0),
            "first ⇒ stored"
        );
        let h1 = hash_file(&archive_dir.join(&name)).unwrap();
        assert!(archive_dir.join(stored_filename(&name, &h1)).exists());
        // Per-install lock recorded the resolved hash (SPEC §13.12a).
        assert_eq!(
            InstallArchiveLock::load(&dest_s).hash_for(&name),
            Some(h1.as_str())
        );

        // Same modlist re-attempt with identical content ⇒ dedupe (no
        // duplicate copy, no new index entry).
        let s2 = ingest_downloaded_archives(&state, &dest_s, &[name.clone()]);
        assert_eq!(
            (s2.stored, s2.deduped),
            (0, 1),
            "identical re-download ⇒ deduped"
        );

        // A *different* version of the same logical name ⇒ a distinct
        // stored file; both coexist (SPEC §13.12a "both coexist").
        std::fs::write(archive_dir.join(&name), b"VERSION-2-DIFFERENT").unwrap();
        let s3 = ingest_downloaded_archives(&state, &dest_s, &[name.clone()]);
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
        // SPEC §13.12a: a second modlist reusing an identical archive does
        // not re-download. Modlist B's lock records the hash modlist A
        // already stored ⇒ stage copies it to the deterministic path AND
        // drops the asset (so BIO never downloads it).
        let archive_dir = td();
        let dest_a = td();
        let dest_b = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();

        let a = asset("MOD/MOD.TP2", "github", "v1", "MOD-v1.zip");
        let name = archive_file_name(&a);

        // Modlist A downloads + ingests v1.
        std::fs::write(archive_dir.join(&name), b"SHARED-CONTENT").unwrap();
        ingest_downloaded_archives(&state, &dest_a.to_string_lossy(), &[name.clone()]);
        let h = hash_file(&archive_dir.join(&name)).unwrap();
        // Simulate BIO having cleaned the deterministic file (or modlist B
        // starting fresh): only the content-addressed copy remains.
        std::fs::remove_file(archive_dir.join(&name)).unwrap();

        // Modlist B resolves the SAME hash for the SAME logical name
        // (recorded in B's per-install lock — e.g. from the pinned
        // installed_refs / a prior reproduce).
        let mut lock_b = InstallArchiveLock::default();
        lock_b.resolved.insert(name.clone(), h.clone());
        lock_b.save(Path::new(dest_b.to_string_lossy().trim()));

        state.step2.update_selected_update_assets = vec![a.clone()];
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
        // First-ever resolution: no lock entry ⇒ the asset stays so BIO
        // downloads it normally (and ingest then records it).
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
        // No panic; both load to empty (worst case: a redundant download).
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
