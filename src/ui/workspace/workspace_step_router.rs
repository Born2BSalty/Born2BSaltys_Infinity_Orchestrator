// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_text_faint, redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WorkspaceStep;
use crate::ui::workspace::step5::page_workspace_step5::{
    self, WorkspaceStep5Action, WorkspaceStep5Runtime, WorkspaceStep5SuccessInfo,
};

pub struct WorkspaceStepRouterRuntime<'a> {
    pub step5_runtime: Option<WorkspaceStep5Runtime<'a>>,
    pub step5_success_info: WorkspaceStep5SuccessInfo,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    step: WorkspaceStep,
    wizard_state: &mut WizardState,
    runtime: WorkspaceStepRouterRuntime<'_>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<WorkspaceStep5Action> {
    match step {
        WorkspaceStep::Step2 => {
            let _ignored_action = crate::ui::step2::page_step2::render(
                ui,
                wizard_state,
                dev_mode,
                exe_fingerprint,
                palette,
            );
            None
        }
        WorkspaceStep::Step3 => {
            crate::ui::step3::page_step3::render(
                ui,
                wizard_state,
                dev_mode,
                exe_fingerprint,
                palette,
            );
            None
        }
        WorkspaceStep::Step4 => {
            render_step4_placeholder(ui, palette);
            None
        }
        WorkspaceStep::Step5 => runtime.step5_runtime.and_then(|step5_runtime| {
            page_workspace_step5::render(
                ui,
                wizard_state,
                step5_runtime,
                runtime.step5_success_info,
                palette,
                dev_mode,
                exe_fingerprint,
            )
        }),
    }
}

fn render_step4_placeholder(ui: &mut egui::Ui, palette: ThemePalette) {
    redesign_box(ui, palette, Some("Step 4: Review"), |ui| {
        ui.label(
            egui::RichText::new("Step 4 review renderer lands after Step 2/3 embedding is proven.")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_primary(palette)),
        );
        ui.label(
            egui::RichText::new("No WeiDU log save or install behavior is wired in this batch.")
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_faint(palette)),
        );
    });
}
