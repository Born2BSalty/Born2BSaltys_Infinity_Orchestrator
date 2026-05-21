// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::app_step2_update_extract::archive::extract_one_archive;
use crate::app::app_step2_update_extract::plan::{Step2UpdateExtractJob, build_extract_jobs};
use crate::app::state::WizardState;

pub const EXTRACT_POOL_SIZE: usize = 10;

pub enum ExtractAssetEvent {
    AssetDone {
        index: usize,
        ok: bool,
        label: String,
        target_or_err: String,
    },
    Finished(ExtractResult),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractResult {
    pub extracted: Vec<String>,
    pub failed: Vec<String>,
}

struct AssetExtractResult {
    index: usize,
    label: String,
    outcome: Result<String, String>,
}

pub fn start_parallel_extract(
    state: &mut WizardState,
    extract_progress: &Arc<Mutex<Option<(usize, usize)>>>,
) -> Option<Receiver<ExtractAssetEvent>> {
    if state.step2.update_selected_extract_running {
        tracing::info!(
            target = "orchestrator",
            "parallel extract not started: extractor already running"
        );
        return None;
    }
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        tracing::info!(
            target = "orchestrator",
            "parallel extract not started: archive dir is empty"
        );
        return None;
    }
    let jobs = build_extract_jobs(state, &archive_dir);
    if jobs.is_empty() {
        let failed = state.step2.update_selected_extract_failed_sources.len();
        if failed > 0 {
            state.step2.scan_status =
                format!("Extract updates finished: 0 updated, {failed} failed");
        }
        tracing::info!(
            target = "orchestrator",
            failed,
            archive_dir = %archive_dir.display(),
            "parallel extract not started: no extract jobs"
        );
        return None;
    }

    state.step2.update_selected_extract_running = true;
    state.step2.scan_status = format!("Extracting updates: 0/{}", jobs.len());
    if let Ok(mut g) = extract_progress.lock() {
        *g = Some((0, jobs.len()));
    }

    let (tx, rx) = mpsc::channel::<ExtractAssetEvent>();
    let jobs_total = jobs.len();
    tracing::info!(
        target = "orchestrator",
        jobs_total,
        archive_dir = %archive_dir.display(),
        "parallel extract starting"
    );
    thread::spawn(move || {
        run_parallel_extract(jobs, jobs_total, &tx);
    });
    Some(rx)
}

fn run_parallel_extract(
    jobs: Vec<Step2UpdateExtractJob>,
    total: usize,
    tx: &Sender<ExtractAssetEvent>,
) {
    let mut result = ExtractResult::default();
    if total == 0 {
        match tx.send(ExtractAssetEvent::Finished(result)) {
            Ok(()) => tracing::info!(
                target = "orchestrator",
                "extract coordinator: Finished sent on empty-jobs path"
            ),
            Err(err) => tracing::warn!(
                target = "orchestrator",
                "extract coordinator: Finished send failed on empty-jobs \
                 path: {err} (receiver dropped — Downloading screen will \
                 not auto-advance; install runtime hands back to user)"
            ),
        }
        return;
    }

    let jobs = Arc::new(jobs);
    let next = Arc::new(AtomicUsize::new(0));
    let results: Arc<Mutex<Vec<AssetExtractResult>>> =
        Arc::new(Mutex::new(Vec::with_capacity(total)));

    let worker_count = EXTRACT_POOL_SIZE.min(total);
    let mut handles = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let jobs = Arc::clone(&jobs);
        let next = Arc::clone(&next);
        let results = Arc::clone(&results);
        let tx = tx.clone();
        handles.push(thread::spawn(move || {
            worker_loop(&jobs, &next, &results, &tx);
        }));
    }

    for (i, h) in handles.into_iter().enumerate() {
        if let Err(panic) = h.join() {
            tracing::warn!(
                target = "orchestrator",
                "extract worker {i}: panicked during join: {panic:?} \
                 (coordinator continues; missing AssetDone events will \
                 leave the Extract bar short by one or more counts)"
            );
        }
    }

    let mut recs = results.lock().expect("extract results mutex");
    recs.sort_by_key(|r| r.index);
    for rec in recs.iter() {
        match &rec.outcome {
            Ok(target_display) => result
                .extracted
                .push(format!("{} -> {}", rec.label, target_display)),
            Err(err) => result.failed.push(format!("{}: {}", rec.label, err)),
        }
    }
    drop(recs);

    let extracted = result.extracted.len();
    let failed = result.failed.len();
    tracing::info!(
        target = "orchestrator",
        extracted,
        failed,
        "parallel extract coordinator: all workers joined"
    );
    match tx.send(ExtractAssetEvent::Finished(result)) {
        Ok(()) => tracing::info!(
            target = "orchestrator",
            extracted,
            failed,
            "extract coordinator: Finished sent"
        ),
        Err(err) => tracing::warn!(
            target = "orchestrator",
            "extract coordinator: Finished send failed: {err} (receiver \
             dropped — Downloading screen will not auto-advance; install \
             runtime stays armed until user action)"
        ),
    }
}

fn worker_loop(
    jobs: &Arc<Vec<Step2UpdateExtractJob>>,
    next: &Arc<AtomicUsize>,
    results: &Arc<Mutex<Vec<AssetExtractResult>>>,
    tx: &Sender<ExtractAssetEvent>,
) {
    loop {
        let index = next.fetch_add(1, Ordering::SeqCst);
        if index >= jobs.len() {
            break;
        }
        let job = &jobs[index];
        let label = job.label.clone();
        match extract_one_archive(job) {
            Ok(target_root) => {
                let target_display = target_root.display().to_string();
                results
                    .lock()
                    .expect("extract results mutex")
                    .push(AssetExtractResult {
                        index,
                        label: label.clone(),
                        outcome: Ok(target_display.clone()),
                    });
                match tx.send(ExtractAssetEvent::AssetDone {
                    index,
                    ok: true,
                    label: label.clone(),
                    target_or_err: target_display,
                }) {
                    Ok(()) => tracing::info!(
                        target = "orchestrator",
                        index,
                        label = %label,
                        "extract worker: AssetDone sent"
                    ),
                    Err(err) => tracing::warn!(
                        target = "orchestrator",
                        "extract worker: AssetDone send failed for \
                         index {index} ({label}): {err} (receiver \
                         dropped — progress bar will stall)"
                    ),
                }
            }
            Err(err) => {
                results
                    .lock()
                    .expect("extract results mutex")
                    .push(AssetExtractResult {
                        index,
                        label: label.clone(),
                        outcome: Err(err.clone()),
                    });
                match tx.send(ExtractAssetEvent::AssetDone {
                    index,
                    ok: false,
                    label: label.clone(),
                    target_or_err: err,
                }) {
                    Ok(()) => tracing::info!(
                        target = "orchestrator",
                        index,
                        label = %label,
                        "extract worker: AssetDone sent for failed archive"
                    ),
                    Err(send_err) => tracing::warn!(
                        target = "orchestrator",
                        "extract worker: AssetDone (failure) send \
                         failed for index {index} ({label}): \
                         {send_err} (receiver dropped — failed \
                         archive will not appear in the Downloading \
                         screen's failed list)"
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_extract_par_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_archive_dir_early_returns_none() {
        let mut state = WizardState::default();
        let handle = Arc::new(Mutex::new(None));
        let r = start_parallel_extract(&mut state, &handle);
        assert!(r.is_none(), "empty archive_dir ⇒ early-return None");
        assert!(
            !state.step2.update_selected_extract_running,
            "running flag not set on early-return"
        );
        assert!(
            handle.lock().unwrap().is_none(),
            "extract_progress handle untouched on early-return"
        );
    }

    #[test]
    fn reentry_guard_returns_none_when_already_running() {
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = td().to_string_lossy().into_owned();
        state.step2.update_selected_extract_running = true;
        let handle = Arc::new(Mutex::new(None));
        assert!(
            start_parallel_extract(&mut state, &handle).is_none(),
            "a second start while running is a no-op (BIO serial parity)"
        );
    }

    #[test]
    fn empty_jobs_early_return_after_archive_dir_check() {
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = td().to_string_lossy().into_owned();
        let handle = Arc::new(Mutex::new(None));
        let r = start_parallel_extract(&mut state, &handle);
        assert!(r.is_none(), "no assets ⇒ no jobs ⇒ None");
        assert!(
            !state.step2.update_selected_extract_running,
            "running flag not set when jobs are empty"
        );
    }

    #[test]
    fn run_parallel_extract_with_zero_total_sends_only_finished() {
        let (tx, rx) = mpsc::channel::<ExtractAssetEvent>();
        run_parallel_extract(Vec::new(), 0, &tx);
        let mut got_finished = false;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                ExtractAssetEvent::Finished(r) => {
                    assert!(r.extracted.is_empty() && r.failed.is_empty());
                    got_finished = true;
                }
                ExtractAssetEvent::AssetDone { .. } => {
                    panic!("unexpected AssetDone on empty-jobs path");
                }
            }
        }
        assert!(got_finished);
    }

    #[test]
    fn extract_pool_size_is_ten() {
        assert_eq!(EXTRACT_POOL_SIZE, 10);
    }

    #[test]
    fn asset_done_carries_label_and_target_or_err() {
        let ev = ExtractAssetEvent::AssetDone {
            index: 7,
            ok: true,
            label: "MyMod".to_string(),
            target_or_err: "C:\\Mods\\MyMod".to_string(),
        };
        match ev {
            ExtractAssetEvent::AssetDone {
                index,
                ok,
                label,
                target_or_err,
            } => {
                assert_eq!(index, 7);
                assert!(ok);
                assert_eq!(label, "MyMod");
                assert_eq!(target_or_err, "C:\\Mods\\MyMod");
            }
            ExtractAssetEvent::Finished(_) => panic!("not AssetDone"),
        }
    }

    #[test]
    fn extract_result_format_matches_serial_loop() {
        let r = ExtractResult {
            extracted: vec!["MyMod -> C:\\Mods\\MyMod".to_string()],
            failed: vec!["BadMod: archive corrupt".to_string()],
        };
        assert_eq!(r.extracted[0], "MyMod -> C:\\Mods\\MyMod");
        assert_eq!(r.failed[0], "BadMod: archive corrupt");
    }

    #[test]
    fn extract_progress_total_tracks_actual_jobs_not_asset_count() {
        use crate::app::app_step2_update_download::archive_file_name;
        use crate::app::state::Step2UpdateAsset;

        let archive_dir = td();
        let mods_folder = td();

        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        state.step1.mods_folder = mods_folder.to_string_lossy().into_owned();

        let assets: Vec<Step2UpdateAsset> = (0..3)
            .map(|i| Step2UpdateAsset {
                game_tab: "BGEE".to_string(),
                tp_file: format!("MOD{i}/MOD{i}.TP2"),
                label: format!("MOD{i}"),
                source_id: "github".to_string(),
                tag: "v1".to_string(),
                asset_name: format!("MOD{i}-v1.zip"),
                asset_url: format!("https://example/MOD{i}-v1.zip"),
                installed_source_ref: None,
            })
            .collect();
        for asset in assets.iter().take(2) {
            let name = archive_file_name(asset);
            std::fs::write(archive_dir.join(&name), b"fake-archive-body").unwrap();
        }
        state.step2.update_selected_update_assets = assets;

        let handle = Arc::new(Mutex::new(None));
        let rx = start_parallel_extract(&mut state, &handle);
        assert!(
            rx.is_some(),
            "two on-disk archives ⇒ at least one job ⇒ Some(rx)"
        );

        let (completed, total) = {
            let progress = handle.lock().unwrap();
            progress.expect("extract_progress was initialised by start")
        };
        assert_eq!(completed, 0, "Extract bar starts at 0/total");
        assert_eq!(
            total, 2,
            "Extract bar denominator MUST equal the actual job count \
             (two archives on disk), NOT the asset-list count (three) — \
             a stale denominator would freeze the bar at N-1 / N forever"
        );

        let _ = std::fs::remove_dir_all(&archive_dir);
        let _ = std::fs::remove_dir_all(&mods_folder);
    }
}
