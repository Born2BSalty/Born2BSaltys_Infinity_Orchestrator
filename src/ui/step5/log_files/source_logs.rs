// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::ui::state::Step1State;

#[derive(Debug, Clone)]
pub struct SourceLogInfo {
    pub tag: &'static str,
    pub path: PathBuf,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub modified: Option<SystemTime>,
}

pub fn copy_source_weidu_logs(step1: &Step1State, out_dir: &Path, suffix: &str) -> Vec<PathBuf> {
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

pub fn source_log_infos(step1: &Step1State) -> Vec<SourceLogInfo> {
    resolve_source_logs(step1)
        .into_iter()
        .map(|(tag, path)| {
            let meta = fs::metadata(&path).ok();
            SourceLogInfo {
                tag,
                path,
                exists: meta.as_ref().is_some_and(|m| m.is_file()),
                size_bytes: meta.as_ref().map(|m| m.len()),
                modified: meta.and_then(|m| m.modified().ok()),
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
    if step1.have_weidu_logs && !step1.bgee_log_file.trim().is_empty() {
        PathBuf::from(step1.bgee_log_file.trim())
    } else {
        let folder = if step1.game_install == "EET" {
            step1.eet_bgee_log_folder.trim()
        } else {
            step1.bgee_log_folder.trim()
        };
        PathBuf::from(folder).join("weidu.log")
    }
}

fn resolve_bg2_log_path(step1: &Step1State) -> PathBuf {
    if step1.have_weidu_logs && !step1.bg2ee_log_file.trim().is_empty() {
        PathBuf::from(step1.bg2ee_log_file.trim())
    } else {
        let folder = if step1.game_install == "EET" {
            step1.eet_bg2ee_log_folder.trim()
        } else {
            step1.bg2ee_log_folder.trim()
        };
        PathBuf::from(folder).join("weidu.log")
    }
}
