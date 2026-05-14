// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_PAGE_PADDING_X_PX, REDESIGN_PAGE_PADDING_Y_PX, REDESIGN_WORKSPACE_HEADER_GAP_PX,
    REDESIGN_WORKSPACE_SECTION_GAP_PX, ThemePalette,
};
use crate::ui::workspace::state_workspace::WorkspaceViewState;
use crate::ui::workspace::step5::page_workspace_step5::{
    WorkspaceStep5Action, WorkspaceStep5Runtime, WorkspaceStep5SuccessInfo,
};
use crate::ui::workspace::step5::share_paste_code_dialog;
use crate::ui::workspace::{
    workspace_header, workspace_hint_line, workspace_nav_bar, workspace_progress_bar,
    workspace_step_router::{self, WorkspaceStepRouterRuntime},
};

pub struct WorkspaceRuntimeOptions<'a> {
    pub step5_runtime: Option<WorkspaceStep5Runtime<'a>>,
    pub disable_prev: bool,
    pub latest_share_code: Option<&'a str>,
    pub step5_success_info: WorkspaceStep5SuccessInfo,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut WorkspaceViewState,
    wizard_state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let _ = render_with_step5_runtime(
        ui,
        palette,
        state,
        wizard_state,
        WorkspaceRuntimeOptions {
            step5_runtime: None,
            disable_prev: false,
            latest_share_code: None,
            step5_success_info: WorkspaceStep5SuccessInfo {
                mod_count: 0,
                component_count: 0,
            },
        },
        dev_mode,
        exe_fingerprint,
    );
}

pub fn render_with_step5_runtime(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut WorkspaceViewState,
    wizard_state: &mut WizardState,
    runtime_options: WorkspaceRuntimeOptions<'_>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<WorkspaceStep5Action> {
    let mut step5_action = None;

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_PAGE_PADDING_X_PX as i8,
            REDESIGN_PAGE_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let share_enabled = state.install_complete;
            workspace_header::render(ui, palette, state, share_enabled);
            ui.add_space(REDESIGN_WORKSPACE_HEADER_GAP_PX);
            workspace_progress_bar::render(ui, palette, state);
            ui.add_space(REDESIGN_WORKSPACE_SECTION_GAP_PX);
            workspace_hint_line::render(ui, palette, state.current_step);
            ui.add_space(REDESIGN_WORKSPACE_HEADER_GAP_PX);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    step5_action = workspace_step_router::render(
                        ui,
                        palette,
                        state.current_step,
                        wizard_state,
                        WorkspaceStepRouterRuntime {
                            step5_runtime: runtime_options.step5_runtime,
                            step5_success_info: runtime_options.step5_success_info,
                        },
                        dev_mode,
                        exe_fingerprint,
                    );
                });

            ui.add_space(REDESIGN_WORKSPACE_HEADER_GAP_PX);
            workspace_nav_bar::render(ui, palette, state, runtime_options.disable_prev);
        });

    share_paste_code_dialog::render(
        ui.ctx(),
        palette,
        &mut state.share_paste_open,
        runtime_options.latest_share_code,
    );

    step5_action
}
