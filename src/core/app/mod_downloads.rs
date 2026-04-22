// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;

use crate::platform_defaults::app_config_file;

const MOD_DOWNLOADS_USER_FILE_NAME: &str = "mod_downloads_user.toml";
const MOD_DOWNLOADS_DEFAULT_FILE_NAME: &str = "mod_downloads_default.toml";

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadsFile {
    #[serde(default)]
    mods: Vec<ModDownloadSourceOverlay>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ModDownloadSourceOverlay {
    pub(crate) name: Option<String>,
    pub(crate) tp2: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) github: Option<String>,
    pub(crate) exact_github: Option<Vec<String>>,
    pub(crate) channel: Option<String>,
    pub(crate) pkg: Option<String>,
    pub(crate) pkg_windows: Option<String>,
    pub(crate) pkg_linux: Option<String>,
    pub(crate) pkg_macos: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ModDownloadSource {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) tp2: String,
    #[serde(default)]
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) github: Option<String>,
    #[serde(default)]
    pub(crate) exact_github: Vec<String>,
    #[serde(default)]
    pub(crate) channel: Option<String>,
    #[serde(default)]
    pub(crate) pkg: Option<String>,
    #[serde(default)]
    pub(crate) pkg_windows: Option<String>,
    #[serde(default)]
    pub(crate) pkg_linux: Option<String>,
    #[serde(default)]
    pub(crate) pkg_macos: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ModDownloadsLoad {
    pub(crate) sources: Vec<ModDownloadSource>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ModDownloadsOverlayLoad {
    sources: Vec<ModDownloadSourceOverlay>,
    error: Option<String>,
}

impl ModDownloadsLoad {
    pub(crate) fn find_source(&self, tp2: &str) -> Option<ModDownloadSource> {
        let key = normalize_mod_download_tp2(tp2);
        self.sources
            .iter()
            .find(|source| normalize_mod_download_tp2(&source.tp2) == key)
            .cloned()
    }
}

pub(crate) fn mod_downloads_user_path() -> PathBuf {
    app_config_file(MOD_DOWNLOADS_USER_FILE_NAME, "config")
}

pub(crate) fn mod_downloads_default_path() -> PathBuf {
    app_config_file(MOD_DOWNLOADS_DEFAULT_FILE_NAME, "config")
}

pub(crate) fn ensure_mod_downloads_files() -> io::Result<()> {
    let default_path = mod_downloads_default_path();
    let user_path = mod_downloads_user_path();

    if let Some(parent) = default_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_if_changed(&default_path, default_mod_downloads_content())?;

    if !user_path.exists() {
        fs::write(&user_path, user_mod_downloads_content())?;
    }

    Ok(())
}

pub(crate) fn load_mod_download_sources() -> ModDownloadsLoad {
    let default_path = mod_downloads_default_path();
    let user_path = mod_downloads_user_path();
    let mut by_tp2 = BTreeMap::<String, ModDownloadSource>::new();
    let default_load = load_source_overlays_from_path(&default_path);
    let user_load = load_source_overlays_from_path(&user_path);

    for overlay in default_load.sources {
        let key = overlay_tp2_key(&overlay);
        if !key.is_empty() {
            let mut source = ModDownloadSource::default();
            apply_source_overlay(&mut source, overlay);
            normalize_source(&mut source);
            if !source_is_valid(&source) {
                continue;
            }
            by_tp2.insert(key, source);
        }
    }
    for overlay in user_load.sources {
        let key = overlay_tp2_key(&overlay);
        if key.is_empty() {
            continue;
        }
        let mut source = by_tp2.remove(&key).unwrap_or_default();
        apply_source_overlay(&mut source, overlay);
        normalize_source(&mut source);
        if !source_is_valid(&source) {
            continue;
        }
        by_tp2.insert(key, source);
    }

    ModDownloadsLoad {
        sources: by_tp2.into_values().collect(),
        error: merge_load_errors(default_load.error, user_load.error),
    }
}

pub(crate) fn normalize_mod_download_tp2(value: &str) -> String {
    let replaced = value.replace('\\', "/").to_ascii_lowercase();
    let file = replaced.rsplit('/').next().unwrap_or(&replaced).trim();
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}

pub(crate) fn source_is_auto_resolvable(source: &ModDownloadSource) -> bool {
    source.github.is_some()
        || is_direct_archive_url(&source.url)
        || source_is_sentrizeal_download_url(&source.url)
        || source_is_page_archive_url(&source.url)
}

pub(crate) fn preferred_pkg_for_current_platform(source: &ModDownloadSource) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        source.pkg_windows.clone().or_else(|| source.pkg.clone())
    }
    #[cfg(target_os = "linux")]
    {
        source.pkg_linux.clone().or_else(|| source.pkg.clone())
    }
    #[cfg(target_os = "macos")]
    {
        source.pkg_macos.clone().or_else(|| source.pkg.clone())
    }
}

fn is_direct_archive_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    [
        ".zip", ".7z", ".rar", ".tar.gz", ".tgz", ".tar.bz2", ".tbz2", ".tar.xz", ".txz",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn source_is_sentrizeal_download_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://www.sentrizeal.com/downloaditm")
        || lower.starts_with("http://www.sentrizeal.com/downloaditm")
        || lower.starts_with("https://sentrizeal.com/downloaditm")
        || lower.starts_with("http://sentrizeal.com/downloaditm")
}

fn write_if_changed(path: &Path, content: &str) -> io::Result<()> {
    match fs::read_to_string(path) {
        Ok(existing) if existing == content => Ok(()),
        _ => fs::write(path, content),
    }
}

fn default_mod_downloads_content() -> &'static str {
    include_str!("../config/default_mod_downloads.toml")
}

fn user_mod_downloads_content() -> &'static str {
    include_str!("../config/user_mod_downloads.toml")
}

pub(crate) fn source_is_weaselmods_page_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://downloads.weaselmods.net/download/")
        || lower.starts_with("http://downloads.weaselmods.net/download/")
}

pub(crate) fn source_is_morpheus_mart_page_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    lower.starts_with("https://www.morpheus-mart.com/")
        || lower.starts_with("http://www.morpheus-mart.com/")
        || lower.starts_with("https://morpheus-mart.com/")
        || lower.starts_with("http://morpheus-mart.com/")
}

pub(crate) fn source_is_page_archive_url(url: &str) -> bool {
    source_is_weaselmods_page_url(url) || source_is_morpheus_mart_page_url(url)
}

fn load_source_overlays_from_path(path: &Path) -> ModDownloadsOverlayLoad {
    let content = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            return ModDownloadsOverlayLoad {
                sources: Vec::new(),
                error: Some(format!(
                    "mod downloads load failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    let parsed = match toml::from_str::<ModDownloadsFile>(&content) {
        Ok(value) => value,
        Err(err) => {
            return ModDownloadsOverlayLoad {
                sources: Vec::new(),
                error: Some(format!(
                    "mod downloads parse failed for {}: {err}",
                    path.display()
                )),
            };
        }
    };
    ModDownloadsOverlayLoad {
        sources: parsed.mods,
        error: None,
    }
}

fn overlay_tp2_key(source: &ModDownloadSourceOverlay) -> String {
    source
        .tp2
        .as_deref()
        .map(normalize_mod_download_tp2)
        .unwrap_or_default()
}

fn apply_source_overlay(target: &mut ModDownloadSource, overlay: ModDownloadSourceOverlay) {
    if let Some(name) = overlay.name {
        target.name = name;
    }
    if let Some(tp2) = overlay.tp2 {
        target.tp2 = tp2;
    }
    if let Some(url) = overlay.url {
        target.url = url;
    }
    if let Some(github) = overlay.github {
        target.github = Some(github);
    }
    if let Some(exact_github) = overlay.exact_github {
        target.exact_github = exact_github;
    }
    if let Some(channel) = overlay.channel {
        target.channel = Some(channel);
    }
    if let Some(pkg) = overlay.pkg {
        target.pkg = Some(pkg);
    }
    if let Some(pkg_windows) = overlay.pkg_windows {
        target.pkg_windows = Some(pkg_windows);
    }
    if let Some(pkg_linux) = overlay.pkg_linux {
        target.pkg_linux = Some(pkg_linux);
    }
    if let Some(pkg_macos) = overlay.pkg_macos {
        target.pkg_macos = Some(pkg_macos);
    }
}

fn normalize_source(source: &mut ModDownloadSource) {
    source.name = source.name.trim().to_string();
    source.tp2 = source.tp2.trim().to_string();
    source.url = source.url.trim().to_string();
    source.github = source
        .github
        .take()
        .map(|github| github.trim().to_string())
        .filter(|github| !github.is_empty());
    source.exact_github = source
        .exact_github
        .iter()
        .map(|github| github.trim().to_string())
        .filter(|github| !github.is_empty())
        .collect();
    if let Some(primary) = source.github.as_deref() {
        source
            .exact_github
            .retain(|github| !github.eq_ignore_ascii_case(primary));
    }
    source.channel = source
        .channel
        .take()
        .map(|channel| channel.trim().to_string())
        .filter(|channel| !channel.is_empty());
    source.pkg = source
        .pkg
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
    source.pkg_windows = source
        .pkg_windows
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
    source.pkg_linux = source
        .pkg_linux
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
    source.pkg_macos = source
        .pkg_macos
        .take()
        .map(|pkg| pkg.trim().to_string())
        .filter(|pkg| !pkg.is_empty());
}

fn source_is_valid(source: &ModDownloadSource) -> bool {
    !normalize_mod_download_tp2(&source.tp2).is_empty() && !source.url.is_empty()
}

fn merge_load_errors(left: Option<String>, right: Option<String>) -> Option<String> {
    match (left, right) {
        (Some(left), Some(right)) => Some(format!("{left} | {right}")),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}
