// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `operations` — high-level registry write helpers used by Home (Phase 5
// Run 2: P5.T7 / P5.T17 / P5.T18).
//
// Per H6 these are plain `pub fn` taking `&mut ModlistRegistry +
// &RegistryStore` — no write-guard wrapper (egui is single-threaded; the
// borrow checker enforces single-mutator; atomic file writes handle disk
// safety).
//
// Functions:
//   - `delete_modlist`        — P5.T7: remove the registry entry **and** the
//                                on-disk install folder (recursive), then
//                                persist the registry immediately. SPEC §3.1
//                                (Delete semantics).
//   - `share_code_for`        — returns the entry's `latest_share_code` for
//                                the UI to copy via `ui.ctx().copy_text(...)`
//                                (no clipboard crate — egui built-in).
//   - `open_install_folder`   — P5.T17: open the entry's destination folder
//                                in the OS file manager. SPEC §3.2.
//   - `queue_reinstall_stub`  — P5.T18: preview-only placeholder; the real
//                                reinstall route lands in Phase 7.
//
// ── CRITICAL SAFETY: `remove_dir_all` guard ──
// `delete_modlist` MUST NOT recursively delete an empty / relative / root /
// non-existent path. The Phase-3 dev-seed entries carry
// `destination_folder: ""`; an unguarded `std::fs::remove_dir_all("")` (or on
// a relative path, or `/`, or `C:\`) is catastrophic and irreversible. The
// folder removal is attempted **only** when the path is, in order:
//   1. non-empty,
//   2. absolute,
//   3. not a filesystem root / prefix-only path (has a parent),
//   4. an existing directory on disk.
// If any check fails, `delete_modlist` does a registry-entry-only delete (no
// filesystem operation at all) and reports it via `DeleteOutcome`.
//
// SPEC: §3.1 (Delete / Reinstall semantics), §3.2 (Open install folder),
// §13.1.

use std::path::Path;

use crate::registry::errors::RegistryError;
use crate::registry::model::{ModlistEntry, ModlistRegistry};
use crate::registry::store::RegistryStore;

/// Result of a `delete_modlist` call — surfaces whether the on-disk folder
/// was actually removed so the caller can log / message accurately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteOutcome {
    /// Entry removed; the on-disk install folder was also recursively deleted.
    EntryAndFolderRemoved,
    /// Entry removed; no filesystem op ran because `destination_folder` failed
    /// the safety guard (empty / relative / root / missing). This is the
    /// normal path for in-progress builds / dev-seed entries that have no
    /// real install folder yet.
    EntryRemovedFolderSkipped,
    /// Entry removed; the folder existed and passed the guard but
    /// `remove_dir_all` failed (permission denied, in use, …). The registry
    /// delete still went through; the folder error is reported, not fatal.
    EntryRemovedFolderError(String),
}

/// Whether `dest` is safe to feed to `std::fs::remove_dir_all`.
///
/// Returns `true` ONLY when the path is non-empty, absolute, has a parent
/// (i.e. is not a filesystem root / bare prefix like `C:\` or `/`), and
/// currently exists as a directory on disk. Anything else → `false` (caller
/// must skip the filesystem operation entirely).
///
/// This is the single chokepoint guarding the irreversible recursive delete;
/// it is unit-tested below.
fn is_safe_install_folder(dest: &str) -> bool {
    // 1. Non-empty (dev-seed entries have `""`).
    let trimmed = dest.trim();
    if trimmed.is_empty() {
        return false;
    }
    let path = Path::new(trimmed);

    // 2. Absolute — never operate on a relative path (would resolve against
    //    the process CWD).
    if !path.is_absolute() {
        return false;
    }

    // 3. Must have a parent. A filesystem root (`/`) or a Windows
    //    prefix-only path (`C:\`, `\\server\share`) has no parent (or an
    //    empty one) — refuse to recursively delete a drive root.
    match path.parent() {
        None => return false,
        Some(parent) if parent.as_os_str().is_empty() => return false,
        Some(_) => {}
    }

    // 4. Must currently be an existing directory. (Missing → nothing to
    //    delete; a file → not our install folder.)
    path.is_dir()
}

/// Delete a modlist (P5.T7 / SPEC §3.1 Delete semantics).
///
/// Removes the registry entry for `id` and — **only if the entry's
/// `destination_folder` passes [`is_safe_install_folder`]** — recursively
/// deletes that folder. Then persists the registry to disk **immediately**
/// (atomic temp-file + rename via `RegistryStore::save`) so the deletion
/// survives even if the process exits before the debounced write cycle.
///
/// Returns `Ok(None)` if no entry with `id` exists (no-op). On a successful
/// delete returns `Ok(Some(DeleteOutcome))` describing what happened on disk.
/// A registry-persist failure returns `Err` (the in-memory entry is already
/// gone; the caller surfaces the error).
pub fn delete_modlist(
    id: &str,
    store: &RegistryStore,
    registry: &mut ModlistRegistry,
) -> Result<Option<DeleteOutcome>, RegistryError> {
    let Some(pos) = registry.entries.iter().position(|e| e.id == id) else {
        return Ok(None);
    };

    // Capture the destination before removing the entry.
    let dest = registry.entries[pos].destination_folder.clone();

    // Remove the registry entry first (the in-memory mutation is the part
    // that must always happen — SPEC §3.1: "the registry entry is removed").
    registry.entries.remove(pos);

    // Attempt the on-disk folder removal ONLY behind the safety guard.
    let outcome = if is_safe_install_folder(&dest) {
        match std::fs::remove_dir_all(dest.trim()) {
            Ok(()) => DeleteOutcome::EntryAndFolderRemoved,
            Err(err) => DeleteOutcome::EntryRemovedFolderError(err.to_string()),
        }
    } else {
        // Empty / relative / root / missing → registry-only delete, NO
        // filesystem operation. (Dev-seed entries + in-progress builds
        // without a destination land here.)
        DeleteOutcome::EntryRemovedFolderSkipped
    };

    // Persist immediately (atomic). SPEC §3.1: "the registry entry is removed
    // (debounced write to modlists.json)" — we write through directly so the
    // card disappears durably; the debounce cycle is a no-op afterwards
    // (in-memory == last-saved once the cycle's snapshot catches up).
    store.save(registry)?;

    Ok(Some(outcome))
}

/// The entry's last captured BIO-MODLIST-V1 share/import code, if any
/// (P5.T2 / SPEC §3.2 "Copy import code"). The actual clipboard write is done
/// by the UI caller via `ui.ctx().copy_text(code)` (egui built-in — no
/// clipboard crate; a ctx-less registry helper can't reach the clipboard).
///
/// Returns `None` when the entry doesn't exist or has no code yet (Phase 7
/// generates `latest_share_code` post-install; pre-Phase-7 in-progress
/// entries may legitimately have none).
pub fn share_code_for(id: &str, registry: &ModlistRegistry) -> Option<String> {
    registry.find(id).and_then(|e| e.latest_share_code.clone())
}

/// Open the modlist's install folder in the OS file manager (P5.T17 / SPEC
/// §3.2 "Open install folder").
///
/// SPEC §3.2: "If the folder no longer exists on disk (deleted externally),
/// surface an error message … do not attempt to open or recreate the
/// folder." We therefore validate the path up-front and **never spawn on an
/// empty / relative / missing path**. Returns `Ok(())` on a successful spawn;
/// `Err(message)` with a user-facing string otherwise (the caller surfaces it
/// via the bottom-of-screen toast in its error tone).
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

/// Platform-native "reveal this directory in the file manager". Never called
/// on an unvalidated path — `open_install_folder` gates this.
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

    // `explorer` returns a non-zero exit code even on success in some
    // configurations, so we only care that the process spawned. `status()`
    // would misreport; `spawn()` + drop is the right call here.
    cmd.spawn().map(|_child| ())
}

/// Preview-only Reinstall placeholder (P5.T18 / SPEC §3.1 Reinstall
/// semantics). Phase 5 does NOT perform a reinstall or route through the
/// Install-preview stage — that is Phase 7. This returns the toast text the
/// caller shows on confirm. Kept as a function (not an inline string) so the
/// Phase-7 wiring has an obvious single seam to replace.
pub fn queue_reinstall_stub(_id: &str, _registry: &ModlistRegistry) -> String {
    // Em dash (U+2014) — rendered in Poppins (Latin-Extended-A range; safe,
    // unlike symbol glyphs). Phase 7 replaces this with the real route.
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

    // ─────────────────── is_safe_install_folder guard ───────────────────

    #[test]
    fn guard_rejects_empty_string() {
        // The dev-seed entries have `destination_folder: ""` — this is the
        // single most important case: an unguarded remove_dir_all("") is
        // catastrophic.
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
        // Roots / bare prefixes have no parent — never recursively delete a
        // drive root.
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
        // Absolute + has-parent but does not exist → still skipped (nothing
        // to delete; never errors).
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

    // ─────────────────────── delete_modlist ───────────────────────

    #[test]
    fn delete_empty_dest_removes_entry_no_fs_op() {
        // The dev-seed / in-progress case: destination_folder == "". MUST be
        // a registry-only delete with NO filesystem operation and no panic.
        let path = tmp_registry_path("empty_dest");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("AAA000000000", ""));
        reg.entries.push(entry("BBB000000000", ""));

        let out = delete_modlist("AAA000000000", &store, &mut reg).expect("delete ok");
        assert_eq!(out, Some(DeleteOutcome::EntryRemovedFolderSkipped));
        assert_eq!(reg.entries.len(), 1);
        assert_eq!(reg.entries[0].id, "BBB000000000");

        // Registry persisted immediately + reloads without the entry.
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
        // A bogus relative destination must NEVER be remove_dir_all'd
        // (would resolve against CWD).
        let path = tmp_registry_path("rel_dest");
        let store = RegistryStore::new_with_path(&path);
        let mut reg = ModlistRegistry::default();
        reg.entries.push(entry("DDD000000000", "some/relative/dir"));
        let out = delete_modlist("DDD000000000", &store, &mut reg).expect("delete ok");
        assert_eq!(out, Some(DeleteOutcome::EntryRemovedFolderSkipped));
        assert!(reg.entries.is_empty());
        let _ = std::fs::remove_file(&path);
    }

    // ─────────────────────── share_code_for ───────────────────────

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
        reg.entries.push(entry("FFF000000000", "")); // no code
        assert_eq!(share_code_for("FFF000000000", &reg), None);
        assert_eq!(share_code_for("NOPE00000000", &reg), None);
    }

    // ─────────────────────── open_install_folder ───────────────────────

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

    // ─────────────────────── queue_reinstall_stub ───────────────────────

    #[test]
    fn reinstall_stub_message_is_phase7_placeholder() {
        let reg = ModlistRegistry::default();
        assert_eq!(
            queue_reinstall_stub("X", &reg),
            "Reinstall queued \u{2014} install runtime arrives in Phase 7"
        );
    }
}
