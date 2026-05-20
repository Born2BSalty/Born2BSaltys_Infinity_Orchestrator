// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::app_step2_update_extract::{Step2UpdateExtractEvent, start_step2_update_extract};
use crate::app::state::WizardState;

pub(crate) fn start_extract_with_intercept(
    state: &mut WizardState,
    bio_extract_rx_slot: &mut Option<Receiver<Step2UpdateExtractEvent>>,
    progress_handle: &Arc<Mutex<Option<(usize, usize)>>>,
) -> bool {
    if let Ok(mut g) = progress_handle.lock() {
        *g = None;
    }

    let mut temp_slot: Option<Receiver<Step2UpdateExtractEvent>> = None;
    start_step2_update_extract(state, &mut temp_slot);
    let Some(orig_rx) = temp_slot else {
        *bio_extract_rx_slot = None;
        return false;
    };

    let (int_tx, int_rx) = mpsc::channel::<Step2UpdateExtractEvent>();
    let handle = Arc::clone(progress_handle);
    thread::spawn(move || {
        while let Ok(event) = orig_rx.recv() {
            match &event {
                Step2UpdateExtractEvent::Progress { completed, total } => {
                    if let Ok(mut g) = handle.lock() {
                        *g = Some((*completed, *total));
                    }
                }
                Step2UpdateExtractEvent::Finished(_) => {
                    if let Ok(mut g) = handle.lock() {
                        *g = None;
                    }
                }
            }
            if int_tx.send(event).is_err() {
                break;
            }
        }
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
        let mut state = WizardState::default();

        let mut slot: Option<Receiver<Step2UpdateExtractEvent>> = None;
        let handle = Arc::new(Mutex::new(None));
        let r = start_extract_with_intercept(&mut state, &mut slot, &handle);
        assert!(!r, "empty-jobs early-return ⇒ false");
        assert!(
            slot.is_none(),
            "no real spawn ⇒ orchestrator slot stays None"
        );

        assert!(handle.lock().unwrap().is_none());
    }

    #[test]
    fn intercept_returns_false_when_no_archive_to_extract() {
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

        orig_tx
            .send(Step2UpdateExtractEvent::Progress {
                completed: 3,
                total: 10,
            })
            .unwrap();

        let received = int_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        match received {
            Step2UpdateExtractEvent::Progress { completed, total } => {
                assert_eq!(completed, 3);
                assert_eq!(total, 10);
            }
            Step2UpdateExtractEvent::Finished(_) => panic!("expected Progress"),
        }

        let started = Instant::now();
        loop {
            let value = *handle.lock().unwrap();
            if let Some((c, t)) = value {
                assert_eq!((c, t), (3, 10));
                break;
            }
            assert!(
                started.elapsed() <= Duration::from_secs(2),
                "handle never snapshotted Progress (3,10)"
            );
            thread::sleep(Duration::from_millis(5));
        }

        orig_tx
            .send(Step2UpdateExtractEvent::Finished(
                crate::app::app_step2_update_extract::Step2UpdateExtractResult {
                    extracted: Vec::new(),
                    failed: Vec::new(),
                },
            ))
            .unwrap();

        let _ = int_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        loop {
            if handle.lock().unwrap().is_none() {
                break;
            }
            assert!(
                started.elapsed() <= Duration::from_secs(2),
                "handle never cleared on Finished"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
}
