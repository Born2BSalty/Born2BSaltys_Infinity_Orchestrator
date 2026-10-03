// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::app::mod_downloads::{active_modlist_dir, normalize_mod_download_tp2};

const ADDED_MODS_FILE_NAME: &str = "added_mods.json";

pub(crate) fn added_mods_path() -> Option<PathBuf> {
    active_modlist_dir().map(|d| d.join(ADDED_MODS_FILE_NAME))
}

pub(crate) fn load_added_mods() -> BTreeSet<String> {
    let Some(path) = added_mods_path() else {
        return BTreeSet::new();
    };
    read_added_mods(&path).unwrap_or_else(|err| {
        warn!("{err}");
        BTreeSet::new()
    })
}

pub(crate) fn record_added_mod(key: &str) -> Result<(), String> {
    let key = normalize_mod_download_tp2(key);
    let Some(path) = added_mods_path() else {
        return Ok(());
    };
    if key.is_empty() {
        return Ok(());
    }
    let mut keys = read_added_mods(&path)?;
    if !keys.insert(key) {
        return Ok(());
    }
    write_added_mods(&path, &keys)
}

pub(crate) fn forget_added_mods_present<'a>(
    present_keys: impl IntoIterator<Item = &'a str>,
) -> Result<usize, String> {
    let Some(path) = added_mods_path() else {
        return Ok(0);
    };
    let mut keys = read_added_mods(&path)?;
    if keys.is_empty() {
        return Ok(0);
    }
    let mut removed = 0;
    for present in present_keys {
        if keys.remove(&normalize_mod_download_tp2(present)) {
            removed += 1;
        }
    }
    if removed > 0 {
        write_added_mods(&path, &keys)?;
    }
    Ok(removed)
}

fn read_added_mods(path: &Path) -> Result<BTreeSet<String>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(err) => {
            return Err(format!(
                "Load added mods failed for {}: {err}",
                path.display()
            ));
        }
    };
    let parsed = serde_json::from_str::<Vec<String>>(&content)
        .map_err(|err| format!("Parse added mods failed for {}: {err}", path.display()))?;
    Ok(parsed
        .iter()
        .map(|value| normalize_mod_download_tp2(value))
        .filter(|value| !value.is_empty())
        .collect())
}

fn write_added_mods(path: &Path, keys: &BTreeSet<String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Save added mods failed for {}: {err}", path.display()))?;
    }
    let raw = serde_json::to_string_pretty(keys)
        .map_err(|err| format!("Save added mods failed for {}: {err}", path.display()))?;
    fs::write(path, raw)
        .map_err(|err| format!("Save added mods failed for {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::app::mod_downloads::{AMBIENT_TEST_LOCK, set_active_modlist_dir};

    struct AmbientModlistDir {
        previous: Option<PathBuf>,
        root: PathBuf,
    }

    impl AmbientModlistDir {
        fn create(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "bio_added_mods_test_{}_{}_{label}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            let previous = active_modlist_dir();
            std::fs::create_dir_all(&root).expect("create temp modlist dir");
            set_active_modlist_dir(Some(root.clone()));
            Self { previous, root }
        }
    }

    impl Drop for AmbientModlistDir {
        fn drop(&mut self) {
            set_active_modlist_dir(self.previous.take());
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn record_and_load_round_trip() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = AmbientModlistDir::create("round_trip");

        assert_eq!(added_mods_path(), Some(dir.root.join("added_mods.json")));
        record_added_mod("Widget/Setup-Widget.TP2").expect("record widget");
        record_added_mod("widget").expect("record widget again");
        record_added_mod("gadget").expect("record gadget");

        assert_eq!(
            load_added_mods(),
            BTreeSet::from(["gadget".to_string(), "widget".to_string()])
        );
        let raw = std::fs::read_to_string(dir.root.join("added_mods.json")).expect("read file");
        let parsed: Vec<String> = serde_json::from_str(&raw).expect("json array");
        assert_eq!(parsed, vec!["gadget".to_string(), "widget".to_string()]);
    }

    #[test]
    fn forget_removes_only_present_keys() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = AmbientModlistDir::create("forget");
        record_added_mod("widget").expect("record widget");
        record_added_mod("gadget").expect("record gadget");
        let before = std::fs::metadata(dir.root.join("added_mods.json"))
            .and_then(|meta| meta.modified())
            .expect("file time");

        assert_eq!(forget_added_mods_present(["other", "thing"]), Ok(0));
        let after_noop = std::fs::metadata(dir.root.join("added_mods.json"))
            .and_then(|meta| meta.modified())
            .expect("file time");
        assert_eq!(before, after_noop);

        assert_eq!(
            forget_added_mods_present(["setup-WIDGET.tp2", "other"]),
            Ok(1)
        );
        assert_eq!(load_added_mods(), BTreeSet::from(["gadget".to_string()]));
    }

    #[test]
    fn missing_file_is_empty_and_parse_error_is_left_untouched() {
        let _lock = AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = AmbientModlistDir::create("parse_error");
        let path = dir.root.join("added_mods.json");

        assert_eq!(load_added_mods().len(), 0);
        assert_eq!(forget_added_mods_present(["widget"]), Ok(0));
        assert!(!path.exists());

        std::fs::write(&path, "{ not json").expect("write broken file");
        assert_eq!(load_added_mods().len(), 0);
        assert!(record_added_mod("widget").is_err());
        assert!(forget_added_mods_present(["widget"]).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).expect("read broken file"),
            "{ not json"
        );
    }
}
