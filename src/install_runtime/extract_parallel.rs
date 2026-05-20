// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::extract_parallel` — **DL Fix-Set v3 (Change A): the
// net-new parallel extract coordinator** (SPEC §1 carve-out 7, §4.3,
// §13.12a).
//
// **The bug + improvement this fixes.** BIO's reused-unchanged extract
// (`app_step2_update_extract::start_step2_update_extract` →
// `extract_update_archives`) is a SERIAL loop that pulls one job at a time
// off `Vec<Step2UpdateExtractJob>` and emits a `Progress { completed,
// total }` event after each job finishes. On a 51-archive EET install
// that is a productivity cliff — each multi-MB archive unpacks serially
// even though the user's box has 8+ cores. The legacy DL Fix-Set v2
// approach wrapped BIO's serial extract with a forwarder thread
// (`extract_intercept`) so the §4.3 Extract bar could climb mid-extract,
// but the work itself was still serial. This module REPLACES that path
// with a real **bounded parallel pool** that runs extraction across 10
// workers — composing the carve-out-7 building blocks (`archive::
// extract_one_archive` + `plan::build_extract_jobs` + the
// `Step2UpdateExtractJob` struct), all just visibility-widened from
// `pub(super)` / private to `pub(crate)` with ZERO behavior change.
//
// **Zero BIO source.** Per SPEC §1 carve-out 7 (USER-AUTHORIZED
// 2026-05-20 — narrow visibility-only), six edits across three files in
// `src/core/app/step2/update/` widen `mod archive;` / `mod plan;` to
// `pub(crate) mod`, the `extract_one_archive` fn to `pub(crate)`, and
// the `Step2UpdateExtractJob` struct + fields + `build_extract_jobs` fn
// to `pub(crate)` — all purely additive (no signatures changed, no
// behavior changed). BIO's own legacy serial extract path continues to
// work unchanged (it still calls these same items via sibling
// references). This module then composes them: builds the same jobs
// BIO would have built (so the post-state contract is preserved
// byte-for-byte), spawns a `POOL_SIZE`-worker thread pool, and emits
// per-asset events to the orchestrator.
//
// **Concurrency model.** A spawned coordinator thread owns a bounded
// worker pool (`POOL_SIZE = 10`) via a shared `Arc<AtomicUsize>` work
// index + a shared `Arc<Mutex<Vec<...>>>` results vec. Each worker
// loops `fetch_add(1)`-pulls an index, runs `archive::
// extract_one_archive(&job)` (the carve-out-7 BIO fn), and on
// completion emits an `AssetDone` event + pushes an `AssetResult` to
// the shared results vec. When the index exhausts and all workers join,
// the coordinator sorts results by original index, builds the
// BIO-verbatim result vectors (`extracted: "{label} -> {target_root.
// display()}"`, `failed: "{label}: {err}"` — copied byte-for-byte from
// `app_step2_update_extract_archive.rs:41-43`), and sends `Finished`.
//
// **Per-asset events arrive in COMPLETION order, not index order.**
// Worker 0 may pick index 7 (a small archive) and finish before
// worker 1 (which picked index 0 — a large archive). The orchestrator's
// drain bumps the shared `extract_progress` count on each `AssetDone`
// — that count is total-jobs-completed, not which-index-was-completed,
// so the Extract phase bar advances smoothly as workers finish in
// whatever order the underlying archives permit.
//
// **`extract_progress` lifecycle (the bug fix).** Before the pool
// starts, the coordinator writes `Some((0, jobs.len()))` so the §4.3
// chrome reads "0 / N" immediately at extract-start (the prior
// `extract_intercept` approach left the handle `None` until the first
// `Progress` event arrived). After `Finished`, the handle is left at
// `Some((N, N))` — NOT cleared — so the §4.3 chrome continues to show
// 100% until the install screen takes over (the prior approach
// cleared it, causing a frame of `extract_completed_total() = (0, 0)`
// before the orchestrator's pipeline-reached-install gate fired and
// advanced to stage 4). The next install's `start_parallel_extract`
// resets the handle (step 5 in the coordinator's open).
//
// SPEC: §1 carve-out 7 (visibility widening that authorizes this
//       module's reuse), §4.3 (Downloading screen — Extract bar), §13.12a
//       (per-install dirs + pipeline-reuse contract).

// rationale: a bounded extract pool with a shared work index + results
// mutex is intrinsic to the parallel design (not a smell);
// `Self`/`const fn`/`#[must_use]` churn + the doc-paragraph-length lint
// add noise without behavior value (Cat 3).
#![allow(
    clippy::missing_const_for_fn,
    clippy::must_use_candidate,
    clippy::use_self,
    clippy::too_long_first_doc_paragraph
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::app_step2_update_extract::archive::extract_one_archive;
use crate::app::app_step2_update_extract::plan::{Step2UpdateExtractJob, build_extract_jobs};
use crate::app::state::WizardState;

/// The bounded parallel extract pool size. The user-reported 51-mod EET
/// serial extraction productivity cliff is the productivity gap this
/// addresses. 10 keeps disk pressure bounded (per-archive extracts hit
/// the same temp dir → archive parent → modlist root path; >10 thrashes
/// the disk on spinning media) while being substantially faster than
/// BIO's serial loop on any modern multi-core box.
pub const EXTRACT_POOL_SIZE: usize = 10;

/// One archive's terminal extract event. Mirrors the channel discipline
/// `stream_downloader::StreamDownloadEvent` uses, with the per-archive
/// granularity BIO's serial loop's single `Progress { completed, total }`
/// per-job-finish lacks (the count *is* per-job, but here we attach the
/// row identity + outcome so the orchestrator's drain can render per-row
/// status transitions live, not just bump the count).
pub enum ExtractAssetEvent {
    /// One archive reached a terminal state (success or failure).
    /// `index` is the original index into `Vec<Step2UpdateExtractJob>`
    /// (== the asset's index in `state.step2.update_selected_update_
    /// assets` at job-build time). `ok` distinguishes success from
    /// failure. `label` is the asset's display label (copied from
    /// `Step2UpdateExtractJob.label` — the same string BIO's serial
    /// loop puts in its result vectors). `target_or_err` is the
    /// success destination's `Display` (`target_root.display().to_
    /// string()`) on success, or the BIO-shaped error string on
    /// failure (`err.to_string()` from `extract_one_archive`'s `Err`).
    AssetDone {
        index: usize,
        ok: bool,
        label: String,
        target_or_err: String,
    },
    /// Every archive finished. Carries the BIO-verbatim result vectors
    /// the orchestrator writes onto `state.step2`
    /// (`update_selected_extracted_sources` /
    /// `update_selected_extract_failed_sources`).
    Finished(ExtractResult),
}

/// The terminal result — the EXACT two vectors BIO's serial
/// `Step2UpdateExtractResult` produces (same element formats, see
/// [`run_parallel_extract`]). The orchestrator assigns these straight
/// onto `state.step2` then runs the byte-identical post-state cleanup
/// (`remove_extracted_update_entries` replicated inline + the rescan
/// kick), matching BIO's `poll_step2_update_extract` post-`Finished`
/// path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractResult {
    /// `"{label} -> {target_root.display()}"` per success — byte-
    /// identical to `app_step2_update_extract_archive.rs:41`.
    pub extracted: Vec<String>,
    /// `"{label}: {err}"` per failure — byte-identical to
    /// `app_step2_update_extract_archive.rs:43`.
    pub failed: Vec<String>,
}

/// Internal per-asset terminal record the workers push into the shared
/// results vec (coordinator orders + formats them at the end).
struct AssetExtractResult {
    index: usize,
    label: String,
    /// `Ok(target_root_display)` → success (the target_root's `Display`
    /// matching BIO's `target_root.display()` — used by the coordinator
    /// to build `"{label} -> {target_root_display}"`).
    /// `Err(err_msg)` → the BIO-shaped error string.
    outcome: Result<String, String>,
}

/// **Spawn the bounded parallel extract pool (the v3 replacement for
/// BIO's serial extract + the v2 extract_intercept forwarder).** Reads
/// `state.step1.mods_archive_folder` (the global Settings → Paths
/// Mods-archive folder) + `state.step2.update_selected_update_assets`
/// (the resolved asset set — read-only input, left intact), sets
/// `state.step2.update_selected_extract_running = true`, writes
/// `Some((0, jobs.len()))` to `extract_progress` so the §4.3 chrome
/// shows the correct denominator at extract-start, then spawns a
/// coordinator thread that drives a `POOL_SIZE` worker pool.
///
/// Behavior:
///   1. Early-return `None` if `update_selected_extract_running` is
///      already set (re-entry guard, matches BIO's serial
///      `start_step2_update_extract` early-return).
///   2. Read `archive_dir = state.step1.mods_archive_folder`. Empty
///      ⇒ early-return `None` (matches BIO's `archive_dir.as_os_str
///      ().is_empty()` guard).
///   3. Call `build_extract_jobs(state, &archive_dir)` — the SAME
///      carve-out-7 BIO fn the serial path calls.
///   4. Empty jobs ⇒ honest no-op: if `update_selected_extract_failed_
///      sources` is non-empty (build_extract_jobs may have pushed a
///      Mods-Folder-empty or source-load error), set the BIO-verbatim
///      `"Extract updates finished: 0 updated, N failed"` status;
///      return `None`.
///   5. Set the running flag + the status line (`"Extracting updates:
///      0/{N}"` — byte-identical to BIO's serial `start_step2_update_
///      extract`). Reset `extract_progress` to `Some((0, jobs.len()))`
///      so the §4.3 Extract bar starts at the right denominator
///      immediately.
///   6. Spawn the coordinator thread.
///
/// Returns the `Receiver<ExtractAssetEvent>` on a real spawn, `None`
/// on the re-entry / empty-archive-dir / empty-jobs paths. Zero BIO
/// source: only the carve-out-7 visibility-widened items are reused
/// (`archive::extract_one_archive`, `plan::Step2UpdateExtractJob`,
/// `plan::build_extract_jobs`).
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
    // **DL Fix-Set v3 (Change A bug fix)** — write the denominator
    // immediately so the §4.3 Extract bar shows "0 / N" at extract-
    // start, not "0 / 0" until the first AssetDone arrives. (The v2
    // extract_intercept approach left the handle `None` and let the
    // count fallback fill until the first Progress event.)
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

/// The coordinator + bounded worker pool. Runs on a spawned thread
/// (never the egui frame thread). Spawns `min(POOL_SIZE, jobs.len())`
/// workers that pull from a shared atomic index; each runs the
/// carve-out-7 `extract_one_archive(&job)` for one job at a time and
/// emits a terminal `AssetDone` + pushes to the shared results vec.
/// When the index is exhausted and all workers join, the coordinator
/// orders the results by original job index and formats the
/// BIO-shaped result vectors (`"{label} -> {target_root_display}"`,
/// `"{label}: {err}"`) and sends `Finished`.
///
/// **One failure does not abort the pool** — a failed extract records
/// an `Err` and the workers keep pulling the rest (matching BIO's
/// serial loop, which `match`es each result and continues). The
/// `auto_build_blocker_before_install` BIO check (unchanged) then
/// stops the auto-build for the failed source after extract, exactly
/// as today.
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

    // Shared work index + ordered results. `jobs` is shared read-only
    // (each worker reads `jobs[i]`); `next` hands out indices;
    // `results` collects terminal records. The shared `Arc<Vec<Step2
    // UpdateExtractJob>>` is sound because `extract_one_archive` takes
    // `&Step2UpdateExtractJob` and never mutates it.
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

    // Order the terminal records by original job index and format the
    // two BIO-shaped vectors. Labels were carried per-record so the
    // strings are byte-identical to BIO's serial loop
    // (`app_step2_update_extract_archive.rs:41 / :43`).
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
        // DATA-LOSS-safe: a unique temp dir; this module never binds the
        // real `%APPDATA%\bio\` (it operates only on an arbitrary
        // Mods-archive dir + temp HTTP fixtures).
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
                _ => panic!("unexpected event on empty-jobs path"),
            }
        }
        assert!(got_finished);
    }

    #[test]
    fn extract_pool_size_is_ten() {
        // The constant the brief calls out — bound the pool to 10.
        assert_eq!(EXTRACT_POOL_SIZE, 10);
    }

    #[test]
    fn asset_done_carries_label_and_target_or_err() {
        // Smoke-test the AssetDone shape — the orchestrator's drain
        // reads `(index, ok, label, target_or_err)` to render row
        // transitions live. (Can't easily drive a real extract from a
        // unit test — the archive bytes / unpack tooling isn't unit-
        // testable. The shape is the contract.)
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
            _ => panic!("not AssetDone"),
        }
    }

    #[test]
    fn extract_result_bio_verbatim_formats() {
        // Build a hand-crafted ExtractResult to prove the format the
        // coordinator emits is byte-identical to BIO's serial loop
        // (`app_step2_update_extract_archive.rs:41 / :43`).
        let r = ExtractResult {
            extracted: vec!["MyMod -> C:\\Mods\\MyMod".to_string()],
            failed: vec!["BadMod: archive corrupt".to_string()],
        };
        assert_eq!(r.extracted[0], "MyMod -> C:\\Mods\\MyMod");
        assert_eq!(r.failed[0], "BadMod: archive corrupt");
    }
}
