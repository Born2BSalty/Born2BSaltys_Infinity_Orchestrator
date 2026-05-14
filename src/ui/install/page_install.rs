// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::install::stage_installing::{self, InstallStep5Runtime};
use crate::ui::install::state_install::{InstallAction, InstallScreenState, InstallStage};
use crate::ui::install::{stage_paste, stage_preview};
use crate::ui::shared::redesign_tokens::ThemePalette;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
    wizard_state: &mut WizardState,
    runtime: InstallStep5Runtime<'_>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<InstallAction> {
    match state.stage {
        InstallStage::Paste => {
            stage_paste::render(ui, palette, state);
            None
        }
        InstallStage::Preview => stage_preview::render(ui, palette, state),
        InstallStage::Installing => stage_installing::render(
            ui,
            palette,
            wizard_state,
            runtime,
            dev_mode,
            exe_fingerprint,
        )
        .map(InstallAction::Step5),
    }
}
