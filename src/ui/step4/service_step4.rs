// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::{fs, io};

use crate::platform_defaults::normalize_weidu_like_line;
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
