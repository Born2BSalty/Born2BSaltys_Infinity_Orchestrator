// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::settings::store::SettingsStore;
use crate::ui::state::{Step1State, WizardState};
use crate::ui::step2_worker::Step2ScanEvent;
use crate::ui::terminal::EmbeddedTerminal;

#[path = "app_bootstrap.rs"]
mod bootstrap;
#[path = "app_nav.rs"]
mod nav;
#[path = "app_nav_ui.rs"]
mod nav_ui;
#[path = "app_lifecycle.rs"]
mod lifecycle;
#[path = "app_methods.rs"]
mod methods;
#[path = "app_step2_log.rs"]
mod step2_log;
#[path = "app_step2_router.rs"]
mod step2_router;
#[path = "app_step2_sync_flow.rs"]
mod step2_sync_flow;
#[path = "app_step2_scan.rs"]
mod step2_scan;
#[path = "app_step3_sync_flow.rs"]
mod step3_sync_flow;
#[path = "app_step5_flow.rs"]
pub mod step5_flow;
#[path = "app_update_loop.rs"]
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
        let _ = crate::ui::step2::service_compat_defaults_step2::ensure_compat_rules_files();
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
