// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::app::state::Step2ModState;
use crate::platform_defaults::app_config_file;

const MOD_UPDATE_LOCKS_FILE_NAME: &str = "mod_update_locks.toml";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ModUpdateLocksFile {
    #[serde(default)]
    locked_tp2: Vec<String>,
}

pub(crate) fn mod_update_locks_path() -> PathBuf {
    app_config_file(MOD_UPDATE_LOCKS_FILE_NAME, "config")
}

pub(crate) fn apply_mod_update_locks(mods: &mut [Step2ModState]) {
    let locked_tp2 = match load_mod_update_locks() {
        Ok(locked_tp2) => locked_tp2,
        Err(err) => {
            set_last_load_error(Some(err));
            return;
        }
    };
    set_last_load_error(None);
    for mod_state in mods {
        mod_state.update_locked =
            locked_tp2.contains(&normalize_mod_download_tp2(&mod_state.tp_file));
        if mod_state.update_locked {
            mod_state.package_marker = None;
        }
    }
}

pub(crate) fn set_mod_update_lock(tp2: &str, locked: bool) -> io::Result<()> {
    let key = normalize_mod_download_tp2(tp2);
    if key.is_empty() {
        return Ok(());
    }
    let mut locked_tp2 = load_mod_update_locks().unwrap_or_default();
    if locked {
        locked_tp2.insert(key);
    } else {
        locked_tp2.remove(&key);
    }
    save_mod_update_locks(&locked_tp2)
}

pub(crate) fn clear_mod_update_locks() -> io::Result<()> {
    match fs::remove_file(mod_update_locks_path()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

pub(crate) fn take_last_load_error() -> Option<String> {
    last_load_error()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

fn load_mod_update_locks() -> Result<BTreeSet<String>, String> {
    let path = mod_update_locks_path();
    let content = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(err) => {
            return Err(format!(
                "mod update locks load failed for {}: {err}",
                path.display()
            ));
        }
    };
    let parsed = match toml::from_str::<ModUpdateLocksFile>(&content) {
        Ok(value) => value,
        Err(err) => {
            return Err(format!(
                "mod update locks parse failed for {}: {err}",
                path.display()
            ));
        }
    };
    Ok(parsed
        .locked_tp2
        .into_iter()
        .map(|value| normalize_mod_download_tp2(&value))
        .filter(|value| !value.is_empty())
        .collect())
}

fn save_mod_update_locks(locked_tp2: &BTreeSet<String>) -> io::Result<()> {
    let path = mod_update_locks_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(&ModUpdateLocksFile {
        locked_tp2: locked_tp2.iter().cloned().collect(),
    })
    .map_err(io::Error::other)?;
    fs::write(path, content)
}

fn last_load_error() -> &'static Mutex<Option<String>> {
    static LAST_LOAD_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LAST_LOAD_ERROR.get_or_init(|| Mutex::new(None))
}

fn set_last_load_error(error: Option<String>) {
    if let Ok(mut guard) = last_load_error().lock() {
        *guard = error;
    }
}
