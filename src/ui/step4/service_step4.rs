// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::{fs, io};

use crate::ui::state::Step3ItemState;

pub fn build_weidu_export_lines(items: &[Step3ItemState]) -> Vec<String> {
    items
        .iter()
        .filter(|i| !i.is_parent)
        .map(format_step4_item)
        .collect()
}

pub fn format_step4_item(item: &Step3ItemState) -> String {
    if !item.raw_line.trim().is_empty() {
        normalize_weidu_like_line(&item.raw_line)
    } else {
        let folder = item.mod_name.replace('/', "\\");
        format!(
            "~{}\\{}~ #0 #{} // {}",
            folder, item.tp_file, item.component_id, item.component_label
        )
    }
}

pub fn read_source_log_lines(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}

fn normalize_weidu_like_line(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('~') {
        return trimmed.to_string();
    }
    let Some(end) = trimmed[1..].find('~').map(|i| i + 1) else {
        return trimmed.to_string();
    };
    let path_part = &trimmed[1..end];
    let suffix = &trimmed[end + 1..];
    let p = Path::new(path_part);
    let file = p
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| item_fallback_file(path_part));
    let folder = p
        .parent()
        .and_then(|v| v.file_name())
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| item_fallback_folder(path_part));
    format!("~{}\\{}~{}", folder, file, suffix)
}

fn item_fallback_file(path_part: &str) -> String {
    path_part
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(path_part)
        .to_string()
}

fn item_fallback_folder(path_part: &str) -> String {
    let mut parts = path_part.rsplit(['\\', '/']);
    let _ = parts.next();
    parts.next().unwrap_or("MOD").to_string()
}
