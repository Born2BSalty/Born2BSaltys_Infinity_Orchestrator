// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::platform_defaults::app_config_file;

const PENDING_MOD_CONFIGS_FILE_NAME: &str = "modlist_pending_mod_configs.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
struct PendingModConfigsFile {
    format_version: u64,
    #[serde(default)]
    files: Vec<PendingModConfigFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PendingModConfigFile {
    tp2: String,
    source_id: String,
    relative_path: String,
    base64_data: String,
}

pub(crate) fn pending_mod_configs_path() -> PathBuf {
    app_config_file(PENDING_MOD_CONFIGS_FILE_NAME, "config")
}

pub(crate) fn save_pending_mod_configs(
    files: &[crate::app::modlist_share::ModlistShareConfigFile],
) -> Result<(), String> {
    for file in files {
        validate_relative_config_path(&file.relative_path)?;
    }
    let path = pending_mod_configs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let pending = PendingModConfigsFile {
        format_version: 1,
        files: files
            .iter()
            .map(|file| PendingModConfigFile {
                tp2: file.tp2.clone(),
                source_id: file.source_id.clone(),
                relative_path: file.relative_path.clone(),
                base64_data: file.base64_data.clone(),
            })
            .collect(),
    };
    let content = toml::to_string_pretty(&pending).map_err(|err| err.to_string())?;
    fs::write(path, content).map_err(|err| err.to_string())
}

pub(crate) fn validate_relative_config_path(relative_path: &str) -> Result<PathBuf, String> {
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("mod config path is empty".to_string());
    }
    if normalized.starts_with('/') || normalized.starts_with('\\') {
        return Err(format!("mod config path is root-prefixed: {relative_path}"));
    }
    if has_drive_prefix(&normalized) {
        return Err(format!(
            "mod config path has a drive prefix: {relative_path}"
        ));
    }
    if Path::new(&normalized).is_absolute() {
        return Err(format!("mod config path is absolute: {relative_path}"));
    }
    if normalized.split('/').any(|part| part == "..") {
        return Err(format!(
            "mod config path contains traversal: {relative_path}"
        ));
    }
    Ok(PathBuf::from(normalized))
}

pub(crate) fn restore_pending_mod_configs_for_mod(
    tp2: &str,
    source_id: &str,
    aliases: &[String],
    target_root: &Path,
) -> Result<(), String> {
    let path = pending_mod_configs_path();
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let pending = toml::from_str::<PendingModConfigsFile>(&content).map_err(|err| {
        format!(
            "Read pending mod config metadata failed ({}): {err}",
            path.display()
        )
    })?;
    let accepted_tp2s = accepted_tp2_keys(tp2, aliases);
    let source_id = source_id.trim().to_ascii_lowercase();
    for file in pending.files {
        let pending_tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&file.tp2);
        if !accepted_tp2s.contains(&pending_tp2) {
            continue;
        }
        let pending_source_id = file.source_id.trim().to_ascii_lowercase();
        if !pending_source_id.is_empty() && pending_source_id != source_id {
            continue;
        }
        let relative_path = validate_relative_config_path(&file.relative_path)?;
        let destination = safe_config_destination(target_root, &relative_path)?;
        let bytes = base64url_decode(&file.base64_data)
            .map_err(|err| format!("Decode pending mod config failed: {err}"))?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        fs::write(&destination, bytes).map_err(|err| {
            format!(
                "Write pending mod config failed ({}): {err}",
                destination.display()
            )
        })?;
    }
    Ok(())
}

fn accepted_tp2_keys(tp2: &str, aliases: &[String]) -> Vec<String> {
    let mut accepted = vec![crate::app::mod_downloads::normalize_mod_download_tp2(tp2)];
    accepted.extend(
        aliases
            .iter()
            .map(|alias| crate::app::mod_downloads::normalize_mod_download_tp2(alias)),
    );
    accepted.sort();
    accepted.dedup();
    accepted
}

fn safe_config_destination(target_root: &Path, relative_path: &Path) -> Result<PathBuf, String> {
    let canonical_root = target_root.canonicalize().map_err(|err| {
        format!(
            "Canonicalize mod config root failed ({}): {err}",
            target_root.display()
        )
    })?;
    let destination = target_root.join(relative_path);
    let parent = destination
        .parent()
        .ok_or_else(|| "mod config destination parent is missing".to_string())?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    let canonical_parent = parent.canonicalize().map_err(|err| {
        format!(
            "Canonicalize mod config destination failed ({}): {err}",
            parent.display()
        )
    })?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "mod config destination escapes mod root: {}",
            destination.display()
        ));
    }
    Ok(destination)
}

fn has_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) && chars.next() == Some(':')
}

fn base64url_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut values = Vec::new();
    for ch in text.chars().filter(|ch| !ch.is_whitespace()) {
        match ch {
            'A'..='Z' => values.push(ch as u8 - b'A'),
            'a'..='z' => values.push(ch as u8 - b'a' + 26),
            '0'..='9' => values.push(ch as u8 - b'0' + 52),
            '-' => values.push(62),
            '_' => values.push(63),
            _ => {
                return Err("pending mod config contains invalid base64url characters.".to_string());
            }
        }
    }
    let remainder = values.len() % 4;
    if remainder == 1 {
        return Err("pending mod config base64url length is invalid.".to_string());
    }
    if remainder != 0 {
        values.extend(std::iter::repeat_n(64, 4 - remainder));
    }
    let mut out = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        // Lint-only bytecount dependency is not approved for this local base64 padding check.
        #[allow(clippy::naive_bytecount)]
        let pad = chunk.iter().filter(|value| **value == 64).count();
        if pad > 2 || chunk[..4 - pad].contains(&64) {
            return Err("pending mod config base64 padding is invalid.".to_string());
        }
        let c0 = chunk[0];
        let c1 = chunk[1];
        let c2 = if chunk[2] == 64 { 0 } else { chunk[2] };
        let c3 = if chunk[3] == 64 { 0 } else { chunk[3] };
        out.push((c0 << 2) | (c1 >> 4));
        if pad < 2 {
            out.push(((c1 & 0b0000_1111) << 4) | (c2 >> 2));
        }
        if pad == 0 {
            out.push(((c2 & 0b0000_0011) << 6) | c3);
        }
    }
    Ok(out)
}
