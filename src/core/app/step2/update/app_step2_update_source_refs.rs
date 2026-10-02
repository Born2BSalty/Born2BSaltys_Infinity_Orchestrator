// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::platform_defaults::{app_config_dir, app_config_file};

const MOD_SOURCE_REFS_FILE_NAME: &str = "mod_installed_refs.toml";
const MODS_FOLDER_REFS_DIR_NAME: &str = "mods_folder_refs";
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ModSourceRefsFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) folder: Option<String>,
    #[serde(default)]
    pub(crate) refs: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) sources: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) archives: BTreeMap<String, InstalledArchiveRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct InstalledArchiveRecord {
    pub(crate) name: String,
    pub(crate) size: u64,
    pub(crate) hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) etag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct RemoteFileFacts {
    pub(crate) size: Option<u64>,
    pub(crate) last_modified: Option<String>,
    pub(crate) etag: Option<String>,
}

pub(crate) fn installed_archive_record(archive_path: &Path) -> io::Result<InstalledArchiveRecord> {
    let name = archive_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let size = fs::metadata(archive_path)?.len();
    let hash = crate::install_runtime::archive_store::hash_file(archive_path)?;
    Ok(InstalledArchiveRecord {
        name,
        size,
        hash,
        last_modified: None,
        etag: None,
    })
}

#[must_use]
pub(crate) fn same_remote_file(
    record: &InstalledArchiveRecord,
    remote: &RemoteFileFacts,
) -> Option<bool> {
    if let (Some(recorded), Some(served)) = (&record.etag, &remote.etag) {
        return Some(recorded == served);
    }
    if let (Some(recorded), Some(served)) = (&record.last_modified, &remote.last_modified) {
        return Some(recorded == served);
    }
    remote.size.map(|size| size == record.size)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefsTargets {
    pub(crate) list: PathBuf,
    pub(crate) folder: Option<PathBuf>,
    pub(crate) folder_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstalledRecord {
    pub(crate) tp2: String,
    pub(crate) source_id: Option<String>,
    pub(crate) source_ref: Option<String>,
    pub(crate) archive: Option<InstalledArchiveRecord>,
}

fn normalized_mods_folder(mods_folder: &str) -> String {
    let mut normalized = mods_folder.trim().replace('\\', "/").to_ascii_lowercase();
    while normalized.len() > 3 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

#[must_use]
pub(crate) fn mods_folder_key(mods_folder: &str) -> String {
    format!(
        "{:016x}",
        fnv1a_64(normalized_mods_folder(mods_folder).as_bytes())
    )
}

#[must_use]
pub(crate) fn mods_folder_refs_path(mods_folder: &str) -> Option<PathBuf> {
    if normalized_mods_folder(mods_folder).is_empty() {
        return None;
    }
    Some(
        app_config_dir()?
            .join(MODS_FOLDER_REFS_DIR_NAME)
            .join(format!("{}.toml", mods_folder_key(mods_folder))),
    )
}

fn save_refs_file_at(target: &Path, refs: &ModSourceRefsFile) -> io::Result<()> {
    let content = toml::to_string_pretty(refs).map_err(io::Error::other)?;
    fs::write(target, content)
}

fn insert_record(refs: &mut ModSourceRefsFile, record: &InstalledRecord) {
    let key = normalize_mod_download_tp2(&record.tp2);
    if let Some(source_ref) = &record.source_ref {
        refs.refs.insert(key.clone(), source_ref.trim().to_string());
    }
    if let Some(source_id) = &record.source_id {
        refs.sources
            .insert(key.clone(), source_id.trim().to_string());
    }
    if let Some(archive) = &record.archive {
        refs.archives.insert(key, archive.clone());
    }
}

fn write_records(
    target: &Path,
    label: Option<&str>,
    records: &[InstalledRecord],
) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = load_refs_file_at(target);
    for record in records {
        insert_record(&mut refs, record);
    }
    if refs.folder.is_none() {
        refs.folder = label.map(str::to_string);
    }
    save_refs_file_at(target, &refs)
}

pub(crate) fn save_installed_records(
    records: &[InstalledRecord],
    targets: &RefsTargets,
) -> io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    write_records(&targets.list, None, records)?;
    if let Some(folder) = &targets.folder
        && let Err(err) = write_records(folder, targets.folder_label.as_deref(), records)
    {
        tracing::warn!(
            target = "orchestrator",
            "record {} installed sources in the mods folder refs {}: {err}",
            records.len(),
            folder.display()
        );
    }
    Ok(())
}

pub(crate) fn installed_source_refs_path() -> std::path::PathBuf {
    crate::app::mod_downloads::active_modlist_dir().map_or_else(
        || app_config_file(MOD_SOURCE_REFS_FILE_NAME, "config"),
        |d| d.join(MOD_SOURCE_REFS_FILE_NAME),
    )
}

pub(crate) fn load_refs_file_at(path: &Path) -> ModSourceRefsFile {
    fs::read_to_string(path).map_or_else(
        |_| ModSourceRefsFile::default(),
        |value| parse_refs_file_text(&value),
    )
}

pub(crate) fn parse_refs_file_text(text: &str) -> ModSourceRefsFile {
    toml::from_str::<ModSourceRefsFile>(text).unwrap_or_default()
}

pub(crate) fn installed_source_ids_from_refs_file(
    refs_file: &ModSourceRefsFile,
) -> BTreeMap<String, String> {
    refs_file
        .sources
        .iter()
        .map(|(tp2, source_id)| (normalize_mod_download_tp2(tp2), source_id.clone()))
        .filter(|(tp2, source_id)| !tp2.is_empty() && !source_id.trim().is_empty())
        .collect()
}

pub(crate) fn load_installed_source_ids() -> BTreeMap<String, String> {
    let Ok(content) = fs::read_to_string(installed_source_refs_path()) else {
        return BTreeMap::new();
    };
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).unwrap_or_default();
    parsed
        .sources
        .into_iter()
        .map(|(tp2, source_id)| (normalize_mod_download_tp2(&tp2), source_id))
        .filter(|(tp2, source_id)| !tp2.is_empty() && !source_id.trim().is_empty())
        .collect()
}

#[derive(Debug, Default)]
pub(crate) struct InstalledRefLookup {
    folder: Option<ModSourceRefsFile>,
    list: ModSourceRefsFile,
}

impl InstalledRefLookup {
    #[must_use]
    pub(crate) fn load(mods_folder: &str) -> Self {
        let list_path = installed_source_refs_path();
        let folder_path = mods_folder_refs_path(mods_folder);
        let list = load_refs_file_at(&list_path);
        let folder = folder_path
            .filter(|path| path.is_file())
            .map(|path| load_refs_file_at(&path));
        Self::from_files(folder, list)
    }

    #[must_use]
    pub(crate) const fn from_files(
        folder: Option<ModSourceRefsFile>,
        list: ModSourceRefsFile,
    ) -> Self {
        Self { folder, list }
    }

    fn winner(&self, key: &str) -> &ModSourceRefsFile {
        self.folder_wins(key).unwrap_or(&self.list)
    }

    fn folder_wins(&self, key: &str) -> Option<&ModSourceRefsFile> {
        self.folder
            .as_ref()
            .filter(|folder| folder.sources.contains_key(key))
    }

    #[must_use]
    pub(crate) fn source_id(&self, tp2: &str) -> Option<String> {
        let key = normalize_mod_download_tp2(tp2);
        self.winner(&key).sources.get(&key).cloned()
    }

    #[must_use]
    pub(crate) fn source_id_and_ref(&self, tp2: &str) -> Option<(String, String)> {
        let key = normalize_mod_download_tp2(tp2);
        let winner = self.winner(&key);
        Some((
            winner.sources.get(&key)?.clone(),
            winner.refs.get(&key)?.clone(),
        ))
    }

    #[must_use]
    pub(crate) fn archive(&self, tp2: &str) -> Option<&InstalledArchiveRecord> {
        let key = normalize_mod_download_tp2(tp2);
        self.winner(&key).archives.get(&key)
    }

    #[must_use]
    pub(crate) fn refs_for_export(&self, include: &BTreeSet<String>) -> BTreeMap<String, String> {
        let mut refs = self.list.refs.clone();
        for key in include {
            let Some(folder) = self.folder_wins(key) else {
                continue;
            };
            match folder.refs.get(key) {
                Some(source_ref) => refs.insert(key.clone(), source_ref.clone()),
                None => refs.remove(key),
            };
        }
        refs
    }
}

pub(super) fn prune_installed_source_refs<I, S>(
    mods_folder: &str,
    present_tp2s: I,
) -> io::Result<usize>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let present_tp2s = present_tp2s
        .into_iter()
        .map(|tp2| normalize_mod_download_tp2(tp2.as_ref()))
        .collect::<BTreeSet<_>>();
    let list_path = installed_source_refs_path();
    let folder_path = mods_folder_refs_path(mods_folder);
    let (removed, list) = prune_list_file(&list_path, &present_tp2s)?;
    if let Some(folder_path) = folder_path {
        prune_and_seed_folder_file(&folder_path, mods_folder.trim(), &list, &present_tp2s)?;
    }
    Ok(removed)
}

fn prune_list_file(
    path: &Path,
    present_tp2s: &BTreeSet<String>,
) -> io::Result<(usize, ModSourceRefsFile)> {
    let Ok(content) = fs::read_to_string(path) else {
        return Ok((0, ModSourceRefsFile::default()));
    };
    let mut refs = parse_refs_file_text(&content);
    let removed = retain_present(&mut refs, present_tp2s);
    if removed > 0 {
        save_refs_file_at(path, &refs)?;
    }
    Ok((removed, refs))
}

fn prune_and_seed_folder_file(
    path: &Path,
    label: &str,
    list: &ModSourceRefsFile,
    present_tp2s: &BTreeSet<String>,
) -> io::Result<()> {
    let mut folder = load_refs_file_at(path);
    let changed = retain_present(&mut folder, present_tp2s) + seed_missing(&mut folder, list);
    if changed == 0 {
        return Ok(());
    }
    if folder.folder.is_none() {
        folder.folder = Some(label.to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    save_refs_file_at(path, &folder)
}

fn retain_present(refs: &mut ModSourceRefsFile, present_tp2s: &BTreeSet<String>) -> usize {
    let before = refs.refs.len() + refs.sources.len() + refs.archives.len();
    refs.refs.retain(|tp2, _| present_tp2s.contains(tp2));
    refs.sources.retain(|tp2, _| present_tp2s.contains(tp2));
    refs.archives.retain(|tp2, _| present_tp2s.contains(tp2));
    before.saturating_sub(refs.refs.len() + refs.sources.len() + refs.archives.len())
}

fn seed_missing(folder: &mut ModSourceRefsFile, list: &ModSourceRefsFile) -> usize {
    let mut added = 0;
    for (tp2, archive) in &list.archives {
        if folder.refs.contains_key(tp2)
            || folder.sources.contains_key(tp2)
            || folder.archives.contains_key(tp2)
        {
            continue;
        }
        folder.archives.insert(tp2.clone(), archive.clone());
        added += 1;
        added += seed_value(&mut folder.sources, &list.sources, tp2);
        added += seed_value(&mut folder.refs, &list.refs, tp2);
    }
    added
}

fn seed_value(
    target: &mut BTreeMap<String, String>,
    source: &BTreeMap<String, String>,
    tp2: &str,
) -> usize {
    source.get(tp2).map_or(0, |value| {
        target.insert(tp2.to_string(), value.clone());
        1
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::app::mod_downloads::AMBIENT_TEST_LOCK;

    struct AmbientGuard(Option<PathBuf>);

    impl AmbientGuard {
        fn acquire() -> Self {
            Self(crate::app::mod_downloads::active_modlist_dir())
        }
    }

    impl Drop for AmbientGuard {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    fn unique_tmp_dir(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "bio_refs_test_{}_{}_{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ))
    }

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Self {
            let path = unique_tmp_dir(label);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn record(name: &str, size: u64, hash: &str) -> InstalledArchiveRecord {
        InstalledArchiveRecord {
            name: name.to_string(),
            size,
            hash: hash.to_string(),
            last_modified: None,
            etag: None,
        }
    }

    fn list_only(path: &Path) -> RefsTargets {
        RefsTargets {
            list: path.to_path_buf(),
            folder: None,
            folder_label: None,
        }
    }

    struct FolderRefsRoot(PathBuf);

    impl FolderRefsRoot {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let root = Self(std::env::temp_dir().join(format!(
                "bio_folderrefs_test_{}_{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            )));
            std::fs::create_dir_all(&root.0).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(root.0.clone()));
            root
        }

        fn dir(&self, name: &str) -> PathBuf {
            let dir = self.0.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn folder_targets(&self, mods_folder: &str) -> RefsTargets {
            RefsTargets {
                list: self.0.join("scratch_list.toml"),
                folder: mods_folder_refs_path(mods_folder),
                folder_label: Some(mods_folder.to_string()),
            }
        }
    }

    impl Drop for FolderRefsRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn installed(tp2: &str) -> InstalledRecord {
        InstalledRecord {
            tp2: tp2.to_string(),
            source_id: None,
            source_ref: None,
            archive: None,
        }
    }

    fn full_record(tp2: &str, source_ref: &str, archive_hash: &str) -> InstalledRecord {
        InstalledRecord {
            source_id: Some("primary".to_string()),
            source_ref: Some(source_ref.to_string()),
            archive: Some(record(&format!("{tp2}.zip"), 1, archive_hash)),
            ..installed(tp2)
        }
    }

    fn save_record(tp2: &str, source_ref: &str, archive_hash: &str, targets: &RefsTargets) {
        save_installed_records(&[full_record(tp2, source_ref, archive_hash)], targets).unwrap();
    }

    fn save_ref(tp2: &str, source_ref: &str, targets: &RefsTargets) {
        let saved = InstalledRecord {
            source_ref: Some(source_ref.to_string()),
            ..installed(tp2)
        };
        save_installed_records(&[saved], targets).unwrap();
    }

    fn save_id(tp2: &str, source_id: &str, targets: &RefsTargets) {
        let saved = InstalledRecord {
            source_id: Some(source_id.to_string()),
            ..installed(tp2)
        };
        save_installed_records(&[saved], targets).unwrap();
    }

    fn save_archive(tp2: &str, archive: &InstalledArchiveRecord, targets: &RefsTargets) {
        let saved = InstalledRecord {
            archive: Some(archive.clone()),
            ..installed(tp2)
        };
        save_installed_records(&[saved], targets).unwrap();
    }

    fn pair(source_id: &str, source_ref: &str) -> (String, String) {
        (source_id.to_string(), source_ref.to_string())
    }

    #[test]
    fn mods_folder_key_ignores_case_slashes_and_trailing_separator() {
        let key = mods_folder_key(r"C:\Games\Mods");
        assert_eq!(key.len(), 16);
        assert!(
            key.chars()
                .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch)),
            "{key}"
        );
        assert_eq!(mods_folder_key("c:/games/mods/"), key);
        assert_eq!(mods_folder_key("C:/GAMES/MODS"), key);
        assert_eq!(mods_folder_key(r"  C:\Games\Mods\\  "), key);
        assert_ne!(mods_folder_key(r"C:\Games\Mods2"), key);
        assert_eq!(mods_folder_key(r"C:\"), mods_folder_key("c:/"));
        assert_ne!(mods_folder_key("c:/"), mods_folder_key("c:"));
    }

    #[test]
    fn blank_mods_folder_has_no_folder_file() {
        let root = FolderRefsRoot::new();
        assert_eq!(mods_folder_refs_path(""), None);
        assert_eq!(mods_folder_refs_path("  \t "), None);

        let path = mods_folder_refs_path(r"C:\Games\Mods").unwrap();
        assert_eq!(
            path,
            root.0
                .join(MODS_FOLDER_REFS_DIR_NAME)
                .join(format!("{}.toml", mods_folder_key(r"C:\Games\Mods")))
        );
    }

    #[test]
    fn save_installed_records_writes_both_files_once_and_labels_the_folder() {
        let root = FolderRefsRoot::new();
        let label = root.dir("mods").to_string_lossy().into_owned();
        let targets = RefsTargets {
            list: root.0.join("list").join(MOD_SOURCE_REFS_FILE_NAME),
            ..root.folder_targets(&label)
        };
        let folder_path = targets.folder.clone().unwrap();

        save_installed_records(
            &[
                full_record("Alpha/Alpha.tp2", " master@abc ", "aa"),
                full_record("Omega/Omega.tp2", "v7", "oo"),
            ],
            &targets,
        )
        .unwrap();

        let key = normalize_mod_download_tp2("Alpha/Alpha.tp2");
        let omega = normalize_mod_download_tp2("Omega/Omega.tp2");
        for path in [&targets.list, &folder_path] {
            let loaded = load_refs_file_at(path);
            assert_eq!(
                loaded.refs.get(&key).map(String::as_str),
                Some("master@abc")
            );
            assert_eq!(
                loaded.sources.get(&key).map(String::as_str),
                Some("primary")
            );
            assert_eq!(
                loaded.archives.get(&key),
                Some(&record("Alpha/Alpha.tp2.zip", 1, "aa"))
            );
            assert_eq!(loaded.refs.get(&omega).map(String::as_str), Some("v7"));
            assert_eq!(
                loaded.archives.get(&omega),
                Some(&record("Omega/Omega.tp2.zip", 1, "oo"))
            );
        }
        assert_eq!(
            load_refs_file_at(&folder_path).folder.as_deref(),
            Some(label.as_str())
        );
        assert_eq!(load_refs_file_at(&targets.list).folder, None);
        let list_text = std::fs::read_to_string(&targets.list).unwrap();
        assert!(
            !list_text
                .lines()
                .any(|line| line.trim_start().starts_with("folder")),
            "{list_text}"
        );

        let relabelled = RefsTargets {
            folder_label: Some("elsewhere".to_string()),
            ..targets.clone()
        };
        save_ref("beta", "v2", &relabelled);
        assert_eq!(
            load_refs_file_at(&folder_path).folder.as_deref(),
            Some(label.as_str())
        );

        let without_ref = InstalledRecord {
            source_id: Some("fork".to_string()),
            archive: Some(record("alpha2.zip", 2, "a2")),
            ..installed("Alpha/Alpha.tp2")
        };
        save_installed_records(&[without_ref], &targets).unwrap();
        for path in [&targets.list, &folder_path] {
            let loaded = load_refs_file_at(path);
            assert_eq!(
                loaded.refs.get(&key).map(String::as_str),
                Some("master@abc")
            );
            assert_eq!(loaded.sources.get(&key).map(String::as_str), Some("fork"));
            assert_eq!(
                loaded.archives.get(&key),
                Some(&record("alpha2.zip", 2, "a2"))
            );
        }

        let blocker = root.0.join("blocker");
        std::fs::write(&blocker, b"file").unwrap();
        let unwritable_folder = RefsTargets {
            folder: Some(blocker.join("refs.toml")),
            ..targets.clone()
        };
        save_ref("gamma", "v3", &unwritable_folder);
        assert_eq!(
            load_refs_file_at(&targets.list)
                .refs
                .get("gamma")
                .map(String::as_str),
            Some("v3")
        );
    }

    #[test]
    fn lookup_prefers_the_folder_record_then_the_list() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        let root = FolderRefsRoot::new();
        crate::app::mod_downloads::set_active_modlist_dir(Some(root.dir("list")));
        let mods = root.dir("mods").to_string_lossy().into_owned();
        let list = list_only(&installed_source_refs_path());
        let folder = root.folder_targets(&mods);

        save_record("alpha", "master@sha1", "a1", &list);
        save_record("alpha", "master@sha2", "a2", &folder);
        save_record("beta", "master@sha1", "b1", &list);
        save_record("delta", "master@sha4", "d4", &folder);
        save_record("epsilon", "master@list_e", "le", &list);
        save_ref("epsilon", "master@folder_e", &folder);
        save_archive("epsilon", &record("e.zip", 1, "fe"), &folder);
        save_ref("zeta", "master@folder_z", &folder);
        save_archive("zeta", &record("z.zip", 1, "fz"), &folder);

        let lookup = InstalledRefLookup::load(&mods);
        assert_eq!(
            lookup.source_id_and_ref("Alpha/ALPHA.TP2"),
            Some(pair("primary", "master@sha2"))
        );
        assert_eq!(lookup.archive("alpha").map(|a| a.hash.as_str()), Some("a2"));
        assert_eq!(lookup.source_id("alpha").as_deref(), Some("primary"));
        assert_eq!(
            lookup.source_id_and_ref("beta"),
            Some(pair("primary", "master@sha1"))
        );
        assert_eq!(lookup.archive("beta").map(|a| a.hash.as_str()), Some("b1"));
        assert_eq!(
            lookup.source_id_and_ref("epsilon"),
            Some(pair("primary", "master@list_e"))
        );
        assert_eq!(
            lookup.archive("epsilon").map(|a| a.hash.as_str()),
            Some("le")
        );
        assert_eq!(lookup.source_id_and_ref("zeta"), None);
        assert_eq!(lookup.archive("zeta"), None);
        assert_eq!(lookup.source_id("zeta"), None);
        assert_eq!(lookup.source_id_and_ref("gamma"), None);
        assert_eq!(lookup.archive("gamma"), None);

        let include = ["alpha", "beta", "delta", "epsilon", "zeta"]
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        let exported = lookup.refs_for_export(&include);
        assert_eq!(
            exported.get("alpha").map(String::as_str),
            Some("master@sha2")
        );
        assert_eq!(
            exported.get("beta").map(String::as_str),
            Some("master@sha1")
        );
        assert_eq!(
            exported.get("delta").map(String::as_str),
            Some("master@sha4")
        );
        assert_eq!(
            exported.get("epsilon").map(String::as_str),
            Some("master@list_e")
        );
        assert_eq!(exported.get("zeta"), None);

        let unchecked = lookup.refs_for_export(&BTreeSet::new());
        assert_eq!(
            unchecked.get("alpha").map(String::as_str),
            Some("master@sha1")
        );
        assert_eq!(unchecked.get("delta"), None);

        let list_only_lookup = InstalledRefLookup::load("");
        assert_eq!(
            list_only_lookup.source_id_and_ref("alpha"),
            Some(pair("primary", "master@sha1"))
        );
        assert_eq!(list_only_lookup.source_id_and_ref("delta"), None);
    }

    #[test]
    fn prune_prunes_both_files_and_seeds_missing_folder_records() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        let root = FolderRefsRoot::new();
        crate::app::mod_downloads::set_active_modlist_dir(Some(root.dir("list")));
        let mods = root.dir("mods").to_string_lossy().into_owned();
        let list = list_only(&installed_source_refs_path());
        let folder = root.folder_targets(&mods);
        let folder_path = folder.folder.clone().unwrap();

        save_record("x", "master@list_x", "lx", &list);
        save_record("y", "master@list_y", "ly", &list);
        save_record("x", "master@folder_x", "fx", &folder);
        save_id("z", "primary", &list);
        save_ref("z", "master@list_z", &list);
        save_record("w", "master@list_w", "lw", &list);
        save_ref("w", "master@folder_w", &folder);
        save_archive("v", &record("v.zip", 1, "lv"), &list);
        save_ref("v", "master@list_v", &list);

        assert_eq!(
            prune_installed_source_refs(&mods, ["x", "y", "z", "w", "v"]).unwrap(),
            0
        );
        let seeded = load_refs_file_at(&folder_path);
        assert!(!seeded.refs.contains_key("z"));
        assert!(!seeded.sources.contains_key("z"));
        assert_eq!(
            seeded.refs.get("w").map(String::as_str),
            Some("master@folder_w")
        );
        assert!(!seeded.sources.contains_key("w"));
        assert!(!seeded.archives.contains_key("w"));
        assert_eq!(
            seeded.archives.get("v").map(|a| a.hash.as_str()),
            Some("lv")
        );
        assert_eq!(
            seeded.refs.get("v").map(String::as_str),
            Some("master@list_v")
        );
        assert!(!seeded.sources.contains_key("v"));
        assert_eq!(seeded.folder.as_deref(), Some(mods.as_str()));
        assert_eq!(
            seeded.refs.get("x").map(String::as_str),
            Some("master@folder_x")
        );
        assert_eq!(
            seeded.archives.get("x").map(|a| a.hash.as_str()),
            Some("fx")
        );
        assert_eq!(
            seeded.refs.get("y").map(String::as_str),
            Some("master@list_y")
        );
        assert_eq!(seeded.sources.get("y").map(String::as_str), Some("primary"));
        assert_eq!(
            seeded.archives.get("y").map(|a| a.hash.as_str()),
            Some("ly")
        );

        assert_eq!(prune_installed_source_refs(&mods, ["x"]).unwrap(), 10);
        for path in [&list.list, &folder_path] {
            let pruned = load_refs_file_at(path);
            assert!(!pruned.refs.contains_key("y"), "{}", path.display());
            assert!(!pruned.sources.contains_key("y"), "{}", path.display());
            assert!(!pruned.archives.contains_key("y"), "{}", path.display());
        }
        assert_eq!(
            load_refs_file_at(&folder_path)
                .refs
                .get("x")
                .map(String::as_str),
            Some("master@folder_x")
        );
    }

    #[test]
    fn archive_record_round_trips_through_the_refs_file() {
        let root = TempRoot::new("archive_round_trip");
        let path = root.0.join("mod_installed_refs.toml");
        save_ref("Alpha/Alpha.tp2", "v19", &list_only(&path));
        save_id("Alpha/Alpha.tp2", "primary", &list_only(&path));
        let saved = record(
            "alpha__primary__v19.zip",
            42,
            "00ff00ff00ff00ff00ff00ff00ff00ff",
        );

        save_archive("Alpha/Alpha.tp2", &saved, &list_only(&path));

        let loaded = load_refs_file_at(&path);
        let key = normalize_mod_download_tp2("Alpha/Alpha.tp2");
        assert_eq!(loaded.archives.get(&key), Some(&saved));
        assert_eq!(loaded.archives.len(), 1);
        assert_eq!(loaded.refs.get(&key).map(String::as_str), Some("v19"));
        assert_eq!(loaded.refs.len(), 1);
        assert_eq!(
            loaded.sources.get(&key).map(String::as_str),
            Some("primary")
        );
        assert_eq!(loaded.sources.len(), 1);
    }

    #[test]
    fn refs_file_without_archives_serialises_as_before() {
        let text = "[refs]\nalpha = \"v19\"\n\n[sources]\nalpha = \"primary\"\n";
        let parsed = parse_refs_file_text(text);
        assert_eq!(parsed.archives.len(), 0);

        let serialised = toml::to_string_pretty(&parsed).unwrap();
        assert!(!serialised.contains("archives"), "{serialised}");

        let reparsed = parse_refs_file_text(&serialised);
        assert_eq!(reparsed.refs, parsed.refs);
        assert_eq!(reparsed.sources, parsed.sources);
        assert_eq!(reparsed.archives.len(), 0);
    }

    fn refs_with_archive(archive: &InstalledArchiveRecord) -> ModSourceRefsFile {
        let mut refs = ModSourceRefsFile::default();
        refs.archives
            .insert("d0questpack".to_string(), archive.clone());
        refs
    }

    #[test]
    fn archive_record_round_trips_the_headers() {
        let archive = InstalledArchiveRecord {
            last_modified: Some("Thu, 10 Sep 2020 17:25:37 GMT".to_string()),
            etag: Some("\"abc-123\"".to_string()),
            ..record("questpack-v35-win.zip", 21_707_615, "ff00")
        };

        let serialised = toml::to_string_pretty(&refs_with_archive(&archive)).unwrap();
        let reparsed = parse_refs_file_text(&serialised);

        assert_eq!(reparsed.archives.get("d0questpack"), Some(&archive));
    }

    #[test]
    fn archive_record_without_headers_omits_them() {
        let archive = record("questpack-v35-win.zip", 21_707_615, "ff00");

        let serialised = toml::to_string_pretty(&refs_with_archive(&archive)).unwrap();

        assert!(!serialised.contains("last_modified"), "{serialised}");
        assert!(!serialised.contains("etag"), "{serialised}");
        let before_this_run = "[archives.d0questpack]\nname = \"questpack-v35-win.zip\"\nsize = 21707615\nhash = \"ff00\"\n";
        let parsed = parse_refs_file_text(before_this_run);
        let loaded = parsed.archives.get("d0questpack").unwrap();
        assert_eq!(loaded.last_modified, None);
        assert_eq!(loaded.etag, None);
        assert_eq!(loaded, &archive);
    }

    #[test]
    fn same_remote_file_prefers_etag_then_date_then_size() {
        let date = "Thu, 10 Sep 2020 17:25:37 GMT".to_string();
        let dated = InstalledArchiveRecord {
            last_modified: Some(date.clone()),
            etag: Some("\"one\"".to_string()),
            ..record("q.zip", 10, "aa")
        };
        let other_etag = RemoteFileFacts {
            size: Some(10),
            last_modified: Some(date.clone()),
            etag: Some("\"two\"".to_string()),
        };
        assert_eq!(same_remote_file(&dated, &other_etag), Some(false));

        let dated_only = InstalledArchiveRecord {
            etag: None,
            ..dated
        };
        let same_date = RemoteFileFacts {
            size: Some(99),
            last_modified: Some(date),
            etag: Some("\"two\"".to_string()),
        };
        assert_eq!(same_remote_file(&dated_only, &same_date), Some(true));

        let bare = record("q.zip", 10, "aa");
        let same_size = RemoteFileFacts {
            size: Some(10),
            ..RemoteFileFacts::default()
        };
        assert_eq!(same_remote_file(&bare, &same_size), Some(true));

        assert_eq!(same_remote_file(&bare, &RemoteFileFacts::default()), None);
    }

    #[test]
    fn installed_archive_record_hashes_with_the_store_hasher() {
        let root = TempRoot::new("archive_hash");
        let archive = root.0.join("alpha__primary__v19.zip");
        let bytes = b"ARCHIVE-BYTES-FOR-THE-RECORD";
        std::fs::write(&archive, bytes).unwrap();

        let made = installed_archive_record(&archive).unwrap();

        assert_eq!(made.name, "alpha__primary__v19.zip");
        assert_eq!(made.size, u64::try_from(bytes.len()).unwrap());
        assert_eq!(
            made.hash,
            crate::install_runtime::archive_store::hash_file(&archive).unwrap()
        );
    }

    #[test]
    fn prune_drops_archive_records_of_absent_mods() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        let root = TempRoot::new("archive_prune");
        crate::app::mod_downloads::set_active_modlist_dir(Some(root.0.clone()));
        let path = installed_source_refs_path();
        save_archive("alpha", &record("a.zip", 1, "aa"), &list_only(&path));
        save_archive("beta", &record("b.zip", 2, "bb"), &list_only(&path));

        let removed = prune_installed_source_refs("", ["alpha"]).unwrap();

        assert_eq!(removed, 1);
        let loaded = load_refs_file_at(&path);
        assert_eq!(loaded.archives.len(), 1);
        assert_eq!(
            loaded.archives.get("alpha"),
            Some(&record("a.zip", 1, "aa"))
        );
    }

    #[test]
    fn installed_refs_ambient_unset_targets_global() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let tmp = unique_tmp_dir("global");
        std::fs::create_dir_all(&tmp).unwrap();
        let global_path = tmp.join("mod_installed_refs.toml");

        save_ref("testmod", "abc123", &list_only(&global_path));
        save_id("testmod", "main", &list_only(&global_path));

        let content = std::fs::read_to_string(&global_path).unwrap();
        assert!(content.contains("abc123"), "ref written to global path");
        assert!(content.contains("main"), "source id written to global path");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn installed_refs_ambient_set_targets_per_modlist() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let per_dir = unique_tmp_dir("per");
        let global_dir = unique_tmp_dir("global2");
        std::fs::create_dir_all(&per_dir).unwrap();
        std::fs::create_dir_all(&global_dir).unwrap();

        let per_path = per_dir.join("mod_installed_refs.toml");
        let global_path = global_dir.join("mod_installed_refs.toml");
        std::fs::write(&global_path, "# sentinel\n").unwrap();

        crate::app::mod_downloads::set_active_modlist_dir(Some(per_dir.clone()));

        let resolved = installed_source_refs_path();
        assert_eq!(
            resolved, per_path,
            "resolver returns per-modlist path when ambient is set"
        );

        save_ref("testmod", "v42", &list_only(&resolved));

        let per_content = std::fs::read_to_string(&per_path).unwrap();
        assert!(
            per_content.contains("v42"),
            "ref written to per-modlist file"
        );

        let global_content = std::fs::read_to_string(&global_path).unwrap();
        assert_eq!(global_content, "# sentinel\n", "global file unchanged");

        let _ = std::fs::remove_dir_all(&per_dir);
        let _ = std::fs::remove_dir_all(&global_dir);
    }

    #[test]
    fn installed_refs_captured_path_write_is_thread_safe() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let per_dir = unique_tmp_dir("capture");
        std::fs::create_dir_all(&per_dir).unwrap();
        let captured = per_dir.join("mod_installed_refs.toml");

        crate::app::mod_downloads::set_active_modlist_dir(Some(per_dir.clone()));
        let captured_path = installed_source_refs_path();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        save_id("mod", "source-id", &list_only(&captured_path));

        assert!(captured.exists(), "captured per-modlist file was written");
        let content = std::fs::read_to_string(&captured).unwrap();
        assert!(content.contains("source-id"), "source id in captured file");

        let _ = std::fs::remove_dir_all(&per_dir);
    }

    #[test]
    fn installed_refs_paste_install_captures_per_modlist_not_global() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let per_dir = unique_tmp_dir("r3crit");
        let per_refs = per_dir.join("mod_installed_refs.toml");

        crate::app::mod_downloads::set_active_modlist_dir(None);

        let install_ctx_path = per_refs.clone();

        let ambient_path = installed_source_refs_path();
        assert_ne!(
            ambient_path, install_ctx_path,
            "install_ctx path must differ from global when ambient is None"
        );

        assert_eq!(
            install_ctx_path, per_refs,
            "install context path is the per-modlist file"
        );
    }
}
