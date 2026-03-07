// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<crate::ui::step4::action_step4::Step4Action> {
    crate::ui::step4::frame_step4::render(ui, state, dev_mode, exe_fingerprint)
}
