// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::app::state::WizardState;
use crate::app::step2_worker::Step2ScanEvent;

#[path = "app_step2_update_extract_archive.rs"]
mod archive;
#[path = "app_step2_update_extract_plan.rs"]
mod plan;

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateExtractResult {
    pub(crate) extracted: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) fn start_step2_update_extract(
    state: &mut WizardState,
    step2_update_extract_rx: &mut Option<Receiver<Step2UpdateExtractResult>>,
) {
    if state.step2.update_selected_extract_running {
        return;
    }
    let archive_dir = PathBuf::from(state.step1.mods_archive_folder.trim());
    if archive_dir.as_os_str().is_empty() {
        return;
    }

    let jobs = plan::build_extract_jobs(state, &archive_dir);
    if jobs.is_empty() {
        let failed = state.step2.update_selected_extract_failed_sources.len();
        if failed > 0 {
            state.step2.scan_status =
                format!("Extract updates finished: 0 updated, {failed} failed");
        }
        return;
    }

    let (tx, rx) = mpsc::channel::<Step2UpdateExtractResult>();
    *step2_update_extract_rx = Some(rx);
    state.step2.update_selected_extract_running = true;
    state.step2.scan_status = format!("Extracting updates: {}", jobs.len());

    thread::spawn(move || {
        let result = archive::extract_update_archives(&jobs);
        let _ = tx.send(result);
    });
}

pub(crate) fn poll_step2_update_extract(
    state: &mut WizardState,
    step2_update_extract_rx: &mut Option<Receiver<Step2UpdateExtractResult>>,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    let Some(rx) = step2_update_extract_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.step2.update_selected_extract_running = false;
            state.step2.scan_status = "Extract updates failed: worker disconnected".to_string();
            *step2_update_extract_rx = None;
            return;
        }
    };
    let Some(result) = result else {
        return;
    };

    *step2_update_extract_rx = None;
    state.step2.update_selected_extract_running = false;
    state.step2.update_selected_extracted_sources = result.extracted;
    state
        .step2
        .update_selected_extract_failed_sources
        .extend(result.failed);

    let extracted = state.step2.update_selected_extracted_sources.len();
    let failed = state.step2.update_selected_extract_failed_sources.len();
    if extracted > 0 {
        state.step1_mods_folder_has_tp2 = Some(true);
        state.step2.log_pending_downloads.clear();
        state.step2.scan_status = format!("Extracted {extracted} updates; rescanning Mods Folder");
        super::app_step2_scan::start_step2_scan(
            state,
            step2_scan_rx,
            step2_cancel,
            step2_progress_queue,
        );
    } else {
        state.step2.scan_status =
            format!("Extract updates finished: {extracted} updated, {failed} failed");
    }
}
