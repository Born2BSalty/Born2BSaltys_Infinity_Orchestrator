// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;

use serde::{Deserialize, Serialize};

use crate::app::mod_downloads::normalize_mod_download_tp2;
use crate::platform_defaults::app_config_file;

const MOD_SOURCE_REFS_FILE_NAME: &str = "mod_installed_refs.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
struct ModSourceRefsFile {
    #[serde(default)]
    refs: BTreeMap<String, String>,
    #[serde(default)]
    sources: BTreeMap<String, String>,
}

pub(crate) fn installed_source_refs_path() -> std::path::PathBuf {
    app_config_file(MOD_SOURCE_REFS_FILE_NAME, "config")
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

pub(super) fn save_installed_source_ref(tp2: &str, source_ref: &str) -> io::Result<()> {
    let path = installed_source_refs_path();
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

pub(super) fn load_installed_source_id(tp2: &str) -> Option<String> {
    let content = fs::read_to_string(installed_source_refs_path()).ok()?;
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).ok()?;
    parsed
        .sources
        .get(&normalize_mod_download_tp2(tp2))
        .cloned()
}

pub(crate) fn load_installed_source_ids() -> BTreeMap<String, String> {
    let content = match fs::read_to_string(installed_source_refs_path()) {
        Ok(content) => content,
        Err(_) => return BTreeMap::new(),
    };
    let parsed = toml::from_str::<ModSourceRefsFile>(&content).unwrap_or_default();
    parsed
        .sources
        .into_iter()
        .map(|(tp2, source_id)| (normalize_mod_download_tp2(&tp2), source_id))
        .filter(|(tp2, source_id)| !tp2.is_empty() && !source_id.trim().is_empty())
        .collect()
}

pub(super) fn save_installed_source_id(tp2: &str, source_id: &str) -> io::Result<()> {
    let path = installed_source_refs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut refs = match fs::read_to_string(&path) {
        Ok(value) => toml::from_str::<ModSourceRefsFile>(&value).unwrap_or_default(),
        Err(_) => ModSourceRefsFile::default(),
    };
    refs.sources.insert(
        normalize_mod_download_tp2(tp2),
        source_id.trim().to_string(),
    );
    let content = toml::to_string_pretty(&refs).map_err(io::Error::other)?;
    fs::write(path, content)
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
