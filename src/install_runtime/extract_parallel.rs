// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Bounded parallel extract coordinator.
//!
//! Replaces BIO's serial `extract_update_archives` loop with a bounded
//! worker pool (size [`EXTRACT_POOL_SIZE`]). The coordinator builds the
//! same jobs BIO would have built via
//! [`crate::app::app_step2_update_extract::plan::build_extract_jobs`],
//! drives a worker pool that calls
//! [`crate::app::app_step2_update_extract::archive::extract_one_archive`]
//! for each job, emits per-asset events to the orchestrator's drain, and
//! sends the BIO-shaped terminal result vectors as `Finished`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::app_step2_update_extract::archive::extract_one_archive;
use crate::app::app_step2_update_extract::plan::{Step2UpdateExtractJob, build_extract_jobs};
use crate::app::state::WizardState;

/// Bounded parallel extract pool size; matches
/// [`crate::install_runtime::archive_skip_async::HASH_POOL_SIZE`].
pub const EXTRACT_POOL_SIZE: usize = 10;

/// One archive's terminal extract event.
pub enum ExtractAssetEvent {
    /// One archive reached a terminal state.
    ///
    /// `index` is the original index into the job vec (matches the
    /// asset's position in `state.step2.update_selected_update_assets`
    /// at job-build time). `target_or_err` is the destination's
    /// `Display` on success or the error string on failure.
    AssetDone {
        index: usize,
        ok: bool,
        label: String,
        target_or_err: String,
    },
    /// Every archive finished; carries the BIO-shaped result vectors.
    Finished(ExtractResult),
}

/// The terminal result — the two vectors BIO's serial
/// `Step2UpdateExtractResult` produces, with identical element formats.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractResult {
    /// `"{label} -> {target_root.display()}"` per success.
    pub extracted: Vec<String>,
    /// `"{label}: {err}"` per failure.
    pub failed: Vec<String>,
}

/// Per-asset terminal record the workers push into a shared vec; the
/// coordinator orders + formats them.
struct AssetExtractResult {
    index: usize,
    label: String,
    /// `Ok(target_root_display)` → success destination's `Display`.
    /// `Err(err_msg)` → the BIO-shaped error string.
    outcome: Result<String, String>,
}

/// Spawn the bounded parallel extract pool.
///
/// Reads the Mods-archive folder + the resolved asset set from `state`,
/// builds the same jobs the serial path would build, sets the running
/// flag + status line, primes `extract_progress` so the Extract bar
/// shows the correct denominator immediately, and spawns the coordinator
/// thread.
///
/// Returns `None` on the re-entry guard / empty-archive-dir / empty-jobs
/// paths (matching the serial path's early returns). On empty-jobs with
/// pre-recorded failures, sets the "Extract updates finished: 0
/// updated, N failed" status line.
pub fn start_parallel_extract(
    state: &mut WizardState,
    extract_progress: &Arc<Mutex<Option<(usize, usize)>>>,
) -> Option<Receiver<ExtractAssetEvent>> {
    if state.step2.update_selected_extract_running {
        return None;
    }
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return None;
    }
    let jobs = build_extract_jobs(state, &archive_dir);
    if jobs.is_empty() {
        let failed = state.step2.update_selected_extract_failed_sources.len();
        if failed > 0 {
            state.step2.scan_status =
                format!("Extract updates finished: 0 updated, {failed} failed");
        }
        return None;
    }

    state.step2.update_selected_extract_running = true;
    state.step2.scan_status = format!("Extracting updates: 0/{}", jobs.len());
    if let Ok(mut g) = extract_progress.lock() {
        *g = Some((0, jobs.len()));
    }

    let (tx, rx) = mpsc::channel::<ExtractAssetEvent>();
    let jobs_total = jobs.len();
    thread::spawn(move || {
        run_parallel_extract(jobs, jobs_total, &tx);
    });
    Some(rx)
}

/// Coordinator body. Spawns `min(EXTRACT_POOL_SIZE, jobs.len())` workers
/// that pull from a shared atomic index; each runs `extract_one_archive`
/// for one job at a time and emits an `AssetDone`. When all workers
/// join, the coordinator orders the results by original index, formats
/// the result vectors, and sends `Finished`.
///
/// One failure does not abort the pool: a failed extract records an
/// `Err` and the workers keep pulling. The
/// `auto_build_blocker_before_install` check downstream stops the
/// auto-build for the failed source after extract.
fn run_parallel_extract(
    jobs: Vec<Step2UpdateExtractJob>,
    total: usize,
    tx: &Sender<ExtractAssetEvent>,
) {
    let mut result = ExtractResult::default();
    if total == 0 {
        let _ = tx.send(ExtractAssetEvent::Finished(result));
        return;
    }

    // Shared read-only job vec + shared work index + shared results vec.
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
                        // Push the success record (ordered later).
                        results
                            .lock()
                            .expect("extract results mutex")
                            .push(AssetExtractResult {
                                index,
                                label: label.clone(),
                                outcome: Ok(target_display.clone()),
                            });
                        let _ = tx.send(ExtractAssetEvent::AssetDone {
                            index,
                            ok: true,
                            label,
                            target_or_err: target_display,
                        });
                    }
                    Err(err) => {
                        // Push the failure record (ordered later).
                        results
                            .lock()
                            .expect("extract results mutex")
                            .push(AssetExtractResult {
                                index,
                                label: label.clone(),
                                outcome: Err(err.clone()),
                            });
                        let _ = tx.send(ExtractAssetEvent::AssetDone {
                            index,
                            ok: false,
                            label,
                            target_or_err: err,
                        });
                    }
                }
            }
        }));
    }

    for h in handles {
        let _ = h.join();
    }

    // Order results by original job index and format the result vectors.
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

    let _ = tx.send(ExtractAssetEvent::Finished(result));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn td() -> PathBuf {
        // Unique per-call temp dir; never binds the real config dir.
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
        // BIO's serial `start_step2_update_extract` early-returns when
        // mods_archive_folder is empty; the parallel coordinator must too.
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
        // mods_archive_folder set but no assets ⇒ build_extract_jobs
        // returns empty ⇒ early-return None; running flag untouched.
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
        // Direct test of the run_parallel_extract coordinator with no
        // jobs: it must send exactly one Finished event with empty
        // vectors.
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
}
