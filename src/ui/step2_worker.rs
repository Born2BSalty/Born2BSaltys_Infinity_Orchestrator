// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::ui::scan::worker::scan_impl;
use crate::ui::state::Step1State;

pub use crate::ui::scan::Step2ScanEvent;

pub fn run_scan(step1: Step1State, sender: Sender<Step2ScanEvent>, cancel: Arc<AtomicBool>) {
    match scan_impl(&step1, &sender, &cancel) {
        Ok((bgee_mods, bg2ee_mods)) => {
            let _ = sender.send(Step2ScanEvent::Finished {
                bgee_mods,
                bg2ee_mods,
            });
        }
        Err(err) if err == "canceled" => {
            let _ = sender.send(Step2ScanEvent::Canceled);
        }
        Err(err) => {
            let _ = sender.send(Step2ScanEvent::Failed(err));
        }
    }
}
