// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{ThemePalette, apply_redesign_bio_visuals};

pub use crate::ui::step2::action_step2::Step2Action;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
    palette: ThemePalette,
) -> Option<Step2Action> {
    let mut action = None;
    ui.scope(|ui| {
        apply_redesign_bio_visuals(ui, palette);
        action =
            crate::ui::step2::frame_step2::render(ui, state, dev_mode, exe_fingerprint, palette);
    });
    action
}
