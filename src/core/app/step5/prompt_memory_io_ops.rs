// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

use super::storage;

pub(super) fn export_json(path: &Path) -> std::io::Result<usize> {
    let guard = storage::memory()
        .lock()
        .map_err(|_| std::io::Error::other("prompt memory lock poisoned"))?;
    let raw = storage::serialize_map(&guard)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, raw)?;
    Ok(guard.len())
}

pub(super) fn import_json(path: &Path) -> std::io::Result<usize> {
    let content = fs::read_to_string(path)?;
    let imported = storage::parse_content(&content)
        .ok_or_else(|| std::io::Error::other("invalid prompt answers json"))?;
    let mut guard = storage::memory()
        .lock()
        .map_err(|_| std::io::Error::other("prompt memory lock poisoned"))?;
    *guard = imported;
    storage::save_to_disk(&guard)?;
    Ok(guard.len())
}
