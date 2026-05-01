// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
const DEFAULT_WEIDU_BINARY: &str = "weidu.exe";
#[cfg(not(target_os = "windows"))]
const DEFAULT_WEIDU_BINARY: &str = "weidu";

#[cfg(target_os = "windows")]
const DEFAULT_MOD_INSTALLER_BINARY: &str = "mod_installer.exe";
#[cfg(not(target_os = "windows"))]
const DEFAULT_MOD_INSTALLER_BINARY: &str = "mod_installer";

pub fn default_weidu_binary() -> String {
    DEFAULT_WEIDU_BINARY.to_string()
}

pub fn default_mod_installer_binary() -> String {
    DEFAULT_MOD_INSTALLER_BINARY.to_string()
}

pub fn resolve_weidu_binary(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_weidu_binary()
    } else {
        normalize_binary_for_platform(trimmed)
    }
}

pub fn resolve_mod_installer_binary(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_mod_installer_binary()
    } else {
        normalize_binary_for_platform(trimmed)
    }
}

fn normalize_binary_for_platform(value: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        value.to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if !value.contains('/')
            && !value.contains('\\')
            && let Some(stripped) = value.strip_suffix(".exe")
            && !stripped.is_empty()
        {
            return stripped.to_string();
        }
        value.to_string()
    }
}

pub fn compose_weidu_log_path(folder: &str) -> String {
    let trimmed = folder.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        PathBuf::from(trimmed)
            .join("weidu.log")
            .to_string_lossy()
            .to_string()
    }
}

pub fn app_config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA")
            && !appdata.trim().is_empty()
        {
            return Some(PathBuf::from(appdata).join("bio"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME")
            && !home.trim().is_empty()
        {
            return Some(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("bio"),
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME")
            && !home.trim().is_empty()
        {
            return Some(PathBuf::from(home).join(".config").join("bio"));
        }
    }
    None
}

pub fn app_config_file(file_name: &str, fallback_dir: &str) -> PathBuf {
    if let Some(dir) = app_config_dir() {
        return dir.join(file_name);
    }
    PathBuf::from(fallback_dir).join(file_name)
}

pub fn normalize_tp2_filename(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let filename = replaced
        .rsplit('/')
        .next()
        .unwrap_or(replaced.as_str())
        .trim();
    filename.to_ascii_uppercase()
}

pub fn compose_component_key(tp_file: &str, component: &str) -> String {
    format!("{}#{}", normalize_tp2_filename(tp_file), component.trim())
}

pub fn normalize_weidu_like_line(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('~') {
        return trimmed.to_string();
    }
    let Some(end) = trimmed[1..].find('~').map(|i| i + 1) else {
        return trimmed.to_string();
    };
    let path_part = &trimmed[1..end];
    let suffix = &trimmed[end + 1..];
    let path = Path::new(path_part);
    let file = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| weidu_path_fallback_file(path_part));
    let folder = path
        .parent()
        .and_then(|v| v.file_name())
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| weidu_path_fallback_folder(path_part));
    format!("~{}\\{}~{}", folder, file, suffix)
}

fn weidu_path_fallback_file(path_part: &str) -> String {
    path_part
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path_part)
        .to_string()
}

fn weidu_path_fallback_folder(path_part: &str) -> String {
    let mut parts = path_part.rsplit(['\\', '/']);
    let _ = parts.next();
    parts.next().unwrap_or("MOD").to_string()
}
