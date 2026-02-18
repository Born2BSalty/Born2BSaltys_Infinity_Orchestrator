// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub fn find_best_readme(mods_root: &Path, tp2_path: &str, mod_name: &str) -> Option<String> {
    let tp2 = Path::new(tp2_path);
    let rel = tp2.strip_prefix(mods_root).ok()?;
    let first = rel.components().next()?;
    let mod_dir = mods_root.join(first.as_os_str());
    if !mod_dir.is_dir() {
        return None;
    }
    let tp2_stem = tp2
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mod_name_l = mod_name.to_ascii_lowercase();

    let mut best: Option<(i32, PathBuf)> = None;
    for entry in WalkDir::new(&mod_dir)
        .follow_links(false)
        .max_depth(5)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let Some(name_os) = path.file_name() else { continue };
        let filename = name_os.to_string_lossy().to_ascii_lowercase();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "html" | "htm" | "md" | "txt" | "pdf") {
            continue;
        }
        if !(filename.contains("readme")
            || filename.contains("read me")
            || filename.contains("read_me")
            || filename.contains("manual")
            || filename.contains("guide")
            || filename.contains("faq")
            || filename.contains("changelog"))
        {
            continue;
        }
        let mut score = 0i32;
        if filename.starts_with("readme") {
            score += 40;
        }
        if filename.contains("english")
            || filename.contains("en_us")
            || filename.contains("_en")
            || filename.contains("-en")
            || filename.contains("(en)")
        {
            score += 25;
        }
        if filename.contains(&tp2_stem) || filename.contains(&mod_name_l) {
            score += 18;
        }
        score += match ext.as_str() {
            "html" | "htm" => 14,
            "md" => 10,
            "txt" => 8,
            "pdf" => 6,
            _ => 0,
        };
        if let Ok(rel) = path.strip_prefix(&mod_dir) {
            let depth = rel.components().count() as i32;
            score -= depth * 2;
            let rel_l = rel.to_string_lossy().to_ascii_lowercase();
            if rel_l.contains("backup") || rel_l.contains("old") || rel_l.contains("temp") {
                score -= 20;
            }
            if rel_l.contains("lang\\en") || rel_l.contains("english") {
                score += 8;
            }
        }
        if let Ok(meta) = fs::metadata(path) {
            let sz = meta.len();
            if sz < 128 {
                score -= 8;
            }
        }
        match &best {
            Some((best_score, _)) if score <= *best_score => {}
            _ => best = Some((score, path.to_path_buf())),
        }
    }
    best.map(|(_, p)| p.to_string_lossy().to_string())
}
