// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use std::time::Duration;

use super::super::WizardApp;

pub(super) fn request_if_needed(app: &WizardApp, ctx: &egui::Context) {
    if app.step2_scan_rx.is_some()
        || !app.step2_progress_queue.is_empty()
        || app
            .step5_terminal
            .as_ref()
            .map(|t| t.has_new_data())
            .unwrap_or(false)
        || app.state.step5.install_running
    {
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
