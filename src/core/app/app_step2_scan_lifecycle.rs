// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, mpsc};
use std::thread;

use crate::app::state::WizardState;
use crate::app::step2_worker::{Step2ScanEvent, run_scan};

pub(super) fn start_step2_scan(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    cancel_step2_scan(state, step2_cancel);
    let (tx, rx) = mpsc::channel::<Step2ScanEvent>();
    let cancel = Arc::new(AtomicBool::new(false));
    let step1 = state.step1.clone();
    let cancel_for_thread = Arc::clone(&cancel);
    state.step2.scan_status = "0/0".to_string();
    state.step2.scan_progress_percent = 0;
    state.step2.is_scanning = true;
    state.step2.last_scan_report = None;
    state.step2.log_pending_downloads.clear();
    state.step2.review_edit_bgee_log_applied = false;
    state.step2.review_edit_bg2ee_log_applied = false;
    state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
    step2_progress_queue.clear();
    *step2_scan_rx = Some(rx);
    *step2_cancel = Some(cancel);
    thread::spawn(move || run_scan(step1, tx, cancel_for_thread));
}

pub(super) fn cancel_step2_scan(state: &mut WizardState, step2_cancel: &Option<Arc<AtomicBool>>) {
    if let Some(cancel) = step2_cancel {
        cancel.store(true, Ordering::Relaxed);
        state.step2.scan_status = "Canceling...".to_string();
        state.step2.is_scanning = true;
    }
}
