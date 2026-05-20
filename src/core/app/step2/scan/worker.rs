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

    use crate::app::controller::util::current_exe_fingerprint;
    use crate::app::scan::cache::{cache_context, load_scan_cache, save_scan_cache};
    use crate::app::scan::discovery::{build_preview_mods, group_tp2s, resolve_scan_game_dir};
    use crate::app::scan::{ENABLE_TWO_PHASE_PREVIEW, ScannedComponent, Step2ScanEvent};
    use crate::app::state::{Step1State, Step2ModState, Step2ScanReport, Step2Tp2ProbeReport};
    use crate::platform_defaults::resolve_weidu_binary;

    use super::build_states::to_mod_states;
    use super::language::detect_preferred_game_locale;
    use super::scan_group::{ScanGroupContext, scan_tp2_group};

    const SCAN_WORKER_STACK_SIZE: usize = 32 * 1024 * 1024;

    type GroupedTp2s = Vec<(String, Vec<PathBuf>)>;
    type ScannedModMap = BTreeMap<String, Vec<ScannedComponent>>;
    type Tp2PathMap = BTreeMap<String, String>;

    struct ScanInputs {
        mods_root: PathBuf,
        weidu: PathBuf,
        game_dir: PathBuf,
        scan_depth: usize,
    }

    struct WorkerShared {
        grouped: Arc<GroupedTp2s>,
        mods_map: Arc<Mutex<ScannedModMap>>,
        tp2_map: Arc<Mutex<Tp2PathMap>>,
        cache: Arc<Mutex<crate::app::scan::cache::ScanCache>>,
        ctx: Arc<String>,
        next_index: Arc<AtomicUsize>,
        progress_count: Arc<AtomicUsize>,
        scan_error: Arc<Mutex<Option<String>>>,
        scan_reports: Arc<Mutex<Vec<Step2Tp2ProbeReport>>>,
    }

    struct ScanRunResults {
        mods_map: ScannedModMap,
        tp2_map: Tp2PathMap,
        scan_cache_save_error: Option<String>,
        tp2_reports: Vec<Step2Tp2ProbeReport>,
    }

    struct WorkerLoopContext<'a> {
        step1: &'a Step1State,
        grouped: Arc<GroupedTp2s>,
        mods_map: Arc<Mutex<ScannedModMap>>,
        tp2_map: Arc<Mutex<Tp2PathMap>>,
        next_index: Arc<AtomicUsize>,
        progress_count: Arc<AtomicUsize>,
        cancel: Arc<AtomicBool>,
        sender: Sender<Step2ScanEvent>,
        weidu: PathBuf,
        game_dir: PathBuf,
        mods_root: PathBuf,
        cache: Arc<Mutex<crate::app::scan::cache::ScanCache>>,
        ctx: Arc<String>,
        preferred_locale: Arc<super::language::PreferredLocaleInfo>,
        scan_error: Arc<Mutex<Option<String>>>,
        scan_reports: Arc<Mutex<Vec<Step2Tp2ProbeReport>>>,
        total: usize,
    }

    struct SpawnScanWorkers<'a> {
        step1: &'a Step1State,
        inputs: &'a ScanInputs,
        preferred_locale_info: &'a Arc<super::language::PreferredLocaleInfo>,
        sender: &'a Sender<Step2ScanEvent>,
        cancel: &'a Arc<AtomicBool>,
        worker_count: usize,
        total: usize,
        shared: &'a WorkerShared,
    }

    fn select_main_tp2<'a>(
        group_label: &str,
        tp2_paths: &'a [PathBuf],
        mods_root: &Path,
    ) -> Option<&'a PathBuf> {
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
                .map(str::to_ascii_lowercase)
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
        let inputs = scan_inputs(step1)?;
        let grouped = group_tp2s(&inputs.mods_root, inputs.scan_depth)?;
        let preferred_locale_info = Arc::new(detect_preferred_game_locale(step1));
        if ENABLE_TWO_PHASE_PREVIEW {
            send_preview(step1.game_install.as_str(), sender, &grouped);
        }

        let total = grouped.len();
        let loaded_cache = load_scan_cache();
        let cache_meta = loaded_cache.meta.clone();
        let worker_count = std::thread::available_parallelism()
            .map_or(4, std::num::NonZero::get)
            .clamp(2, 16);
        let shared = WorkerShared {
            grouped: Arc::new(grouped),
            mods_map: Arc::new(Mutex::new(BTreeMap::new())),
            tp2_map: Arc::new(Mutex::new(BTreeMap::new())),
            cache: Arc::new(Mutex::new(loaded_cache.cache)),
            ctx: Arc::new(cache_context(
                &inputs.weidu,
                &inputs.game_dir,
                &inputs.mods_root,
            )),
            next_index: Arc::new(AtomicUsize::new(0)),
            progress_count: Arc::new(AtomicUsize::new(0)),
            scan_error: Arc::new(Mutex::new(None)),
            scan_reports: Arc::new(Mutex::new(Vec::new())),
        };

        spawn_scan_workers(&SpawnScanWorkers {
            step1,
            inputs: &inputs,
            preferred_locale_info: &preferred_locale_info,
            sender,
            cancel,
            shared: &shared,
            worker_count,
            total,
        });

        let results = scan_run_results(cancel, &shared)?;
        let ScanRunResults {
            mods_map,
            tp2_map,
            scan_cache_save_error,
            tp2_reports,
        } = results;
        let scanned_mods = to_mod_states(mods_map, &tp2_map, &inputs.mods_root);
        let (first_game_mods, second_game_mods) =
            split_mods_by_game(step1.game_install.as_str(), scanned_mods);
        let report = build_scan_report(
            &inputs,
            &preferred_locale_info,
            worker_count,
            total,
            cache_meta,
            scan_cache_save_error,
            tp2_reports,
        );
        Ok((first_game_mods, second_game_mods, report))
    }

    fn scan_inputs(step1: &Step1State) -> Result<ScanInputs, String> {
        let mods_root = PathBuf::from(step1.mods_folder.trim());
        let weidu = PathBuf::from(resolve_weidu_binary(&step1.weidu_binary));
        if mods_root.as_os_str().is_empty() || weidu.as_os_str().is_empty() {
            return Err("missing Mods Folder or WeiDU binary".to_string());
        }

        let Some(game_dir) = resolve_scan_game_dir(step1) else {
            return Err("missing game directory for scan".to_string());
        };
        if !game_dir.exists() {
            return Err(format!(
                "scan game directory does not exist: {}",
                game_dir.display()
            ));
        }

        let scan_depth = if step1.custom_scan_depth {
            step1.depth.max(1)
        } else {
            5
        };
        Ok(ScanInputs {
            mods_root,
            weidu,
            game_dir,
            scan_depth,
        })
    }

    fn send_preview(game_install: &str, sender: &Sender<Step2ScanEvent>, grouped: &GroupedTp2s) {
        let preview = build_preview_mods(grouped);
        let (first_game_mods, second_game_mods) = split_mods_by_game(game_install, preview);
        let _ = sender.send(Step2ScanEvent::Preview {
            bgee_mods: first_game_mods,
            bg2ee_mods: second_game_mods,
            total: grouped.len(),
        });
    }

    fn spawn_scan_workers(args: &SpawnScanWorkers<'_>) {
        thread::scope(|scope| {
            for _ in 0..args.worker_count {
                let worker_ctx = WorkerLoopContext {
                    step1: args.step1,
                    grouped: Arc::clone(&args.shared.grouped),
                    mods_map: Arc::clone(&args.shared.mods_map),
                    tp2_map: Arc::clone(&args.shared.tp2_map),
                    next_index: Arc::clone(&args.shared.next_index),
                    progress_count: Arc::clone(&args.shared.progress_count),
                    cancel: Arc::clone(args.cancel),
                    sender: args.sender.clone(),
                    weidu: args.inputs.weidu.clone(),
                    game_dir: args.inputs.game_dir.clone(),
                    mods_root: args.inputs.mods_root.clone(),
                    cache: Arc::clone(&args.shared.cache),
                    ctx: Arc::clone(&args.shared.ctx),
                    preferred_locale: Arc::clone(args.preferred_locale_info),
                    scan_error: Arc::clone(&args.shared.scan_error),
                    scan_reports: Arc::clone(&args.shared.scan_reports),
                    total: args.total,
                };
                if let Err(err) = thread::Builder::new()
                    .name("bio-scan-worker".to_string())
                    .stack_size(SCAN_WORKER_STACK_SIZE)
                    .spawn_scoped(scope, move || scan_worker_loop(&worker_ctx))
                {
                    record_scan_error(
                        &args.shared.scan_error,
                        format!("failed to spawn scan worker: {err}"),
                    );
                    break;
                }
            }
        });
    }

    fn scan_worker_loop(ctx: &WorkerLoopContext<'_>) {
        loop {
            if ctx.cancel.load(Ordering::Relaxed) || scan_error_is_set(&ctx.scan_error) {
                break;
            }
            let idx = ctx.next_index.fetch_add(1, Ordering::Relaxed);
            if idx >= ctx.grouped.len() {
                break;
            }
            let (label, tp2_paths) = &ctx.grouped[idx];
            let Some(main_tp2) = select_main_tp2(label, tp2_paths, &ctx.mods_root) else {
                continue;
            };

            let scan_group_ctx = ScanGroupContext {
                group_label: label,
                weidu: &ctx.weidu,
                game_dir: &ctx.game_dir,
                mods_root: &ctx.mods_root,
                cache: &ctx.cache,
                ctx: &ctx.ctx,
                preferred_locale: ctx.preferred_locale.locale.as_str(),
                game_install: ctx.step1.game_install.as_str(),
            };
            let (entries, reports) = match scan_tp2_group(&scan_group_ctx, tp2_paths) {
                Ok(result) => result,
                Err(err) => {
                    record_scan_error(&ctx.scan_error, err);
                    break;
                }
            };

            if let Ok(mut map) = ctx.mods_map.lock() {
                map.entry(label.clone()).or_default().extend(entries);
            }
            if let Ok(mut all_reports) = ctx.scan_reports.lock() {
                all_reports.extend(reports);
            }
            if let Ok(mut map) = ctx.tp2_map.lock() {
                map.entry(label.clone())
                    .or_insert_with(|| main_tp2.display().to_string());
            }

            let done = ctx.progress_count.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = ctx.sender.send(Step2ScanEvent::Progress {
                current: done,
                total: ctx.total,
                name: label.clone(),
            });
        }
    }

    fn scan_error_is_set(scan_error: &Arc<Mutex<Option<String>>>) -> bool {
        scan_error.lock().map_or(true, |guard| guard.is_some())
    }

    fn record_scan_error(scan_error: &Arc<Mutex<Option<String>>>, message: String) {
        if let Ok(mut guard) = scan_error.lock()
            && guard.is_none()
        {
            *guard = Some(message);
        }
    }

    fn scan_run_results(
        cancel: &Arc<AtomicBool>,
        shared: &WorkerShared,
    ) -> Result<ScanRunResults, String> {
        if cancel.load(Ordering::Relaxed) {
            return Err("canceled".to_string());
        }
        let scan_error = {
            let guard = shared
                .scan_error
                .lock()
                .map_err(|_| "scan error lock poisoned".to_string())?;
            guard.clone()
        };
        if let Some(err) = scan_error {
            return Err(err);
        }

        let mods_map = {
            let guard = shared
                .mods_map
                .lock()
                .map_err(|_| "scan map lock poisoned".to_string())?;
            guard.clone()
        };
        let tp2_map = {
            let guard = shared
                .tp2_map
                .lock()
                .map_err(|_| "scan tp2 map lock poisoned".to_string())?;
            guard.clone()
        };
        let scan_cache_save_error = shared.cache.lock().map_or_else(
            |_| Some("scan cache lock poisoned during save".to_string()),
            |cache| save_scan_cache(&cache),
        );
        let mut tp2_reports = {
            let guard = shared
                .scan_reports
                .lock()
                .map_err(|_| "scan reports lock poisoned".to_string())?;
            guard.clone()
        };
        tp2_reports.sort_by(|a, b| a.tp2_path.cmp(&b.tp2_path));
        Ok(ScanRunResults {
            mods_map,
            tp2_map,
            scan_cache_save_error,
            tp2_reports,
        })
    }

    fn split_mods_by_game(
        game_install: &str,
        scanned_mods: Vec<Step2ModState>,
    ) -> (Vec<Step2ModState>, Vec<Step2ModState>) {
        match game_install {
            "BG2EE" => (Vec::new(), scanned_mods),
            "EET" => (scanned_mods.clone(), scanned_mods),
            _ => (scanned_mods, Vec::new()),
        }
    }

    fn build_scan_report(
        inputs: &ScanInputs,
        preferred_locale_info: &super::language::PreferredLocaleInfo,
        worker_count: usize,
        total_groups: usize,
        cache_meta: crate::app::scan::cache::ScanCacheLoadMeta,
        scan_cache_save_error: Option<String>,
        tp2_reports: Vec<Step2Tp2ProbeReport>,
    ) -> Step2ScanReport {
        let tp2_cache_hits = tp2_reports.iter().filter(|r| r.used_cache).count();
        let total_tp2 = tp2_reports.len();
        let current_exe_fingerprint = current_exe_fingerprint();
        let writer_app_matches = cache_match(
            tp2_cache_hits,
            cache_meta.file_writer_app_version.as_deref(),
            env!("CARGO_PKG_VERSION"),
        );
        let writer_exe_matches = cache_match(
            tp2_cache_hits,
            cache_meta.file_writer_exe_fingerprint.as_deref(),
            &current_exe_fingerprint,
        );
        Step2ScanReport {
            game_dir: inputs.game_dir.display().to_string(),
            mods_root: inputs.mods_root.display().to_string(),
            scan_depth: inputs.scan_depth,
            preferred_locale: preferred_locale_info.locale.clone(),
            preferred_locale_source: preferred_locale_info.source.clone(),
            preferred_locale_baldur_lua: preferred_locale_info
                .baldur_lua_path
                .as_ref()
                .map(|p| p.display().to_string()),
            worker_count,
            total_groups,
            total_tp2,
            tp2_cache_hits,
            tp2_cache_misses: total_tp2.saturating_sub(tp2_cache_hits),
            scan_cache_path: cache_meta.path,
            scan_cache_source: cache_meta.source,
            scan_cache_load_status: cache_meta.load_status,
            scan_cache_load_error: cache_meta.load_error,
            scan_cache_file_exists: cache_meta.file_exists,
            scan_cache_file_mtime_secs: cache_meta.file_mtime_secs,
            scan_cache_file_version: cache_meta.file_version,
            scan_cache_writer_app_version: cache_meta.file_writer_app_version,
            scan_cache_writer_exe_fingerprint: cache_meta.file_writer_exe_fingerprint,
            scan_cache_entry_count: cache_meta.file_entry_count,
            scan_cache_version_matches_current_schema: cache_meta.version_matches_current_schema,
            scan_cache_fallback_path: cache_meta.fallback_path,
            scan_cache_fallback_source: cache_meta.fallback_source,
            scan_cache_fallback_load_status: cache_meta.fallback_load_status,
            scan_cache_fallback_load_error: cache_meta.fallback_load_error,
            scan_cache_save_error,
            scan_cache_writer_matches_current_app_version: writer_app_matches,
            scan_cache_writer_matches_current_exe: writer_exe_matches,
            tp2_reports,
        }
    }

    fn cache_match(
        tp2_cache_hits: usize,
        cached_value: Option<&str>,
        expected: &str,
    ) -> Option<bool> {
        (tp2_cache_hits != 0).then(|| cached_value.is_some_and(|value| value == expected))
    }
}

#[path = "worker_scan_group.rs"]
mod scan_group;

pub use orchestrate::scan_impl;
