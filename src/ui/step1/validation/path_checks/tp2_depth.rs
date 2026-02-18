// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::Path;

pub(super) fn has_tp2_within_depth(root: &Path, depth: usize) -> bool {
    fn walk(dir: &Path, remaining: usize) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let is_tp2 = path
                    .extension()
                    .and_then(|v| v.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("tp2"))
                    .unwrap_or(false);
                if is_tp2 {
                    return true;
                }
            } else if path.is_dir() && remaining > 0 && walk(&path, remaining - 1) {
                return true;
            }
        }
        false
    }
    walk(root, depth)
}
