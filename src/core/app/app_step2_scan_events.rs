// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

use crate::app::controller::util::sort_mods_alphabetically;
use crate::app::state::WizardState;
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
            }) => {
                step2_progress_queue.push_back((current, total, name));
                if step2_progress_queue.len() > 512 {
                    let _ = step2_progress_queue.pop_front();
                }
            }
            Ok(Step2ScanEvent::Preview {
                mut bgee_mods,
                mut bg2ee_mods,
                total,
            }) => {
                let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
                    &state.step1,
                    &mut bgee_mods,
                    &mut bg2ee_mods,
                );
                crate::app::mod_update_locks::apply_mod_update_locks(&mut bgee_mods);
                crate::app::mod_update_locks::apply_mod_update_locks(&mut bg2ee_mods);
                let lock_load_error = crate::app::mod_update_locks::take_last_load_error();
                state.step2.bgee_mods = bgee_mods;
                state.step2.bg2ee_mods = bg2ee_mods;
                state.step2.selected = None;
                state.step2.next_selection_order = 1;
                state.step2.scan_progress_percent = 0;
                state.step2.scan_status = match (lock_load_error, compat_error) {
                    (Some(lock_err), Some(compat_err)) => {
                        format!(
                            "0/{total}: discovering mods... (update lock load failed: {lock_err}; compat rules load failed: {compat_err})"
                        )
                    }
                    (Some(lock_err), None) => {
                        format!(
                            "0/{total}: discovering mods... (update lock load failed: {lock_err})"
                        )
                    }
                    (None, Some(compat_err)) => {
                        format!(
                            "0/{total}: discovering mods... (compat rules load failed: {compat_err})"
                        )
                    }
                    (None, None) => format!("0/{total}: discovering mods..."),
                };
            }
            Ok(Step2ScanEvent::Finished {
                mut bgee_mods,
                mut bg2ee_mods,
                report,
            }) => {
                sort_mods_alphabetically(&mut bgee_mods);
                sort_mods_alphabetically(&mut bg2ee_mods);
                let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
                    &state.step1,
                    &mut bgee_mods,
                    &mut bg2ee_mods,
                );
                crate::app::mod_update_locks::apply_mod_update_locks(&mut bgee_mods);
                crate::app::mod_update_locks::apply_mod_update_locks(&mut bg2ee_mods);
                let lock_load_error = crate::app::mod_update_locks::take_last_load_error();
                state.step2.bgee_mods = bgee_mods;
                state.step2.bg2ee_mods = bg2ee_mods;
                let installed_refs_cleanup_error = prune_stale_installed_refs(state);
                state.step2.selected = None;
                state.step2.next_selection_order = 1;
                state.step2.scan_progress_percent = 100;
                let mut scan_status = match (lock_load_error, compat_error) {
                    (Some(lock_err), Some(compat_err)) => {
                        format!(
                            "Done (update lock load failed: {lock_err}; compat rules load failed: {compat_err})"
                        )
                    }
                    (Some(lock_err), None) => format!("Done (update lock load failed: {lock_err})"),
                    (None, Some(compat_err)) => {
                        format!("Done (compat rules load failed: {compat_err})")
                    }
                    (None, None) => "Done".to_string(),
                };
                if let Some(err) = installed_refs_cleanup_error {
                    scan_status.push_str(&format!(" (installed refs cleanup failed: {err})"));
                }
                state.step2.scan_status = scan_status;
                state.step2.last_scan_report = Some(*report);
                state.step2.is_scanning = false;
                *step2_scan_rx = None;
                *step2_cancel = None;
                step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Failed(message)) => {
                state.step2.scan_status = format!("Scan failed: {message}");
                state.step2.last_scan_report = None;
                state.step2.is_scanning = false;
                *step2_scan_rx = None;
                *step2_cancel = None;
                step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Canceled) => {
                state.step2.scan_status = "Scan canceled".to_string();
                state.step2.last_scan_report = None;
                state.step2.is_scanning = false;
                *step2_scan_rx = None;
                *step2_cancel = None;
                step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                state.step2.scan_status = "Scan worker disconnected".to_string();
                state.step2.last_scan_report = None;
                state.step2.is_scanning = false;
                *step2_scan_rx = None;
                *step2_cancel = None;
                step2_progress_queue.clear();
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
            .map(|value| value.min(100) as u8)
            .unwrap_or(0);
    }
}

fn prune_stale_installed_refs(state: &WizardState) -> Option<String> {
    let present_tp2s = state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .map(|mod_state| mod_state.tp_file.as_str());

    crate::app::app_step2_update_source_refs::prune_installed_source_refs(present_tp2s)
        .err()
        .map(|err| err.to_string())
}
