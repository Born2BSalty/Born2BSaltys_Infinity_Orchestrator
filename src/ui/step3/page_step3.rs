// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str) {
    crate::ui::step3::frame_step3::render(ui, state, dev_mode, exe_fingerprint)
}
