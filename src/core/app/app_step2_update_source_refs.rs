// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::platform_defaults::app_config_file;

const MOD_SOURCE_REFS_FILE_NAME: &str = "mod_source_refs.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
struct ModSourceRefsFile {
    #[serde(default)]
    refs: BTreeMap<String, String>,
}

pub(super) fn load_installed_source_ref(tp2: &str) -> Option<String> {
    let content = fs::read_to_string(mod_source_refs_path()).ok()?;
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).ok()?;
    parsed.refs.get(&normalize_mod_download_tp2(tp2)).cloned()
}

pub(super) fn save_installed_source_ref(tp2: &str, source_ref: &str) -> io::Result<()> {
    let path = mod_source_refs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = match fs::read_to_string(&path) {
        Ok(value) => toml::from_str::<ModSourceRefsFile>(&value).unwrap_or_default(),
        Err(_) => ModSourceRefsFile::default(),
    };
    refs.refs.insert(
        normalize_mod_download_tp2(tp2),
        source_ref.trim().to_string(),
    );
    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(path, content)
}

fn mod_source_refs_path() -> std::path::PathBuf {
    app_config_file(MOD_SOURCE_REFS_FILE_NAME, "config")
}
