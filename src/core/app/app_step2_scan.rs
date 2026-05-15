// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;

#[path = "app_step2_scan_events.rs"]
mod events;
#[path = "app_step2_scan_lifecycle.rs"]
mod lifecycle;

pub(crate) fn poll_step2_scan_events(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    events::poll_step2_scan_events(state, step2_scan_rx, step2_cancel, step2_progress_queue);
}

pub(crate) fn cancel_step2_scan(state: &mut WizardState, step2_cancel: Option<&Arc<AtomicBool>>) {
    lifecycle::cancel_step2_scan(state, step2_cancel);
}

pub(crate) fn start_step2_scan(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    lifecycle::start_step2_scan(state, step2_scan_rx, step2_cancel, step2_progress_queue);
}
