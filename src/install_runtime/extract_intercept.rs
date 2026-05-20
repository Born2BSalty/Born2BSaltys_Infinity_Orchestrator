// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `install_runtime::extract_intercept` — **Fix 1c: extract-progress
// intercept** (DL Fix-Set v2; SPEC §4.3).
//
// **The bug this fixes.** BIO's `Step2UpdateExtractEvent::Progress {
// completed, total }` (`app_step2_update_extract.rs:25`) is the ONLY
// per-archive extract signal BIO emits, and BIO's own `poll_step2_update_
// extract` only writes it to `state.step2.scan_status` (a string). The
// `update_selected_extracted_sources` vector — which the §4.3 grid reads
// via `extracted_count()` to derive the Extract bar — is bulk-assigned
// only at extract-`Finished`. Mid-extract the vector is empty ⇒ zero rows
// are `Staged` ⇒ the Extract phase bar reads 0/N until the very end then
// snaps to N/N. The §4.3 chrome's "N / T mods" line shows 0 the whole time.
//
// **The fix.** Wrap BIO's extract receiver with a forwarder thread that
// (a) snapshots each `Progress` event's `(completed, total)` into a shared
// `Arc<Mutex<Option<(usize, usize)>>>` the orchestrator reads every frame,
// then (b) forwards the event unchanged onto an intermediate channel BIO's
// `poll_step2_update_extract` drains. The orchestrator's
// `DownloadProgress` checks the shared handle in the Extract phase and
// shows a real `completed / total` Extract bar instead of the count-based
// `extracted_count() / extract_total()` (which doesn't update until
// `Finished`).
//
// **Zero BIO source.** This module:
//
//   - calls BIO's `pub(crate)` `app_step2_update_extract::start_step2_
//     update_extract` (the SAME entry BIO's `poll_step2_update_download`
//     calls) READ-ONLY to do the actual extract spawn;
//   - intercepts ONLY the receiver-end stream, with a forwarder thread —
//     BIO's `Step2UpdateExtractEvent` enum and its
//     `poll_step2_update_extract` consumer are touched zero times;
//   - publishes the per-Progress snapshot into a `Arc<Mutex<...>>` the
//     orchestrator owns (`OrchestratorApp::extract_progress`).
//
// **The forwarder thread's invariants.**
//   - On `Progress { completed, total }`: write `Some((completed, total))`
//     into the handle, then forward the event onto the intermediate
//     channel so BIO's `poll_step2_update_extract` still gets it (and
//     updates the `scan_status` string).
//   - On `Finished(...)`: write `None` into the handle (extract phase is
//     over; the install screen takes over per the existing 0/0-flash
//     hold logic), then forward the event.
//   - On receiver disconnect: drop the intermediate sender (BIO's poll
//     sees the same disconnect it would have seen from the original
//     channel).
//
// **Empty-jobs honest early-return.** BIO's `start_step2_update_extract`
// honestly early-returns (no channel population) when there are no jobs
// to extract (the empty per-install Mods folder case). When this happens
// we leave `bio_extract_rx_slot` as `None`, clear the progress handle,
// and return `false`. The caller knows extract was a no-op (the all-
// cached path proceeds to install via BIO's unchanged
// `start_auto_build_install`).
//
// SPEC: §4.3 (Downloading screen — the two-phase Extract 0→N bar),
//       §13.12a (the pipeline-reuse contract), §1 (CRITICAL DIRECTIVE
//       — net-new, zero BIO source).

// rationale: a thin forwarder thread + a shared `Arc<Mutex<...>>`
// progress handle is intrinsic to intercepting BIO's `pub(crate)`
// receiver-end stream without touching the BIO enum / poll. The
// `Self`/`const fn`/`#[must_use]` and doc-paragraph-length lints add
// noise without behavior value (Cat 3).
#![allow(
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::too_long_first_doc_paragraph
)]

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::app_step2_update_extract::{Step2UpdateExtractEvent, start_step2_update_extract};
use crate::app::state::WizardState;

/// **Fix 1c — kick BIO's extract through an intermediate channel that
/// snapshots `Progress` events into a shared `(completed, total)` handle
/// the orchestrator reads every frame.** The Extract phase bar uses the
/// snapshot for a real `completed / total` mid-extract (instead of the
/// count-based fallback that doesn't update until `Finished`).
///
/// Behavior:
///   1. Reset the progress handle to `None` (a fresh run starts with no
///      progress; the §4.3 grid then falls back to the count path until
///      the first `Progress` arrives).
///   2. Call BIO's `start_step2_update_extract` into a temporary
///      `Option<Receiver<...>>`.
///   3. If BIO honestly early-returned (empty jobs ⇒ no spawn ⇒
///      `temp_slot` stays `None`), leave `bio_extract_rx_slot` as `None`,
///      keep the handle cleared, return `false`.
///   4. Otherwise wire the original receiver to a forwarder thread that
///      writes `Some((completed, total))` into the handle on every
///      `Progress` (and `None` on `Finished`) then forwards the event
///      onto a fresh `(int_tx, int_rx) = mpsc::channel()`. Store `int_rx`
///      into `bio_extract_rx_slot` so BIO's unchanged
///      `poll_step2_update_extract` drains the intermediate channel
///      transparently. Return `true`.
///
/// Returns `true` on a real extract spawn (the thread is alive),
/// `false` on BIO's honest empty-jobs early-return. Zero BIO source —
/// only the receiver-end stream is intercepted.
///
/// `pub(crate)` because the signature names BIO's `pub(crate)`
/// `Step2UpdateExtractEvent` (the same visibility rule as the
/// `OrchestratorApp` Step-2 receiver fields); the in-crate
/// `drain_stream_download` is the only caller.
pub(crate) fn start_extract_with_intercept(
    state: &mut WizardState,
    bio_extract_rx_slot: &mut Option<Receiver<Step2UpdateExtractEvent>>,
    progress_handle: &Arc<Mutex<Option<(usize, usize)>>>,
) -> bool {
    // Reset the handle (a fresh run starts with no progress — the §4.3
    // grid then uses its count fallback until the first `Progress`
    // arrives from the forwarder).
    if let Ok(mut g) = progress_handle.lock() {
        *g = None;
    }

    // Call BIO's `start_step2_update_extract` into a temp slot so we can
    // distinguish "real spawn" (channel populated) from "honest empty
    // early-return" (still `None` after the call).
    let mut temp_slot: Option<Receiver<Step2UpdateExtractEvent>> = None;
    start_step2_update_extract(state, &mut temp_slot);
    let Some(orig_rx) = temp_slot else {
        // BIO honestly early-returned (empty jobs / empty Mods folder).
        // Leave the orchestrator's slot as `None` and the handle cleared;
        // the all-cached path proceeds via BIO's unchanged install
        // hand-off (the `modlist_auto_build_waiting_for_install` flag the
        // caller already set drives `start_auto_build_install`).
        *bio_extract_rx_slot = None;
        return false;
    };

    // Real spawn — wire the forwarder.
    let (int_tx, int_rx) = mpsc::channel::<Step2UpdateExtractEvent>();
    let handle = Arc::clone(progress_handle);
    thread::spawn(move || {
        while let Ok(event) = orig_rx.recv() {
            // Snapshot Progress into the shared handle BEFORE forwarding
            // (so the orchestrator's next frame can read it). Match the
            // event without moving so we can forward unchanged.
            match &event {
                Step2UpdateExtractEvent::Progress { completed, total } => {
                    if let Ok(mut g) = handle.lock() {
                        *g = Some((*completed, *total));
                    }
                }
                Step2UpdateExtractEvent::Finished(_) => {
                    // Extract phase is over; the install screen takes
                    // over (the existing 0/0-flash hold logic). Clear
                    // the handle so the next phase's bar starts clean.
                    if let Ok(mut g) = handle.lock() {
                        *g = None;
                    }
                }
            }
            if int_tx.send(event).is_err() {
                // The intermediate receiver was dropped (orchestrator
                // tore down the install screen). BIO's worker keeps
                // running until it naturally finishes — that's fine; we
                // just stop forwarding.
                break;
            }
        }
        // orig_rx disconnected (BIO worker thread ended); the forwarder
        // exits and `int_tx` drops, which `poll_step2_update_extract`
        // sees as a clean disconnect.
    });
    *bio_extract_rx_slot = Some(int_rx);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    fn td() -> PathBuf {
        // DATA-LOSS-safe: a unique temp dir; this module never binds the
        // real `%APPDATA%\bio\` (it operates only on an arbitrary
        // Mods-archive dir).
        static C: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "bio_extract_intercept_test_{}_{}",
            std::process::id(),
            C.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn empty_jobs_early_return_leaves_slot_none_and_returns_false() {
        // BIO's `start_step2_update_extract` honestly early-returns when
        // `mods_archive_folder` is empty (its own guard). The intercept
        // must propagate that as `false` + slot stays `None`.
        let mut state = WizardState::default();
        // mods_archive_folder left empty ⇒ BIO early-returns at its
        // archive_dir.as_os_str().is_empty() guard.
        let mut slot: Option<Receiver<Step2UpdateExtractEvent>> = None;
        let handle = Arc::new(Mutex::new(None));
        let r = start_extract_with_intercept(&mut state, &mut slot, &handle);
        assert!(!r, "empty-jobs early-return ⇒ false");
        assert!(
            slot.is_none(),
            "no real spawn ⇒ orchestrator slot stays None"
        );
        // Handle was reset to None at entry and stays None on early-return.
        assert!(handle.lock().unwrap().is_none());
    }

    #[test]
    fn intercept_returns_false_when_no_archive_to_extract() {
        // mods_archive_folder set but the per-install Mods folder is
        // empty ⇒ build_extract_jobs returns empty jobs ⇒ BIO honestly
        // early-returns without spawning. The intercept reports `false`.
        let archive_dir = td();
        let mut state = WizardState::default();
        state.step1.mods_archive_folder = archive_dir.to_string_lossy().into_owned();
        let mut slot: Option<Receiver<Step2UpdateExtractEvent>> = None;
        let handle = Arc::new(Mutex::new(None));
        let r = start_extract_with_intercept(&mut state, &mut slot, &handle);
        assert!(
            !r,
            "no resolved assets ⇒ no jobs ⇒ BIO honest early-return ⇒ false"
        );
        assert!(slot.is_none());
        let _ = std::fs::remove_dir_all(&archive_dir);
    }

    #[test]
    fn forwarder_snapshots_progress_into_handle_and_clears_on_finished() {
        // Drive the forwarder via a hand-built channel (we can't easily
        // make BIO's real extract spawn jobs from a unit test — the
        // archive bytes / unpack tooling isn't unit-testable). Instead,
        // build the forwarder pieces by hand and prove the snapshot
        // logic: a `Progress` event ⇒ handle holds (completed, total);
        // `Finished` ⇒ handle clears.
        let (orig_tx, orig_rx) =
            mpsc::channel::<crate::app::app_step2_update_extract::Step2UpdateExtractEvent>();
        let (int_tx, int_rx) = mpsc::channel();
        let handle = Arc::new(Mutex::new(None));
        let handle_clone = Arc::clone(&handle);
        thread::spawn(move || {
            while let Ok(event) = orig_rx.recv() {
                match &event {
                    Step2UpdateExtractEvent::Progress { completed, total } => {
                        if let Ok(mut g) = handle_clone.lock() {
                            *g = Some((*completed, *total));
                        }
                    }
                    Step2UpdateExtractEvent::Finished(_) => {
                        if let Ok(mut g) = handle_clone.lock() {
                            *g = None;
                        }
                    }
                }
                if int_tx.send(event).is_err() {
                    break;
                }
            }
        });

        // Send a Progress; wait briefly for the forwarder to pump.
        orig_tx
            .send(Step2UpdateExtractEvent::Progress {
                completed: 3,
                total: 10,
            })
            .unwrap();
        // Poll the int_rx so the forwarder advances + then the handle.
        let received = int_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        match received {
            Step2UpdateExtractEvent::Progress { completed, total } => {
                assert_eq!(completed, 3);
                assert_eq!(total, 10);
            }
            _ => panic!("expected Progress"),
        }
        // The handle now reflects the snapshot. (After int_rx.recv
        // returned the forwarder has already locked + updated the
        // handle BEFORE the send; the recv-success ⇒ the update is
        // already in place.)
        let started = Instant::now();
        loop {
            if let Some((c, t)) = *handle.lock().unwrap() {
                assert_eq!((c, t), (3, 10));
                break;
            }
            if started.elapsed() > Duration::from_secs(2) {
                panic!("handle never snapshotted Progress (3,10)");
            }
            thread::sleep(Duration::from_millis(5));
        }

        // Send Finished; handle should clear. Construct the result
        // directly — Step2UpdateExtractResult / its fields are
        // `pub(crate)` (carve-out-#3 lib+bin same-crate reach).
        orig_tx
            .send(Step2UpdateExtractEvent::Finished(
                crate::app::app_step2_update_extract::Step2UpdateExtractResult {
                    extracted: Vec::new(),
                    failed: Vec::new(),
                },
            ))
            .unwrap();
        // Pull the Finished event from the intermediate so the forwarder
        // ran its branch.
        let _ = int_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        loop {
            if handle.lock().unwrap().is_none() {
                break;
            }
            if started.elapsed() > Duration::from_secs(2) {
                panic!("handle never cleared on Finished");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
}
