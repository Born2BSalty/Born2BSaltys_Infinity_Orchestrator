// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;

use crate::ui::step2_worker::{Step2ScanEvent, run_scan};

use super::super::WizardApp;

pub(in crate::ui::app) fn start_step2_scan(app: &mut WizardApp) {
    cancel_step2_scan(app);
    let (tx, rx) = mpsc::channel::<Step2ScanEvent>();
    let cancel = Arc::new(AtomicBool::new(false));
    let step1 = app.state.step1.clone();
    let cancel_for_thread = Arc::clone(&cancel);
    app.state.step2.scan_status = "0/0".to_string();
    app.state.step2.scan_progress_percent = 0;
    app.state.step2.is_scanning = true;
    app.state.step2.collapse_epoch = app.state.step2.collapse_epoch.saturating_add(1);
    app.step2_progress_queue.clear();
    app.step2_scan_rx = Some(rx);
    app.step2_cancel = Some(cancel);
    thread::spawn(move || run_scan(step1, tx, cancel_for_thread));
}

pub(in crate::ui::app) fn cancel_step2_scan(app: &mut WizardApp) {
    if let Some(cancel) = &app.step2_cancel {
        cancel.store(true, Ordering::Relaxed);
        app.state.step2.scan_status = "Canceling...".to_string();
        app.state.step2.is_scanning = true;
    }
}
