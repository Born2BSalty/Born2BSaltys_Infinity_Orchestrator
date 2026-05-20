// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::registry::errors::RegistryError;
use crate::registry::model::{ModlistEntry, ModlistRegistry};
use crate::registry::store::RegistryStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteOutcome {
    EntryAndFolderRemoved,

    EntryRemovedFolderSkipped,

    EntryRemovedFolderError(String),
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

pub fn delete_modlist(
    id: &str,
    store: &RegistryStore,
    registry: &mut ModlistRegistry,
) -> Result<Option<DeleteOutcome>, RegistryError> {
    let Some(pos) = registry.entries.iter().position(|e| e.id == id) else {
        return Ok(None);
    };

    let dest = registry.entries[pos].destination_folder.clone();

    registry.entries.remove(pos);

    let outcome = if is_safe_install_folder(&dest) {
        match std::fs::remove_dir_all(dest.trim()) {
            Ok(()) => DeleteOutcome::EntryAndFolderRemoved,
            Err(err) => DeleteOutcome::EntryRemovedFolderError(err.to_string()),
        }
    } else {
        DeleteOutcome::EntryRemovedFolderSkipped
    };

    store.save(registry)?;

    Ok(Some(outcome))
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
    fn delete_empty_dest_removes_entry_no_fs_op() {
        let path = tmp_registry_path("empty_dest");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("AAA000000000", ""));
        reg.entries.push(entry("BBB000000000", ""));

        let out = delete_modlist("AAA000000000", &store, &mut reg).expect("delete ok");
        assert_eq!(out, Some(DeleteOutcome::EntryRemovedFolderSkipped));
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.entries[0].id, "BBB000000000");

        let reloaded = store.load().expect("reload");
        assert_eq!(reloaded.entries.len(), 1);
        assert_eq!(reloaded.entries[0].id, "BBB000000000");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn delete_unknown_id_is_noop() {
        let path = tmp_registry_path("unknown");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("AAA000000000", ""));
        let out = delete_modlist("ZZZ999999999", &store, &mut reg).expect("delete ok");
        assert_eq!(out, None);
        assert_eq!(reg.entries.len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn delete_real_folder_removes_entry_and_folder() {
        let path = tmp_registry_path("real_folder");
        let store = RegistryStore::new_with_path(&path);

        let install_dir = std::env::temp_dir().join(format!(
            "bio_ops_install_{}_{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(install_dir.join("nested")).unwrap();
        std::fs::write(install_dir.join("nested").join("f.txt"), b"x").unwrap();
        assert!(install_dir.is_dir());

        let mut reg = ModlistRegistry::default();
        reg.entries
            .push(entry("CCC000000000", install_dir.to_str().unwrap()));

        let out = delete_modlist("CCC000000000", &store, &mut reg).expect("delete ok");
        assert_eq!(out, Some(DeleteOutcome::EntryAndFolderRemoved));
        assert!(reg.entries.is_empty());
        assert!(!install_dir.exists(), "install folder recursively removed");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn delete_relative_dest_skips_fs_op() {
        let path = tmp_registry_path("rel_dest");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("DDD000000000", "some/relative/dir"));
        let out = delete_modlist("DDD000000000", &store, &mut reg).expect("delete ok");
        assert_eq!(out, Some(DeleteOutcome::EntryRemovedFolderSkipped));
        assert!(reg.entries.is_empty());
        let _ = std::fs::remove_file(&path);
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
}
