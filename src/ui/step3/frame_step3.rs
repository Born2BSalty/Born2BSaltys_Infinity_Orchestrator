// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::ThemePalette;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
    palette: ThemePalette,
) {
    crate::ui::step3::content_step3::render(ui, state, dev_mode, exe_fingerprint, palette)
}
