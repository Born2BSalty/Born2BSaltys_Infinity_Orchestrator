// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::controller::util::open_in_shell;
use crate::ui::state::Step1State;

pub fn open_last_log_file(step1: &Step1State) -> std::io::Result<()> {
    let Some(path) = newest_log_file(step1) else {
        return Ok(());
    };
    open_in_shell(path.to_string_lossy().as_ref())
}

pub fn save_console_log(content: &str) -> std::io::Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("console_{ts}.log"));
    fs::write(&out_path, content)?;
    Ok(out_path)
}

pub fn open_console_logs_folder() -> std::io::Result<()> {
    let dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&dir)?;
    open_in_shell(dir.to_string_lossy().as_ref())
}

fn newest_log_file(step1: &Step1State) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in log_candidate_dirs(step1) {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("log"))
                != Some(true)
            {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            match &best {
                Some((best_time, _)) if modified <= *best_time => {}
                _ => best = Some((modified, path)),
            }
        }
    }
    best.map(|(_, p)| p)
}

fn log_candidate_dirs(step1: &Step1State) -> Vec<PathBuf> {
    let mut dirs = Vec::<PathBuf>::new();
    let mut add = |v: &str| {
        let t = v.trim();
        if !t.is_empty() {
            dirs.push(PathBuf::from(t));
        }
    };
    add(&step1.weidu_log_folder);
    add(&step1.bgee_log_folder);
    add(&step1.bg2ee_log_folder);
    add(&step1.eet_bgee_log_folder);
    add(&step1.eet_bg2ee_log_folder);
    if !step1.bgee_log_file.trim().is_empty()
        && let Some(p) = Path::new(step1.bgee_log_file.trim()).parent()
    {
        dirs.push(p.to_path_buf());
    }
    if !step1.bg2ee_log_file.trim().is_empty()
        && let Some(p) = Path::new(step1.bg2ee_log_file.trim()).parent()
    {
        dirs.push(p.to_path_buf());
    }
    dirs
}
