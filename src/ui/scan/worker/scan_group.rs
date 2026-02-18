// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::install::weidu_scan;
use crate::ui::scan::ScannedComponent;
use crate::ui::scan::cache::{ScanCache, cache_get, cache_put};
use crate::ui::scan::parse::{
    is_valid_component_label, normalize_tp_file, parse_component_line,
};

use super::language::pick_language_id;

pub(super) fn scan_tp2_group(
    weidu: &Path,
    game_dir: &Path,
    mods_root: &Path,
    main_tp2: &Path,
    tp2_paths: &[std::path::PathBuf],
    cache: &Arc<Mutex<ScanCache>>,
    ctx: &Arc<String>,
) -> Vec<ScannedComponent> {
    let mut entries = Vec::<ScannedComponent>::new();
    let mut lang_id: Option<String> = None;
    for tp2 in tp2_paths {
        if let Some(cached) = cache_get(cache, ctx, tp2) {
            if !cached.is_empty() {
                entries.extend(cached);
                continue;
            }
        }
        if lang_id.is_none() {
            lang_id = pick_language_id(weidu, main_tp2, game_dir, mods_root);
        }
        let Some(lang_id) = lang_id.as_deref() else {
            continue;
        };
        let expected_tp2 = tp2
            .file_name()
            .and_then(|n| n.to_str())
            .map(normalize_tp_file)
            .unwrap_or_default();
        let mut parsed_for_tp2 = Vec::<ScannedComponent>::new();
        let mut parsed_any_for_tp2 = Vec::<ScannedComponent>::new();
        if let Ok(lines) =
            weidu_scan::list_components_lines(weidu, game_dir, mods_root, tp2, lang_id)
        {
            for line in lines {
                if let Some(mut parsed) = parse_component_line(&line) {
                    parsed_any_for_tp2.push(parsed.clone());
                    if let Some(found_tp2) = parsed.tp_file.as_deref()
                        && normalize_tp_file(found_tp2) != expected_tp2
                    {
                        continue;
                    }
                    if parsed.tp_file.is_none() {
                        parsed.tp_file = tp2.file_name().map(|n| n.to_string_lossy().to_string());
                    }
                    if is_valid_component_label(&parsed.display) {
                        parsed_for_tp2.push(parsed);
                    }
                }
            }
        }
        // Some mods emit component lines with a TP2 token that doesn't match the scanned
        // file name (renamed setup files, wrapper tp2s, etc.). If strict filtering removed
        // everything, fall back to parsed lines for this tp2 to avoid false-empty mods.
        if parsed_for_tp2.is_empty() && !parsed_any_for_tp2.is_empty() {
            parsed_for_tp2 = parsed_any_for_tp2;
        }
        if !parsed_for_tp2.is_empty() {
            cache_put(cache, ctx, tp2, parsed_for_tp2.clone());
        }
        entries.extend(parsed_for_tp2);
    }
    entries
}
