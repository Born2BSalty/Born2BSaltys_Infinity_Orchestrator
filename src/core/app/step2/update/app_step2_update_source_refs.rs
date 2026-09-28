// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::platform_defaults::app_config_file;

const MOD_SOURCE_REFS_FILE_NAME: &str = "mod_installed_refs.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ModSourceRefsFile {
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
}

pub(crate) fn installed_archive_record(archive_path: &Path) -> io::Result<InstalledArchiveRecord> {
    let name = archive_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let size = fs::metadata(archive_path)?.len();
    let hash = crate::install_runtime::archive_store::hash_file(archive_path)?;
    Ok(InstalledArchiveRecord { name, size, hash })
}

pub(super) fn save_installed_archive_record(
    tp2: &str,
    record: InstalledArchiveRecord,
    target: &Path,
) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = load_refs_file_at(target);
    refs.archives
        .insert(normalize_mod_download_tp2(tp2), record);
    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(target, content)
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

pub(super) fn load_installed_source_id_and_ref(tp2: &str) -> Option<(String, String)> {
    let content = fs::read_to_string(installed_source_refs_path()).ok()?;
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).ok()?;
    let tp2 = normalize_mod_download_tp2(tp2);
    Some((
        parsed.sources.get(&tp2)?.clone(),
        parsed.refs.get(&tp2)?.clone(),
    ))
}

pub(super) fn save_installed_source_ref(
    tp2: &str,
    source_ref: &str,
    target: &Path,
) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = load_refs_file_at(target);
    refs.refs.insert(
        normalize_mod_download_tp2(tp2),
        source_ref.trim().to_string(),
    );
    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(target, content)
}

pub(super) fn load_installed_source_id(tp2: &str) -> Option<String> {
    let content = fs::read_to_string(installed_source_refs_path()).ok()?;
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).ok()?;
    parsed
        .sources
        .get(&normalize_mod_download_tp2(tp2))
        .cloned()
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

pub(super) fn save_installed_source_id(
    tp2: &str,
    source_id: &str,
    target: &Path,
) -> io::Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = load_refs_file_at(target);
    refs.sources.insert(
        normalize_mod_download_tp2(tp2),
        source_id.trim().to_string(),
    );
    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(target, content)
}

pub(super) fn prune_installed_source_refs<I, S>(present_tp2s: I) -> io::Result<usize>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let path = installed_source_refs_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(0);
    };

    let mut refs = toml::from_str::<ModSourceRefsFile>(&content).unwrap_or_default();
    let present_tp2s = present_tp2s
        .into_iter()
        .map(|tp2| normalize_mod_download_tp2(tp2.as_ref()))
        .collect::<BTreeSet<_>>();

    let before = refs.refs.len();
    refs.refs.retain(|tp2, _| present_tp2s.contains(tp2));
    let before_sources = refs.sources.len();
    refs.sources.retain(|tp2, _| present_tp2s.contains(tp2));
    let before_archives = refs.archives.len();
    refs.archives.retain(|tp2, _| present_tp2s.contains(tp2));
    let removed = before.saturating_sub(refs.refs.len())
        + before_sources.saturating_sub(refs.sources.len())
        + before_archives.saturating_sub(refs.archives.len());
    if removed == 0 {
        return Ok(0);
    }

    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(path, content)?;
    Ok(removed)
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
        }
    }

    #[test]
    fn archive_record_round_trips_through_the_refs_file() {
        let root = TempRoot::new("archive_round_trip");
        let path = root.0.join("mod_installed_refs.toml");
        save_installed_source_ref("Alpha/Alpha.tp2", "v19", &path).unwrap();
        save_installed_source_id("Alpha/Alpha.tp2", "primary", &path).unwrap();
        let saved = record(
            "alpha__primary__v19.zip",
            42,
            "00ff00ff00ff00ff00ff00ff00ff00ff",
        );

        save_installed_archive_record("Alpha/Alpha.tp2", saved.clone(), &path).unwrap();

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
        assert!(parsed.archives.is_empty());

        let serialised = toml::to_string_pretty(&parsed).unwrap();
        assert!(!serialised.contains("archives"), "{serialised}");

        let reparsed = parse_refs_file_text(&serialised);
        assert_eq!(reparsed.refs, parsed.refs);
        assert_eq!(reparsed.sources, parsed.sources);
        assert!(reparsed.archives.is_empty());
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
        save_installed_archive_record("alpha", record("a.zip", 1, "aa"), &path).unwrap();
        save_installed_archive_record("beta", record("b.zip", 2, "bb"), &path).unwrap();

        let removed = prune_installed_source_refs(["alpha"]).unwrap();

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

        save_installed_source_ref("testmod", "abc123", &global_path).unwrap();
        save_installed_source_id("testmod", "main", &global_path).unwrap();

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

        save_installed_source_ref("testmod", "v42", &resolved).unwrap();

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

        save_installed_source_id("mod", "source-id", &captured_path).unwrap();

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
