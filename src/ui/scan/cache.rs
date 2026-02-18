// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::{SCAN_CACHE_FILE, SCAN_CACHE_VERSION, ScannedComponent};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanCache {
    pub version: u32,
    pub entries: BTreeMap<String, ScanCacheEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCacheEntry {
    pub context: String,
    pub mtime_secs: u64,
    pub size: u64,
    pub components: Vec<ScannedComponent>,
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

pub fn load_scan_cache() -> ScanCache {
    let path = PathBuf::from(SCAN_CACHE_FILE);
    let Ok(text) = fs::read_to_string(path) else {
        return ScanCache {
            version: SCAN_CACHE_VERSION,
            entries: BTreeMap::new(),
        };
    };
    let Ok(cache) = serde_json::from_str::<ScanCache>(&text) else {
        return ScanCache {
            version: SCAN_CACHE_VERSION,
            entries: BTreeMap::new(),
        };
    };
    if cache.version != SCAN_CACHE_VERSION {
        return ScanCache {
            version: SCAN_CACHE_VERSION,
            entries: BTreeMap::new(),
        };
    }
    cache
}

pub fn save_scan_cache(cache: &ScanCache) {
    if let Ok(text) = serde_json::to_string(cache) {
        let _ = fs::write(SCAN_CACHE_FILE, text);
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
