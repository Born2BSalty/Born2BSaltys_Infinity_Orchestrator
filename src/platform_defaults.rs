// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
        trimmed.to_string()
    }
}

pub fn resolve_mod_installer_binary(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_mod_installer_binary()
    } else {
        trimmed.to_string()
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
