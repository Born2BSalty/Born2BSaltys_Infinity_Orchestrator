// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "worker_build_states.rs"]
mod build_states;

#[path = "worker_language.rs"]
mod language;

mod orchestrate {
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;

use crate::platform_defaults::resolve_weidu_binary;
use crate::ui::controller::util::current_exe_fingerprint;
use crate::ui::scan::cache::{cache_context, load_scan_cache, save_scan_cache};
use crate::ui::scan::discovery::{build_preview_mods, group_tp2s, resolve_scan_game_dir};
use crate::ui::scan::{ENABLE_TWO_PHASE_PREVIEW, ScannedComponent, Step2ScanEvent};
use crate::ui::state::{Step1State, Step2ModState, Step2ScanReport, Step2Tp2ProbeReport};

use super::build_states::to_mod_states;
use super::language::detect_preferred_game_locale;
use super::scan_group::{scan_tp2_group, ScanGroupContext};

fn select_main_tp2<'a>(group_label: &str, tp2_paths: &'a [PathBuf], mods_root: &Path) -> Option<&'a PathBuf> {
    let first = tp2_paths.first()?;
    let group_root = mods_root.join(group_label);
    let group_name = Path::new(group_label)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(group_label)
        .to_ascii_lowercase();
    let preferred_setup = format!("setup-{group_name}.tp2");
    let preferred_plain = format!("{group_name}.tp2");
    let mut root_level = None::<&PathBuf>;
    let mut root_plain = None::<&PathBuf>;

    for tp2 in tp2_paths {
        if tp2.parent() != Some(group_root.as_path()) {
            continue;
        }
        let file_name = tp2
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if file_name == preferred_setup {
            return Some(tp2);
        }
        if file_name == preferred_plain {
            root_plain = Some(tp2);
        }
        if root_level.is_none() {
            root_level = Some(tp2);
        }
    }

    root_plain.or(root_level).or(Some(first))
}

pub fn scan_impl(
    step1: &Step1State,
    sender: &Sender<Step2ScanEvent>,
    cancel: &Arc<AtomicBool>,
) -> Result<(Vec<Step2ModState>, Vec<Step2ModState>, Step2ScanReport), String> {
    let mods_root = PathBuf::from(step1.mods_folder.trim());
    let weidu = PathBuf::from(resolve_weidu_binary(&step1.weidu_binary));
    if mods_root.as_os_str().is_empty() || weidu.as_os_str().is_empty() {
        return Err("missing Mods Folder or WeiDU binary".to_string());
    }

    let Some(game_dir) = resolve_scan_game_dir(step1) else {
        return Err("missing game directory for scan".to_string());
    };
    if !game_dir.exists() {
        return Err(format!("scan game directory does not exist: {}", game_dir.display()));
    }

    let scan_depth = if step1.custom_scan_depth {
        step1.depth.max(1)
    } else {
        5
    };
    let grouped = group_tp2s(&mods_root, scan_depth);
    let preferred_locale_info = Arc::new(detect_preferred_game_locale(step1));
    if ENABLE_TWO_PHASE_PREVIEW {
        let preview = build_preview_mods(&grouped);
        let (bgee_mods, bg2ee_mods) = match step1.game_install.as_str() {
            "BG2EE" => (Vec::new(), preview),
            "EET" => (preview.clone(), preview),
            _ => (preview, Vec::new()),
        };
        let _ = sender.send(Step2ScanEvent::Preview {
            bgee_mods,
            bg2ee_mods,
            total: grouped.len(),
        });
    }

    let total = grouped.len();
    let grouped = Arc::new(grouped);
    let mods_map = Arc::new(Mutex::new(BTreeMap::<String, Vec<ScannedComponent>>::new()));
    let tp2_map = Arc::new(Mutex::new(BTreeMap::<String, String>::new()));
    let loaded_cache = load_scan_cache();
    let cache_meta = loaded_cache.meta.clone();
    let cache = Arc::new(Mutex::new(loaded_cache.cache));
    let ctx = Arc::new(cache_context(&weidu, &game_dir, &mods_root));
    let next_index = Arc::new(AtomicUsize::new(0));
    let progress_count = Arc::new(AtomicUsize::new(0));
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 16);
    let scan_reports = Arc::new(Mutex::new(Vec::<Step2Tp2ProbeReport>::new()));

    thread::scope(|scope| {
        for _ in 0..worker_count {
            let grouped = Arc::clone(&grouped);
            let mods_map = Arc::clone(&mods_map);
            let tp2_map = Arc::clone(&tp2_map);
            let next_index = Arc::clone(&next_index);
            let progress_count = Arc::clone(&progress_count);
            let cancel = Arc::clone(cancel);
            let sender = sender.clone();
            let weidu = weidu.clone();
            let game_dir = game_dir.clone();
            let mods_root = mods_root.clone();
            let cache = Arc::clone(&cache);
            let ctx = Arc::clone(&ctx);
            let preferred_locale = Arc::clone(&preferred_locale_info);
            let scan_reports = Arc::clone(&scan_reports);
            scope.spawn(move || {
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let idx = next_index.fetch_add(1, Ordering::Relaxed);
                    if idx >= grouped.len() {
                        break;
                    }
                    let (label, tp2_paths) = &grouped[idx];
                    let Some(main_tp2) = select_main_tp2(label, tp2_paths, &mods_root) else {
                        continue;
                    };

                    let scan_group_ctx = ScanGroupContext {
                        group_label: label,
                        weidu: &weidu,
                        game_dir: &game_dir,
                        mods_root: &mods_root,
                        cache: &cache,
                        ctx: &ctx,
                        preferred_locale: preferred_locale.locale.as_str(),
                        game_install: step1.game_install.as_str(),
                    };
                    let (entries, reports) = scan_tp2_group(&scan_group_ctx, tp2_paths);

                    if let Ok(mut map) = mods_map.lock() {
                        map.entry(label.clone()).or_default().extend(entries);
                    }
                    if let Ok(mut all_reports) = scan_reports.lock() {
                        all_reports.extend(reports);
                    }
                    if let Ok(mut map) = tp2_map.lock() {
                        map.entry(label.clone())
                            .or_insert_with(|| main_tp2.display().to_string());
                    }

                    let done = progress_count.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = sender.send(Step2ScanEvent::Progress {
                        current: done,
                        total,
                        name: label.clone(),
                    });
                }
            });
        }
    });

    if cancel.load(Ordering::Relaxed) {
        return Err("canceled".to_string());
    }

    let mods_map = mods_map
        .lock()
        .map_err(|_| "scan map lock poisoned".to_string())?
        .clone();
    let tp2_map = tp2_map
        .lock()
        .map_err(|_| "scan tp2 map lock poisoned".to_string())?
        .clone();
    if let Ok(cache) = cache.lock() {
        save_scan_cache(&cache);
    }

    let scanned_mods = to_mod_states(mods_map, tp2_map, &mods_root);
    let (bgee_mods, bg2ee_mods) = match step1.game_install.as_str() {
        "BG2EE" => (Vec::new(), scanned_mods),
        "EET" => (scanned_mods.clone(), scanned_mods),
        _ => (scanned_mods, Vec::new()),
    };

    let mut tp2_reports = scan_reports
        .lock()
        .map_err(|_| "scan reports lock poisoned".to_string())?
        .clone();
    tp2_reports.sort_by(|a, b| a.tp2_path.cmp(&b.tp2_path));
    let tp2_cache_hits = tp2_reports.iter().filter(|r| r.used_cache).count();
    let total_tp2 = tp2_reports.len();
    let current_exe_fingerprint = current_exe_fingerprint();
    let report = Step2ScanReport {
        game_dir: game_dir.display().to_string(),
        mods_root: mods_root.display().to_string(),
        scan_depth,
        preferred_locale: preferred_locale_info.locale.clone(),
        preferred_locale_source: preferred_locale_info.source.clone(),
        preferred_locale_baldur_lua: preferred_locale_info
            .baldur_lua_path
            .as_ref()
            .map(|p| p.display().to_string()),
        worker_count,
        total_groups: total,
        total_tp2,
        tp2_cache_hits,
        tp2_cache_misses: total_tp2.saturating_sub(tp2_cache_hits),
        scan_cache_path: cache_meta.path,
        scan_cache_source: cache_meta.source,
        scan_cache_file_exists: cache_meta.file_exists,
        scan_cache_file_mtime_secs: cache_meta.file_mtime_secs,
        scan_cache_file_version: cache_meta.file_version,
        scan_cache_writer_app_version: cache_meta.file_writer_app_version.clone(),
        scan_cache_writer_exe_fingerprint: cache_meta.file_writer_exe_fingerprint.clone(),
        scan_cache_entry_count: cache_meta.file_entry_count,
        scan_cache_version_matches_current_schema: cache_meta.version_matches_current_schema,
        scan_cache_writer_matches_current_app_version: if tp2_cache_hits == 0 {
            None
        } else {
            Some(
                cache_meta
                    .file_writer_app_version
                    .as_deref()
                    .is_some_and(|v| v == env!("CARGO_PKG_VERSION")),
            )
        },
        scan_cache_writer_matches_current_exe: if tp2_cache_hits == 0 {
            None
        } else {
            Some(
                cache_meta
                    .file_writer_exe_fingerprint
                    .as_deref()
                    .is_some_and(|v| v == current_exe_fingerprint),
            )
        },
        tp2_reports,
    };
    Ok((bgee_mods, bg2ee_mods, report))
}
}

#[path = "worker_scan_group.rs"]
mod scan_group;

pub use orchestrate::scan_impl;
