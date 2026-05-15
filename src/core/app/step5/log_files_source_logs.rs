// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::app::state::Step1State;

#[derive(Debug, Clone)]
pub(crate) struct SourceLogInfo {
    pub(crate) tag: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) exists: bool,
    pub(crate) size_bytes: Option<u64>,
    pub(crate) modified: Option<SystemTime>,
}

pub(super) fn copy_source_weidu_logs(
    step1: &Step1State,
    out_dir: &Path,
    suffix: &str,
) -> Vec<PathBuf> {
    let mut copied = Vec::new();
    let _ = fs::create_dir_all(out_dir);
    for (tag, source) in resolve_source_logs(step1) {
        if !source.is_file() {
            continue;
        }
        let dest = out_dir.join(format!("weidu_{tag}_{suffix}.log"));
        if fs::copy(&source, &dest).is_ok() {
            copied.push(dest);
        }
    }
    copied
}

pub(super) fn copy_saved_weidu_logs(
    step1: &Step1State,
    out_dir: &Path,
    suffix: &str,
) -> Vec<PathBuf> {
    let mut copied = Vec::new();
    let _ = fs::create_dir_all(out_dir);
    for (tag, source) in resolve_saved_logs(step1) {
        if !source.is_file() {
            continue;
        }
        let dest = out_dir.join(format!("weidu_{tag}_{suffix}.log"));
        if fs::copy(&source, &dest).is_ok() {
            copied.push(dest);
        }
    }
    copied
}

pub(crate) fn source_log_infos(step1: &Step1State) -> Vec<SourceLogInfo> {
    resolve_source_logs(step1)
        .into_iter()
        .map(|(tag, path)| {
            let meta = fs::metadata(&path).ok();
            SourceLogInfo {
                tag,
                path,
                exists: meta.as_ref().is_some_and(std::fs::Metadata::is_file),
                size_bytes: meta.as_ref().map(std::fs::Metadata::len),
                modified: meta.and_then(|value| value.modified().ok()),
            }
        })
        .collect()
}

fn resolve_source_logs(step1: &Step1State) -> Vec<(&'static str, PathBuf)> {
    match step1.game_install.as_str() {
        "EET" => vec![
            ("bgee", resolve_bgee_log_path(step1)),
            ("bg2ee", resolve_bg2_log_path(step1)),
        ],
        "BG2EE" => vec![("bg2ee", resolve_bg2_log_path(step1))],
        _ => vec![("bgee", resolve_bgee_log_path(step1))],
    }
}

fn resolve_bgee_log_path(step1: &Step1State) -> PathBuf {
    if step1.bgee_log_file.trim().is_empty() {
        resolve_saved_bgee_log_path(step1)
    } else {
        PathBuf::from(step1.bgee_log_file.trim())
    }
}

fn resolve_bg2_log_path(step1: &Step1State) -> PathBuf {
    if step1.bg2ee_log_file.trim().is_empty() {
        resolve_saved_bg2_log_path(step1)
    } else {
        PathBuf::from(step1.bg2ee_log_file.trim())
    }
}

fn resolve_saved_logs(step1: &Step1State) -> Vec<(&'static str, PathBuf)> {
    match step1.game_install.as_str() {
        "EET" => vec![
            ("bgee", resolve_saved_bgee_log_path(step1)),
            ("bg2ee", resolve_saved_bg2_log_path(step1)),
        ],
        "BG2EE" => vec![("bg2ee", resolve_saved_bg2_log_path(step1))],
        _ => vec![("bgee", resolve_saved_bgee_log_path(step1))],
    }
}

fn resolve_saved_bgee_log_path(step1: &Step1State) -> PathBuf {
    let folder = if step1.game_install == "EET" {
        step1.eet_bgee_log_folder.trim()
    } else {
        step1.bgee_log_folder.trim()
    };
    PathBuf::from(folder).join("weidu.log")
}

fn resolve_saved_bg2_log_path(step1: &Step1State) -> PathBuf {
    let folder = if step1.game_install == "EET" {
        step1.eet_bg2ee_log_folder.trim()
    } else {
        step1.bg2ee_log_folder.trim()
    };
    PathBuf::from(folder).join("weidu.log")
}
