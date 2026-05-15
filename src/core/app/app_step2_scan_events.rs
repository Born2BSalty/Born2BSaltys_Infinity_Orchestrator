// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

use crate::app::controller::util::sort_mods_alphabetically;
use crate::app::state::{Step2ModState, Step2ScanReport, WizardState};
use crate::app::step2_worker::Step2ScanEvent;

pub(super) fn poll_step2_scan_events(
    state: &mut WizardState,
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    let Some(rx) = step2_scan_rx.as_ref() else {
        return;
    };
    let mut reached_terminal = false;
    loop {
        match rx.try_recv() {
            Ok(Step2ScanEvent::Progress {
                current,
                total,
                name,
            }) => record_scan_progress(step2_progress_queue, current, total, name),
            Ok(Step2ScanEvent::Preview {
                bgee_mods,
                bg2ee_mods,
                total,
            }) => handle_scan_preview(state, bgee_mods, bg2ee_mods, total),
            Ok(Step2ScanEvent::Finished {
                bgee_mods,
                bg2ee_mods,
                report,
            }) => {
                handle_scan_finished(state, bgee_mods, bg2ee_mods, *report);
                clear_scan_worker(step2_scan_rx, step2_cancel, step2_progress_queue);
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Failed(message)) => {
                state.step2.scan_status = format!("Scan failed: {message}");
                clear_failed_scan(step2_scan_rx, step2_cancel, step2_progress_queue, state);
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Canceled) => {
                state.step2.scan_status = "Scan canceled".to_string();
                clear_failed_scan(step2_scan_rx, step2_cancel, step2_progress_queue, state);
                reached_terminal = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                state.step2.scan_status = "Scan worker disconnected".to_string();
                clear_failed_scan(step2_scan_rx, step2_cancel, step2_progress_queue, state);
                reached_terminal = true;
                break;
            }
        }
    }
    if !reached_terminal && let Some((current, total, name)) = step2_progress_queue.pop_front() {
        state.step2.scan_status = format!("{current}/{total}: {name}");
        state.step2.scan_progress_percent = current
            .checked_mul(100)
            .and_then(|value| value.checked_div(total))
            .map_or(0, |value| u8::try_from(value.min(100)).unwrap_or(100));
    }
}

fn record_scan_progress(
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
    current: usize,
    total: usize,
    name: String,
) {
    step2_progress_queue.push_back((current, total, name));
    if step2_progress_queue.len() > 512 {
        let _ = step2_progress_queue.pop_front();
    }
}

fn handle_scan_preview(
    state: &mut WizardState,
    mut primary_mods: Vec<Step2ModState>,
    mut secondary_mods: Vec<Step2ModState>,
    total: usize,
) {
    let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut primary_mods,
        &mut secondary_mods,
    );
    crate::app::mod_update_locks::apply_mod_update_locks(&mut primary_mods);
    crate::app::mod_update_locks::apply_mod_update_locks(&mut secondary_mods);
    let lock_load_error = crate::app::mod_update_locks::take_last_load_error();
    state.step2.bgee_mods = primary_mods;
    state.step2.bg2ee_mods = secondary_mods;
    state.step2.selected = None;
    state.step2.next_selection_order = 1;
    state.step2.scan_progress_percent = 0;
    state.step2.scan_status = scan_preview_status(total, lock_load_error, compat_error);
}

fn scan_preview_status(
    total: usize,
    lock_load_error: Option<String>,
    compat_error: Option<String>,
) -> String {
    match (lock_load_error, compat_error) {
        (Some(lock_err), Some(compat_err)) => {
            format!(
                "0/{total}: discovering mods... (update lock load failed: {lock_err}; compat rules load failed: {compat_err})"
            )
        }
        (Some(lock_err), None) => {
            format!("0/{total}: discovering mods... (update lock load failed: {lock_err})")
        }
        (None, Some(compat_err)) => {
            format!("0/{total}: discovering mods... (compat rules load failed: {compat_err})")
        }
        (None, None) => format!("0/{total}: discovering mods..."),
    }
}

fn handle_scan_finished(
    state: &mut WizardState,
    mut primary_mods: Vec<Step2ModState>,
    mut secondary_mods: Vec<Step2ModState>,
    report: Step2ScanReport,
) {
    sort_mods_alphabetically(&mut primary_mods);
    sort_mods_alphabetically(&mut secondary_mods);
    let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut primary_mods,
        &mut secondary_mods,
    );
    crate::app::mod_update_locks::apply_mod_update_locks(&mut primary_mods);
    crate::app::mod_update_locks::apply_mod_update_locks(&mut secondary_mods);
    let lock_load_error = crate::app::mod_update_locks::take_last_load_error();
    state.step2.bgee_mods = primary_mods;
    state.step2.bg2ee_mods = secondary_mods;
    let installed_refs_cleanup_error = prune_stale_installed_refs(state);
    state.step2.selected = None;
    state.step2.next_selection_order = 1;
    state.step2.scan_progress_percent = 100;
    state.step2.scan_status =
        scan_finished_status(lock_load_error, compat_error, installed_refs_cleanup_error);
    state.step2.last_scan_report = Some(report);
    state.step2.is_scanning = false;
}

fn scan_finished_status(
    lock_load_error: Option<String>,
    compat_error: Option<String>,
    installed_refs_cleanup_error: Option<String>,
) -> String {
    let mut scan_status = match (lock_load_error, compat_error) {
        (Some(lock_err), Some(compat_err)) => {
            format!(
                "Done (update lock load failed: {lock_err}; compat rules load failed: {compat_err})"
            )
        }
        (Some(lock_err), None) => format!("Done (update lock load failed: {lock_err})"),
        (None, Some(compat_err)) => format!("Done (compat rules load failed: {compat_err})"),
        (None, None) => "Done".to_string(),
    };
    if let Some(err) = installed_refs_cleanup_error {
        let _ = write!(scan_status, " (installed refs cleanup failed: {err})");
    }
    scan_status
}

fn clear_failed_scan(
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
    state: &mut WizardState,
) {
    state.step2.last_scan_report = None;
    state.step2.is_scanning = false;
    clear_scan_worker(step2_scan_rx, step2_cancel, step2_progress_queue);
}

fn clear_scan_worker(
    step2_scan_rx: &mut Option<Receiver<Step2ScanEvent>>,
    step2_cancel: &mut Option<Arc<AtomicBool>>,
    step2_progress_queue: &mut VecDeque<(usize, usize, String)>,
) {
    *step2_scan_rx = None;
    *step2_cancel = None;
    step2_progress_queue.clear();
}

fn prune_stale_installed_refs(state: &WizardState) -> Option<String> {
    let sources = crate::app::mod_downloads::load_mod_download_sources();
    let mut present_tp2s = BTreeSet::new();

    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        present_tp2s.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
            &mod_state.tp_file,
        ));

        for source in sources.find_sources(&mod_state.tp_file) {
            present_tp2s.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                &source.tp2,
            ));
            for alias in source.aliases {
                present_tp2s.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                    &alias,
                ));
            }
        }
    }

    crate::app::app_step2_update_source_refs::prune_installed_source_refs(present_tp2s)
        .err()
        .map(|err| err.to_string())
}
