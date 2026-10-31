// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;

pub fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str) {
    crate::ui::step3::content_step3::render(ui, state, dev_mode, exe_fingerprint);
    let palette = crate::ui::shared::redesign_tokens::ThemePalette::default();
    crate::ui::step3::installer_answers_panel_step3::render(ui.ctx(), state, palette);
}
