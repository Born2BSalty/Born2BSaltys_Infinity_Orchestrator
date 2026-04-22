// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::app::controller::util::current_exe_fingerprint;
use crate::platform_defaults::app_config_file;

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
    pub load_status: String,
    pub load_error: Option<String>,
    pub file_exists: bool,
    pub file_mtime_secs: Option<u64>,
    pub file_version: Option<u32>,
    pub file_writer_app_version: Option<String>,
    pub file_writer_exe_fingerprint: Option<String>,
    pub file_entry_count: usize,
    pub version_matches_current_schema: bool,
    pub fallback_path: Option<String>,
    pub fallback_source: Option<String>,
    pub fallback_load_status: Option<String>,
    pub fallback_load_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedScanCache {
    pub cache: ScanCache,
    pub meta: ScanCacheLoadMeta,
}

type ScanCacheLoadMetaErr = Box<ScanCacheLoadMeta>;

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
    path.to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase()
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
    let primary_meta = match try_load_cache(&primary, "appdata") {
        Ok(loaded) => return loaded,
        Err(meta) => *meta,
    };
    let legacy = PathBuf::from(SCAN_CACHE_FILE);
    let legacy_result = if legacy != primary {
        Some(try_load_cache(&legacy, "legacy_local"))
    } else {
        None
    };
    if let Some(Ok(mut loaded)) = legacy_result {
        if primary_meta.file_exists || primary_meta.load_status != "missing" {
            loaded.meta.fallback_path = Some(primary_meta.path.clone());
            loaded.meta.fallback_source = Some(primary_meta.source.clone());
            loaded.meta.fallback_load_status = Some(primary_meta.load_status.clone());
            loaded.meta.fallback_load_error = primary_meta.load_error.clone();
        }
        return loaded;
    }
    let legacy_meta = legacy_result.and_then(Result::err).map(|meta| *meta);
    let meta = if primary_meta.file_exists || primary_meta.load_status != "missing" {
        primary_meta
    } else {
        legacy_meta.unwrap_or(primary_meta)
    };
    LoadedScanCache {
        cache: default_scan_cache(),
        meta,
    }
}

pub fn save_scan_cache(cache: &ScanCache) -> Option<String> {
    let text = match serde_json::to_string(&cache_for_write(cache)) {
        Ok(text) => text,
        Err(err) => return Some(format!("serialize failed: {err}")),
    };
    let path = scan_cache_path();
    if let Some(parent) = path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        return Some(format!(
            "create cache directory failed for {}: {err}",
            parent.display()
        ));
    }
    if let Err(err) = fs::write(&path, text) {
        return Some(format!(
            "write cache file failed for {}: {err}",
            path.display()
        ));
    }
    None
}

pub fn clear_scan_cache_files() -> Vec<String> {
    let mut errors = Vec::<String>::new();
    let primary = scan_cache_path();
    let legacy = PathBuf::from(SCAN_CACHE_FILE);
    remove_cache_file(&primary, &mut errors);
    if legacy != primary {
        remove_cache_file(&legacy, &mut errors);
    }
    errors
}

fn cache_key(path: &Path) -> String {
    normalize_context_path(path)
}

fn file_signature(path: &Path) -> Option<(u64, u64)> {
    let meta = match fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) => {
            record_runtime_error(format!(
                "scan cache metadata failed for {}: {err}",
                path.display()
            ));
            return None;
        }
    };
    let modified = match meta.modified() {
        Ok(modified) => modified,
        Err(err) => {
            record_runtime_error(format!(
                "scan cache modified-time failed for {}: {err}",
                path.display()
            ));
            return None;
        }
    };
    let secs = match modified.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(err) => {
            record_runtime_error(format!(
                "scan cache unix-time conversion failed for {}: {err}",
                path.display()
            ));
            return None;
        }
    };
    Some((secs, meta.len()))
}

pub fn cache_get(
    cache: &Arc<Mutex<ScanCache>>,
    context: &str,
    tp2: &Path,
) -> Option<Vec<ScannedComponent>> {
    let key = cache_key(tp2);
    let (mtime_secs, size) = file_signature(tp2)?;
    let cache = match cache.lock() {
        Ok(cache) => cache,
        Err(err) => {
            record_runtime_error(format!("scan cache lock failed during get: {err}"));
            return None;
        }
    };
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
    match cache.lock() {
        Ok(mut cache) => {
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
        Err(err) => {
            record_runtime_error(format!("scan cache lock failed during put: {err}"));
        }
    }
}

fn scan_cache_path() -> PathBuf {
    app_config_file(SCAN_CACHE_FILE, ".")
}

fn remove_cache_file(path: &Path, errors: &mut Vec<String>) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => errors.push(format!(
            "remove cache file failed for {}: {err}",
            path.display()
        )),
    }
}

fn try_load_cache(path: &Path, source: &str) -> Result<LoadedScanCache, ScanCacheLoadMetaErr> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) => {
            return Err(Box::new(ScanCacheLoadMeta {
                path: path.display().to_string(),
                source: source.to_string(),
                load_status: if err.kind() == std::io::ErrorKind::NotFound {
                    "missing".to_string()
                } else {
                    "read_error".to_string()
                },
                load_error: (err.kind() != std::io::ErrorKind::NotFound).then(|| err.to_string()),
                file_exists: path.is_file(),
                ..ScanCacheLoadMeta::default()
            }));
        }
    };
    let (meta, cache) = inspect_and_parse_cache_text(path, source, &text)?;
    Ok(LoadedScanCache { cache, meta })
}

fn inspect_and_parse_cache_text(
    path: &Path,
    source: &str,
    text: &str,
) -> Result<(ScanCacheLoadMeta, ScanCache), ScanCacheLoadMetaErr> {
    let raw_result = serde_json::from_str::<RawScanCacheMeta>(text);
    let raw = raw_result.as_ref().ok();
    let file_mtime_secs = fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let file_version = raw.as_ref().and_then(|r| r.version);
    let file_writer_app_version = raw.as_ref().and_then(|r| non_empty(&r.writer_app_version));
    let file_writer_exe_fingerprint = raw
        .as_ref()
        .and_then(|r| non_empty(&r.writer_exe_fingerprint));
    let file_entry_count = raw.as_ref().map(|r| r.entries.len()).unwrap_or(0);
    let mut meta = ScanCacheLoadMeta {
        path: path.display().to_string(),
        source: source.to_string(),
        load_status: "ok".to_string(),
        load_error: None,
        file_exists: true,
        file_mtime_secs,
        file_version,
        file_writer_app_version,
        file_writer_exe_fingerprint,
        file_entry_count,
        version_matches_current_schema: file_version == Some(SCAN_CACHE_VERSION),
        fallback_path: None,
        fallback_source: None,
        fallback_load_status: None,
        fallback_load_error: None,
    };

    if let Err(err) = raw_result {
        meta.load_status = "parse_error".to_string();
        meta.load_error = Some(err.to_string());
        return Err(Box::new(meta));
    }

    match parse_cache_text(text) {
        Ok(cache) => Ok((meta, cache)),
        Err(err) => {
            meta.load_status = if file_version != Some(SCAN_CACHE_VERSION) {
                "version_mismatch".to_string()
            } else {
                "parse_error".to_string()
            };
            meta.load_error = Some(err);
            Err(Box::new(meta))
        }
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

fn runtime_error() -> &'static Mutex<Option<String>> {
    static RUNTIME_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    RUNTIME_ERROR.get_or_init(|| Mutex::new(None))
}

fn record_runtime_error(message: String) {
    if let Ok(mut guard) = runtime_error().lock() {
        *guard = Some(message);
    }
}

fn parse_cache_text(text: &str) -> Result<ScanCache, String> {
    let cache = serde_json::from_str::<ScanCache>(text).map_err(|err| err.to_string())?;
    if cache.version != SCAN_CACHE_VERSION {
        return Err(format!(
            "schema version {} does not match current {}",
            cache.version, SCAN_CACHE_VERSION
        ));
    }
    Ok(cache)
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
