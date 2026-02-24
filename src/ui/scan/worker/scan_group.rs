// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::install::weidu_scan;
use crate::ui::scan::ScannedComponent;
use crate::ui::scan::cache::{ScanCache, cache_get, cache_put};
use crate::ui::scan::parse::{normalize_tp_file, parse_component_line};

use super::language::candidate_language_ids;

pub(super) fn scan_tp2_group(
    weidu: &Path,
    game_dir: &Path,
    mods_root: &Path,
    tp2_paths: &[std::path::PathBuf],
    cache: &Arc<Mutex<ScanCache>>,
    ctx: &Arc<String>,
    preferred_locale: &str,
) -> Vec<ScannedComponent> {
    let mut entries = Vec::<ScannedComponent>::new();
    for tp2 in tp2_paths {
        if let Some(cached) = cache_get(cache, ctx, tp2) {
            if !cached.is_empty() {
                entries.extend(cached);
                continue;
            }
        }
        let work_dir = preferred_scan_work_dir(tp2, mods_root);
        let expected_tp2 = tp2
            .file_name()
            .and_then(|n| n.to_str())
            .map(normalize_tp_file)
            .unwrap_or_default();
        let mut fallback_components = Vec::<ScannedComponent>::new();
        let language_ids = candidate_language_ids(weidu, tp2, game_dir, work_dir, preferred_locale);

        for lang_id in language_ids {
            let lines = weidu_scan::list_components_lines(weidu, game_dir, work_dir, tp2, &lang_id)
                .unwrap_or_default();
            let parsed_for_tp2 = parse_lines_for_tp2(tp2, &expected_tp2, lines);
            if parsed_for_tp2.is_empty() {
                continue;
            }
            let undefined = parsed_for_tp2
                .iter()
                .filter(|c| c.display.to_ascii_uppercase().contains("UNDEFINED STRING"))
                .count();
            if undefined < parsed_for_tp2.len() {
                cache_put(cache, ctx, tp2, parsed_for_tp2.clone());
                entries.extend(parsed_for_tp2);
                fallback_components.clear();
                break;
            }
            if fallback_components.is_empty() {
                fallback_components = parsed_for_tp2;
            }
        }
        if !fallback_components.is_empty() {
            cache_put(cache, ctx, tp2, fallback_components.clone());
            entries.extend(fallback_components);
        }
    }
    entries
}

fn preferred_scan_work_dir<'a>(tp2: &'a Path, mods_root: &'a Path) -> &'a Path {
    if let Some(parent) = tp2.parent()
        && let Some(parent_of_parent) = parent.parent()
    {
        return parent_of_parent;
    }
    if let Some(parent) = tp2.parent() {
        return parent;
    }
    mods_root
}

fn parse_lines_for_tp2(
    tp2: &Path,
    expected_tp2: &str,
    lines: Vec<String>,
) -> Vec<ScannedComponent> {
    let mut out = Vec::<ScannedComponent>::new();
    for line in lines {
        if let Some(mut parsed) = parse_component_line(&line) {
            if let Some(found_tp2) = parsed.tp_file.as_deref()
                && normalize_tp_file(found_tp2) != expected_tp2
            {
                continue;
            }
            if parsed.tp_file.is_none() {
                parsed.tp_file = tp2.file_name().map(|n| n.to_string_lossy().to_string());
            }
            out.push(parsed);
        }
    }
    out
}
