// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc::Sender};
use std::thread;

use crate::platform_defaults::resolve_weidu_binary;
use crate::ui::scan::cache::{cache_context, load_scan_cache, save_scan_cache};
use crate::ui::scan::discovery::{build_preview_mods, group_tp2s, resolve_scan_game_dir};
use crate::ui::scan::{ENABLE_TWO_PHASE_PREVIEW, ScannedComponent, Step2ScanEvent};
use crate::ui::state::{Step1State, Step2ModState};

use super::build_states::to_mod_states;
use super::language::detect_preferred_game_locale;
use super::scan_group::scan_tp2_group;

pub fn scan_impl(
    step1: &Step1State,
    sender: &Sender<Step2ScanEvent>,
    cancel: &Arc<AtomicBool>,
) -> Result<(Vec<Step2ModState>, Vec<Step2ModState>), String> {
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
    let preferred_locale = Arc::new(detect_preferred_game_locale(step1));
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
    let cache = Arc::new(Mutex::new(load_scan_cache()));
    let ctx = Arc::new(cache_context(&weidu, &game_dir, &mods_root));
    let next_index = Arc::new(AtomicUsize::new(0));
    let progress_count = Arc::new(AtomicUsize::new(0));
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .clamp(2, 8);

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
            let preferred_locale = Arc::clone(&preferred_locale);
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
                    let Some(main_tp2) = tp2_paths.first() else {
                        continue;
                    };

                    let entries =
                        scan_tp2_group(
                            &weidu,
                            &game_dir,
                            &mods_root,
                            tp2_paths,
                            &cache,
                            &ctx,
                            preferred_locale.as_str(),
                        );

                    if let Ok(mut map) = mods_map.lock() {
                        map.entry(label.clone()).or_default().extend(entries);
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
    Ok((bgee_mods, bg2ee_mods))
}
