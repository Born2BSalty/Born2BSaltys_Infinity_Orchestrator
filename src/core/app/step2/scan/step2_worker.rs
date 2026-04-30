// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use crate::app::compat_conflict_parse::load_component_conflicts;
use crate::app::compat_dependency_parse::load_component_requirements;
use crate::app::scan::worker::scan_impl;
use crate::app::state::{Step1State, Step2ModState};

pub use crate::app::scan::Step2ScanEvent;

pub fn run_scan(step1: Step1State, sender: Sender<Step2ScanEvent>, cancel: Arc<AtomicBool>) {
    match scan_impl(&step1, &sender, &cancel) {
        Ok((bgee_mods, bg2ee_mods, report)) => {
            prewarm_import_compat_caches(&bgee_mods, &bg2ee_mods);
            let _ = sender.send(Step2ScanEvent::Finished {
                bgee_mods,
                bg2ee_mods,
                report: Box::new(report),
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

fn prewarm_import_compat_caches(bgee_mods: &[Step2ModState], bg2ee_mods: &[Step2ModState]) {
    let mut seen = HashSet::<String>::new();
    for mod_state in bgee_mods.iter().chain(bg2ee_mods.iter()) {
        let tp2_path = mod_state.tp2_path.trim();
        if tp2_path.is_empty() || !seen.insert(tp2_path.to_string()) {
            continue;
        }
        let _ = load_component_requirements(tp2_path);
        let _ = load_component_conflicts(tp2_path);
    }
}
