// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;

use crate::app::app_step1_github_oauth;
use crate::app::state::{Step1State, WizardState};
use crate::app::step2_worker::Step2ScanEvent;
use crate::app::step5::install_flow::PendingInstallStart;
use crate::app::step5::log_files::TargetPrepResult;
use crate::app::terminal::EmbeddedTerminal;
use crate::settings::store::SettingsStore;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

#[path = "app_bootstrap.rs"]
mod bootstrap;
#[path = "app_lifecycle.rs"]
mod lifecycle;
#[path = "app_methods.rs"]
mod methods;
#[path = "app_nav_ui.rs"]
pub mod nav_ui;
#[path = "app_step2_log.rs"]
mod step2_log;
#[path = "app_step2_router.rs"]
mod step2_router;
#[path = "app_update_loop.rs"]
mod update_loop;

pub(crate) use crate::app::app_nav as nav;
pub(crate) use crate::app::app_step2_update_check_worker as step2_update_check_worker;
pub(crate) use crate::app::app_step2_update_download as step2_update_download;
pub(crate) use crate::app::app_step2_update_extract as step2_update_extract;
pub(crate) use crate::app::app_step4_flow as step4_flow;

pub struct WizardApp {
    state: WizardState,
    settings_store: SettingsStore,
    last_saved_step1: Step1State,
    dev_mode: bool,
    exe_fingerprint: String,
    step2_scan_rx: Option<Receiver<Step2ScanEvent>>,
    step2_cancel: Option<Arc<AtomicBool>>,
    step2_progress_queue: VecDeque<(usize, usize, String)>,
    step1_github_auth_rx: Option<Receiver<app_step1_github_oauth::GitHubOAuthFlowResult>>,
    step2_update_check_rx: Option<Receiver<step2_update_check_worker::Step2UpdateCheckEvent>>,
    step2_update_download_rx: Option<Receiver<step2_update_download::Step2UpdateDownloadEvent>>,
    step2_update_extract_rx: Option<Receiver<step2_update_extract::Step2UpdateExtractEvent>>,
    step5_terminal: Option<EmbeddedTerminal>,
    step5_terminal_error: Option<String>,
    step5_console_view: Step5ConsoleViewState,
    step5_prep_rx: Option<Receiver<Result<TargetPrepResult, String>>>,
    step5_pending_start: Option<PendingInstallStart>,
}

impl Default for WizardApp {
    fn default() -> Self {
        Self::new(false)
    }
}

impl WizardApp {
    #[must_use]
    pub fn new(dev_mode: bool) -> Self {
        let init = bootstrap::initialize(dev_mode);
        let mut state = WizardState::with_step1(init.step1.clone());
        state.github_auth_login.clone_from(&init.github_auth_login);
        if let Some(startup_status) = init.startup_status.clone() {
            state.step2.scan_status = startup_status;
        }
        Self {
            state,
            settings_store: init.settings_store,
            last_saved_step1: init.step1,
            dev_mode,
            exe_fingerprint: init.exe_fingerprint,
            step2_scan_rx: None,
            step2_cancel: None,
            step2_progress_queue: VecDeque::new(),
            step1_github_auth_rx: None,
            step2_update_check_rx: None,
            step2_update_download_rx: None,
            step2_update_extract_rx: None,
            step5_terminal: None,
            step5_terminal_error: None,
            step5_console_view: Step5ConsoleViewState::default(),
            step5_prep_rx: None,
            step5_pending_start: None,
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
