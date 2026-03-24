// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod events {
use std::sync::mpsc::TryRecvError;

use crate::ui::controller::util::sort_mods_alphabetically;
use crate::ui::step2_worker::Step2ScanEvent;

use super::super::WizardApp;

pub(in crate::ui::app) fn poll_step2_scan_events(app: &mut WizardApp) {
    let Some(rx) = app.step2_scan_rx.as_ref() else {
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
                app.step2_progress_queue.push_back((current, total, name));
                if app.step2_progress_queue.len() > 512 {
                    let _ = app.step2_progress_queue.pop_front();
                }
            }
            Ok(Step2ScanEvent::Preview {
                mut bgee_mods,
                mut bg2ee_mods,
                total,
            }) => {
                crate::ui::step2::service_step2::apply_compat_rules(
                    &app.state.step1,
                    &mut bgee_mods,
                    &mut bg2ee_mods,
                );
                app.state.step2.bgee_mods = bgee_mods;
                app.state.step2.bg2ee_mods = bg2ee_mods;
                app.state.step2.selected = None;
                app.state.step2.next_selection_order = 1;
                app.state.step2.scan_progress_percent = 0;
                app.state.step2.scan_status = format!("0/{total}: discovering mods...");
            }
            Ok(Step2ScanEvent::Finished {
                mut bgee_mods,
                mut bg2ee_mods,
                report,
            }) => {
                sort_mods_alphabetically(&mut bgee_mods);
                sort_mods_alphabetically(&mut bg2ee_mods);
                crate::ui::step2::service_step2::apply_compat_rules(
                    &app.state.step1,
                    &mut bgee_mods,
                    &mut bg2ee_mods,
                );
                app.state.step2.bgee_mods = bgee_mods;
                app.state.step2.bg2ee_mods = bg2ee_mods;
                app.state.step2.selected = None;
                app.state.step2.next_selection_order = 1;
                app.state.step2.scan_progress_percent = 100;
                app.state.step2.scan_status = "Done".to_string();
                app.state.step2.last_scan_report = Some(*report);
                super::super::tp2_metadata::refresh_validator_tp2_metadata(app);
                app.revalidate_compat_step2_checked_order();
                app.state.step2.is_scanning = false;
                app.step2_scan_rx = None;
                app.step2_cancel = None;
                app.step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Failed(message)) => {
                app.state.step2.scan_status = format!("Scan failed: {message}");
                app.state.step2.last_scan_report = None;
                app.state.step2.is_scanning = false;
                app.step2_scan_rx = None;
                app.step2_cancel = None;
                app.step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Canceled) => {
                app.state.step2.scan_status = "Scan canceled".to_string();
                app.state.step2.last_scan_report = None;
                app.state.step2.is_scanning = false;
                app.step2_scan_rx = None;
                app.step2_cancel = None;
                app.step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                app.state.step2.scan_status = "Scan worker disconnected".to_string();
                app.state.step2.last_scan_report = None;
                app.state.step2.is_scanning = false;
                app.step2_scan_rx = None;
                app.step2_cancel = None;
                app.step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
        }
    }
    if !reached_terminal
        && let Some((current, total, name)) = app.step2_progress_queue.pop_front()
    {
        app.state.step2.scan_status = format!("{current}/{total}: {name}");
        app.state.step2.scan_progress_percent = if total == 0 {
            0
        } else {
            ((current * 100) / total).min(100) as u8
        };
    }
}
}
mod lifecycle {
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
    app.state.step2.last_scan_report = None;
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
}

pub(super) use events::poll_step2_scan_events;
pub(super) use lifecycle::cancel_step2_scan;
pub(super) use lifecycle::start_step2_scan;
