// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::platform_defaults::app_config_file;
use crate::ui::controller::util::current_exe_fingerprint;

use super::{SCAN_CACHE_FILE, SCAN_CACHE_VERSION, ScannedComponent};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanCache {
    pub version: u32,
    #[serde(default)]
    pub writer_app_version: String,
    #[serde(default)]
    pub writer_exe_fingerprint: String,
    pub entries: BTreeMap<String, ScanCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCacheEntry {
    pub context: String,
    pub mtime_secs: u64,
    pub size: u64,
    pub components: Vec<ScannedComponent>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanCacheLoadMeta {
    pub path: String,
    pub source: String,
    pub file_exists: bool,
    pub file_mtime_secs: Option<u64>,
    pub file_version: Option<u32>,
    pub file_writer_app_version: Option<String>,
    pub file_writer_exe_fingerprint: Option<String>,
    pub file_entry_count: usize,
    pub version_matches_current_schema: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedScanCache {
    pub cache: ScanCache,
    pub meta: ScanCacheLoadMeta,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawScanCacheMeta {
    version: Option<u32>,
    #[serde(default)]
    writer_app_version: String,
    #[serde(default)]
    writer_exe_fingerprint: String,
    #[serde(default)]
    entries: BTreeMap<String, serde_json::Value>,
}

pub fn normalize_context_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
}

pub fn cache_context(weidu: &Path, game_dir: &Path, mods_root: &Path) -> String {
    format!(
        "v={SCAN_CACHE_VERSION}|weidu={}|game={}|mods={}",
        normalize_context_path(weidu),
        normalize_context_path(game_dir),
        normalize_context_path(mods_root)
    )
}

pub fn load_scan_cache() -> LoadedScanCache {
    let primary = scan_cache_path();
    if let Some(loaded) = try_load_cache(&primary, "appdata") {
        return loaded;
    }
    let legacy = PathBuf::from(SCAN_CACHE_FILE);
    if let Some(loaded) = try_load_cache(&legacy, "legacy_local") {
        return loaded;
    }
    LoadedScanCache {
        cache: default_scan_cache(),
        meta: ScanCacheLoadMeta {
            path: primary.display().to_string(),
            source: "appdata".to_string(),
            ..ScanCacheLoadMeta::default()
        },
    }
}

pub fn save_scan_cache(cache: &ScanCache) {
    if let Ok(text) = serde_json::to_string(&cache_for_write(cache)) {
        let path = scan_cache_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, text);
    }
}

fn cache_key(path: &Path) -> String {
    normalize_context_path(path)
}

fn file_signature(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some((secs, meta.len()))
}

pub fn cache_get(
    cache: &Arc<Mutex<ScanCache>>,
    context: &str,
    tp2: &Path,
) -> Option<Vec<ScannedComponent>> {
    let key = cache_key(tp2);
    let (mtime_secs, size) = file_signature(tp2)?;
    let cache = cache.lock().ok()?;
    let entry = cache.entries.get(&key)?;
    if entry.context == context && entry.mtime_secs == mtime_secs && entry.size == size {
        return Some(entry.components.clone());
    }
    None
}

pub fn cache_put(
    cache: &Arc<Mutex<ScanCache>>,
    context: &str,
    tp2: &Path,
    components: Vec<ScannedComponent>,
) {
    let Some((mtime_secs, size)) = file_signature(tp2) else {
        return;
    };
    let key = cache_key(tp2);
    if let Ok(mut cache) = cache.lock() {
        cache.entries.insert(
            key,
            ScanCacheEntry {
                context: context.to_string(),
                mtime_secs,
                size,
                components,
            },
        );
    }
}

fn scan_cache_path() -> PathBuf {
    app_config_file(SCAN_CACHE_FILE, ".")
}

fn try_load_cache(path: &Path, source: &str) -> Option<LoadedScanCache> {
    let text = fs::read_to_string(path).ok()?;
    let meta = inspect_cache_text(path, source, &text);
    let cache = parse_cache_text(&text)?;
    Some(LoadedScanCache { cache, meta })
}

fn inspect_cache_text(path: &Path, source: &str, text: &str) -> ScanCacheLoadMeta {
    let raw = serde_json::from_str::<RawScanCacheMeta>(text).ok();
    let file_mtime_secs = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let file_version = raw.as_ref().and_then(|r| r.version);
    let file_writer_app_version = raw
        .as_ref()
        .and_then(|r| non_empty(&r.writer_app_version));
    let file_writer_exe_fingerprint = raw
        .as_ref()
        .and_then(|r| non_empty(&r.writer_exe_fingerprint));
    let file_entry_count = raw.as_ref().map(|r| r.entries.len()).unwrap_or(0);
    ScanCacheLoadMeta {
        path: path.display().to_string(),
        source: source.to_string(),
        file_exists: true,
        file_mtime_secs,
        file_version,
        file_writer_app_version,
        file_writer_exe_fingerprint,
        file_entry_count,
        version_matches_current_schema: file_version == Some(SCAN_CACHE_VERSION),
    }
}

fn default_scan_cache() -> ScanCache {
    ScanCache {
        version: SCAN_CACHE_VERSION,
        writer_app_version: env!("CARGO_PKG_VERSION").to_string(),
        writer_exe_fingerprint: current_exe_fingerprint(),
        entries: BTreeMap::new(),
    }
}

fn cache_for_write(cache: &ScanCache) -> ScanCache {
    let mut out = cache.clone();
    out.version = SCAN_CACHE_VERSION;
    out.writer_app_version = env!("CARGO_PKG_VERSION").to_string();
    out.writer_exe_fingerprint = current_exe_fingerprint();
    out
}

fn parse_cache_text(text: &str) -> Option<ScanCache> {
    let cache = serde_json::from_str::<ScanCache>(text).ok()?;
    if cache.version != SCAN_CACHE_VERSION {
        return None;
    }
    Some(cache)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
