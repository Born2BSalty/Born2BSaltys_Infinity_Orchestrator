// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::compat::CompatValidator;
use crate::settings::store::SettingsStore;
use crate::ui::state::{Step1State, WizardState};
use crate::ui::step2_worker::Step2ScanEvent;
use crate::ui::terminal::EmbeddedTerminal;

mod bootstrap;
pub mod compat_flow;
mod nav;
mod nav_ui;
mod lifecycle;
mod methods;
mod step2_log;
mod step2_compat_overlay;
mod step2_router;
mod step2_scan;
mod step3_sync_flow;
mod tp2_metadata;
pub mod step5_flow;
mod update_loop;

pub struct WizardApp {
    state: WizardState,
    settings_store: SettingsStore,
    last_saved_step1: Step1State,
    dev_mode: bool,
    exe_fingerprint: String,
    step2_scan_rx: Option<Receiver<Step2ScanEvent>>,
    step2_cancel: Option<Arc<AtomicBool>>,
    step2_progress_queue: VecDeque<(usize, usize, String)>,
    step5_terminal: Option<EmbeddedTerminal>,
    step5_terminal_error: Option<String>,
    compat_validator: CompatValidator,
    last_step2_sync_signature: Option<String>,
}

impl Default for WizardApp {
    fn default() -> Self {
        Self::new(false)
    }
}

impl WizardApp {
    pub fn new(dev_mode: bool) -> Self {
        let init = bootstrap::initialize(dev_mode);
        let _ = crate::ui::step2::compat::create_default_step2_compat_rules_file();
        Self {
            state: WizardState::with_step1(init.step1.clone()),
            settings_store: init.settings_store,
            last_saved_step1: init.step1,
            dev_mode,
            exe_fingerprint: init.exe_fingerprint,
            step2_scan_rx: None,
            step2_cancel: None,
            step2_progress_queue: VecDeque::new(),
            step5_terminal: None,
            step5_terminal_error: None,
            compat_validator: CompatValidator::new(),
            last_step2_sync_signature: None,
        }
    }

}

impl Drop for WizardApp {
    fn drop(&mut self) {
        self.save_settings_best_effort();
    }
}

impl eframe::App for WizardApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        update_loop::run(self, ctx, frame);
    }

}
