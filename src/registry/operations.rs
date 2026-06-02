// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use crate::registry::errors::RegistryError;
use crate::registry::model::{ModlistEntry, ModlistRegistry};
use crate::registry::store::RegistryStore;

/// Returned by `remove_entry_and_save` when a background folder removal is needed.
#[derive(Debug)]
pub struct DeleteTarget {
    /// Display name of the removed modlist (for the toast message).
    pub name: String,
    /// Absolute path of the install folder to remove asynchronously.
    pub dest: PathBuf,
}

/// Result type sent by the background folder-delete worker.
pub type FolderDeleteResult = Result<(), String>;

/// Receiver for the background folder-delete worker.
pub type FolderDeleteReceiver = Receiver<FolderDeleteResult>;

/// Removes the registry entry and persists the updated registry synchronously.
///
/// Returns `Some(DeleteTarget)` when an on-disk folder removal is needed (the
/// destination passed the safety guard), or `None` when the folder is absent,
/// empty, or relative (no async work to do in those cases).
pub fn remove_entry_and_save(
    id: &str,
    store: &RegistryStore,
    registry: &mut ModlistRegistry,
) -> Result<Option<DeleteTarget>, RegistryError> {
    let Some(pos) = registry.entries.iter().position(|e| e.id == id) else {
        return Ok(None);
    };

    let name = registry.entries[pos].name.clone();
    let dest = registry.entries[pos].destination_folder.clone();

    registry.entries.remove(pos);
    store.save(registry)?;

    if is_safe_install_folder(&dest) {
        Ok(Some(DeleteTarget {
            name,
            dest: PathBuf::from(dest.trim()),
        }))
    } else {
        Ok(None)
    }
}

/// Spawns a background thread that runs `fs::remove_dir_all` on `dest`.
///
/// The caller receives a channel that yields exactly one `FolderDeleteResult`
/// when the operation completes.
#[must_use]
pub fn spawn_delete_folder_worker(dest: PathBuf) -> FolderDeleteReceiver {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = std::fs::remove_dir_all(&dest)
            .map_err(|e| format!("remove_dir_all({}): {e}", dest.display()));
        let _ = tx.send(result);
    });
    rx
}

fn is_safe_install_folder(dest: &str) -> bool {
    let trimmed = dest.trim();
    if trimmed.is_empty() {
        return false;
    }
    let path = Path::new(trimmed);

    if !path.is_absolute() {
        return false;
    }

    match path.parent() {
        None => return false,
        Some(parent) if parent.as_os_str().is_empty() => return false,
        Some(_) => {}
    }

    path.is_dir()
}

#[must_use]
pub fn share_code_for(id: &str, registry: &ModlistRegistry) -> Option<String> {
    registry.find(id).and_then(|e| e.latest_share_code.clone())
}

pub fn open_install_folder(entry: &ModlistEntry) -> Result<(), String> {
    let dest = entry.destination_folder.trim();
    if dest.is_empty() {
        return Err(format!("\"{}\" has no install folder set yet.", entry.name));
    }
    let path = Path::new(dest);
    if !path.is_dir() {
        return Err(format!(
            "Install folder for \"{}\" not found on disk: {dest}",
            entry.name
        ));
    }

    open_path_in_file_manager(path).map_err(|e| {
        format!(
            "Couldn't open the install folder for \"{}\": {e}",
            entry.name
        )
    })
}

fn open_path_in_file_manager(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };

    cmd.spawn().map(|_child| ())
}

#[must_use]
pub fn queue_reinstall_stub(_id: &str, _registry: &ModlistRegistry) -> String {
    "Reinstall queued \u{2014} install runtime arrives in Phase 7".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::model::{Game, ModlistEntry, ModlistRegistry, ModlistState};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_registry_path(label: &str) -> std::path::PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bio_ops_test_{}_{}_{}_modlists.json",
            std::process::id(),
            n,
            label
        ))
    }

    fn entry(id: &str, dest: &str) -> ModlistEntry {
        ModlistEntry {
            id: id.to_string(),
            name: format!("modlist-{id}"),
            game: Game::EET,
            destination_folder: dest.to_string(),
            state: ModlistState::InProgress,
            ..Default::default()
        }
    }

    #[test]
    fn guard_rejects_empty_string() {
        assert!(!is_safe_install_folder(""));
        assert!(!is_safe_install_folder("   "));
    }

    #[test]
    fn guard_rejects_relative_path() {
        assert!(!is_safe_install_folder("modlists/foo"));
        assert!(!is_safe_install_folder("./foo"));
        assert!(!is_safe_install_folder("foo"));
    }

    #[test]
    fn guard_rejects_filesystem_root() {
        #[cfg(windows)]
        {
            assert!(!is_safe_install_folder("C:\\"));
            assert!(!is_safe_install_folder("\\\\server\\share"));
        }
        #[cfg(not(windows))]
        {
            assert!(!is_safe_install_folder("/"));
        }
    }

    #[test]
    fn guard_rejects_missing_absolute_path() {
        let missing = std::env::temp_dir().join(format!(
            "bio_ops_definitely_missing_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!missing.exists());
        assert!(!is_safe_install_folder(missing.to_str().unwrap()));
    }

    #[test]
    fn guard_accepts_existing_absolute_dir() {
        let dir = std::env::temp_dir().join(format!(
            "bio_ops_real_dir_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(is_safe_install_folder(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn share_code_for_returns_code_when_present() {
        let mut reg = ModlistRegistry::default();
        let mut e = entry("EEE000000000", "");
        e.latest_share_code = Some("BIO-MODLIST-V1:abc".to_string());
        reg.entries.push(e);
        assert_eq!(
            share_code_for("EEE000000000", &reg),
            Some("BIO-MODLIST-V1:abc".to_string())
        );
    }

    #[test]
    fn share_code_for_none_when_absent_or_unset() {
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("FFF000000000", ""));
        assert_eq!(share_code_for("FFF000000000", &reg), None);
        assert_eq!(share_code_for("NOPE00000000", &reg), None);
    }

    #[test]
    fn open_install_folder_errors_on_empty_dest_no_spawn() {
        let e = entry("GGG000000000", "");
        let res = open_install_folder(&e);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("no install folder"));
    }

    #[test]
    fn open_install_folder_errors_on_missing_dir() {
        let missing = std::env::temp_dir().join(format!(
            "bio_ops_open_missing_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let e = entry("HHH000000000", missing.to_str().unwrap());
        let res = open_install_folder(&e);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not found on disk"));
    }

    #[test]
    fn reinstall_stub_message_is_phase7_placeholder() {
        let reg = ModlistRegistry::default();
        assert_eq!(
            queue_reinstall_stub("X", &reg),
            "Reinstall queued \u{2014} install runtime arrives in Phase 7"
        );
    }

    // --- remove_entry_and_save / spawn_delete_folder_worker tests ---

    #[test]
    fn remove_entry_and_save_returns_none_for_empty_dest() {
        let path = tmp_registry_path("reas_empty");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("AAA111000000", ""));
        reg.entries.push(entry("BBB111000000", ""));

        let target = remove_entry_and_save("AAA111000000", &store, &mut reg).expect("ok");
        assert!(target.is_none(), "empty dest should yield None (no folder work)");
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.entries[0].id, "BBB111000000");

        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded.entries.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_entry_and_save_returns_none_for_relative_dest() {
        let path = tmp_registry_path("reas_rel");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("CCC111000000", "relative/path"));

        let target = remove_entry_and_save("CCC111000000", &store, &mut reg).expect("ok");
        assert!(target.is_none(), "relative dest should yield None");
        assert!(reg.entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_entry_and_save_returns_some_for_existing_absolute_dir() {
        let path = tmp_registry_path("reas_existing");
        let store = RegistryStore::new_with_path(&path);

        let install_dir = std::env::temp_dir().join(format!(
            "bio_reas_dir_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&install_dir).unwrap();

        let mut reg = ModlistRegistry::default();
        reg.entries
            .push(entry("DDD111000000", install_dir.to_str().unwrap()));

        let target = remove_entry_and_save("DDD111000000", &store, &mut reg).expect("ok");
        let t = target.expect("existing abs dir should yield Some(DeleteTarget)");
        assert_eq!(t.name, "modlist-DDD111000000");
        assert_eq!(t.dest, install_dir);
        assert!(reg.entries.is_empty(), "entry removed from in-memory registry");

        let reloaded = store.load().expect("reload");
        assert!(reloaded.entries.is_empty(), "entry persisted as removed");

        let _ = std::fs::remove_dir_all(&install_dir);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_entry_and_save_returns_none_for_unknown_id() {
        let path = tmp_registry_path("reas_unknown");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("EEE111000000", ""));

        let target = remove_entry_and_save("ZZZ999999999", &store, &mut reg).expect("ok");
        assert!(target.is_none(), "unknown id yields None (entry not found)");
        assert_eq!(reg.entries.len(), 1, "registry unchanged for unknown id");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn spawn_delete_folder_worker_removes_dir_and_signals_ok() {
        let dir = std::env::temp_dir().join(format!(
            "bio_spawn_del_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub").join("file.txt"), b"x").unwrap();
        assert!(dir.is_dir());

        let rx = spawn_delete_folder_worker(dir.clone());
        let result = rx.recv().expect("worker sends exactly one result");
        assert!(result.is_ok(), "worker removal should succeed: {result:?}");
        assert!(!dir.exists(), "directory removed by worker");
    }

    #[test]
    fn spawn_delete_folder_worker_signals_error_for_nonexistent_path() {
        let missing = std::env::temp_dir().join(format!(
            "bio_spawn_del_miss_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!missing.exists());

        let rx = spawn_delete_folder_worker(missing);
        let result = rx.recv().expect("worker sends exactly one result");
        assert!(result.is_err(), "nonexistent path should produce an error");
    }

    #[test]
    fn multiple_concurrent_delete_workers_each_signal_independently() {
        let jobs: Vec<_> = (0..3)
            .map(|i| {
                let d = std::env::temp_dir().join(format!(
                    "bio_spawn_concurrent_{}_{}_{i}",
                    std::process::id(),
                    TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
                ));
                std::fs::create_dir_all(&d).unwrap();
                std::fs::write(d.join("f.txt"), b"y").unwrap();
                let rx = spawn_delete_folder_worker(d.clone());
                (rx, d)
            })
            .collect();

        for (rx, dir) in jobs {
            let result = rx.recv().expect("each worker signals");
            assert!(result.is_ok(), "concurrent delete succeeded: {result:?}");
            assert!(!dir.exists(), "dir removed: {}", dir.display());
        }
    }
}
