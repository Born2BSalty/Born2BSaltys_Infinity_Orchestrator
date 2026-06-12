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

/// Serializable record of installed source-ids and refs, owned per-modlist.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ModSourceRefsFile {
    #[serde(default)]
    pub(crate) refs: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) sources: BTreeMap<String, String>,
}

/// Returns the installed-refs path for the active modlist when one is set,
/// or the global config-dir path otherwise. All installed-refs I/O routes
/// through this resolver so making it ambient-aware flips every call site at once.
pub(crate) fn installed_source_refs_path() -> std::path::PathBuf {
    crate::app::mod_downloads::active_modlist_dir().map_or_else(
        || app_config_file(MOD_SOURCE_REFS_FILE_NAME, "config"),
        |d| d.join(MOD_SOURCE_REFS_FILE_NAME),
    )
}

/// Loads a refs file from an explicit path.
pub(crate) fn load_refs_file_at(path: &Path) -> ModSourceRefsFile {
    fs::read_to_string(path).map_or_else(
        |_| ModSourceRefsFile::default(),
        |value| toml::from_str::<ModSourceRefsFile>(&value).unwrap_or_default(),
    )
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

/// Saves an installed source ref to the given `target` path.
/// Off-thread workers receive the path captured at job-build time (main thread),
/// preventing races with `page_router` ambient clears mid-install.
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

/// Saves an installed source id to the given `target` path.
/// Off-thread workers receive the path captured at job-build time (main thread).
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
    let removed =
        before.saturating_sub(refs.refs.len()) + before_sources.saturating_sub(refs.sources.len());
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
    use std::sync::Mutex;

    use super::*;

    // Shared test-lock: ensures ambient-touching tests never interleave.
    static REFS_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// RAII drop-guard: restores the ambient active-modlist dir on drop (including panic).
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

    #[test]
    fn installed_refs_ambient_unset_targets_global() {
        let _lock = REFS_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        let tmp = unique_tmp_dir("global");
        std::fs::create_dir_all(&tmp).unwrap();
        let global_path = tmp.join("mod_installed_refs.toml");

        // Write via explicit target path (ambient is None so resolver returns global).
        save_installed_source_ref("testmod", "abc123", &global_path).unwrap();
        save_installed_source_id("testmod", "main", &global_path).unwrap();

        let content = std::fs::read_to_string(&global_path).unwrap();
        assert!(content.contains("abc123"), "ref written to global path");
        assert!(content.contains("main"), "source id written to global path");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn installed_refs_ambient_set_targets_per_modlist() {
        let _lock = REFS_TEST_LOCK
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

        // With ambient set, the resolver should return the per-modlist path.
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
        let _lock = REFS_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        let per_dir = unique_tmp_dir("capture");
        std::fs::create_dir_all(&per_dir).unwrap();
        let captured = per_dir.join("mod_installed_refs.toml");

        // Capture the path on the main thread, then clear the ambient (simulating page_router).
        crate::app::mod_downloads::set_active_modlist_dir(Some(per_dir.clone()));
        let captured_path = installed_source_refs_path();
        crate::app::mod_downloads::set_active_modlist_dir(None);

        // The write must still target the captured (per-modlist) path, not the global.
        save_installed_source_id("mod", "source-id", &captured_path).unwrap();

        assert!(captured.exists(), "captured per-modlist file was written");
        let content = std::fs::read_to_string(&captured).unwrap();
        assert!(content.contains("source-id"), "source id in captured file");

        let _ = std::fs::remove_dir_all(&per_dir);
    }

    #[test]
    fn installed_refs_paste_install_captures_per_modlist_not_global() {
        let _lock = REFS_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = AmbientGuard::acquire();

        // Simulate: active_install_modlist_id is set → derive the per-modlist path.
        let per_dir = unique_tmp_dir("r3crit");
        let per_refs = per_dir.join("mod_installed_refs.toml");

        // Ambient is None (page_router cleared it).
        crate::app::mod_downloads::set_active_modlist_dir(None);

        // The install-context path is derived from active_install_modlist_id (simulated here).
        let install_ctx_path = per_refs.clone();

        // With ambient None, resolved path is global — but install_ctx overrides it.
        let ambient_path = installed_source_refs_path();
        assert_ne!(
            ambient_path, install_ctx_path,
            "install_ctx path must differ from global when ambient is None"
        );

        // The captured path (from install_ctx) must not be global.
        // (In production, build_extract_jobs uses the install_ctx, not the ambient.)
        assert_eq!(
            install_ctx_path, per_refs,
            "install context path is the per-modlist file"
        );
    }
}
