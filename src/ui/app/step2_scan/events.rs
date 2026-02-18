// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
                crate::ui::step2::compat::apply_step2_compat_rules(
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
            }) => {
                sort_mods_alphabetically(&mut bgee_mods);
                sort_mods_alphabetically(&mut bg2ee_mods);
                crate::ui::step2::compat::apply_step2_compat_rules(
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
                app.state.step2.is_scanning = false;
                app.step2_scan_rx = None;
                app.step2_cancel = None;
                app.step2_progress_queue.clear();
                reached_terminal = true;
                break;
            }
            Ok(Step2ScanEvent::Canceled) => {
                app.state.step2.scan_status = "Scan canceled".to_string();
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
