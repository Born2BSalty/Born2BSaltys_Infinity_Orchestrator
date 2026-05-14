// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HOME_CARD_LIST_GAP_PX,
    REDESIGN_HOME_GAME_LINE_GAP_PX, REDESIGN_LABEL_FONT_SIZE_PX,
    REDESIGN_SETTINGS_BOX_PADDING_X_PX, REDESIGN_SETTINGS_BOX_PADDING_Y_PX, ThemePalette,
    redesign_border_strong, redesign_success, redesign_text_muted, redesign_text_on_accent,
};
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::state_step5::Step5ConsoleViewState;
use crate::ui::workspace::step5::post_install_actions::{self, PostInstallAction};

pub struct WorkspaceStep5Runtime<'a> {
    pub console_view: &'a mut Step5ConsoleViewState,
    pub terminal: Option<&'a mut EmbeddedTerminal>,
    pub terminal_error: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceStep5SuccessInfo {
    pub mod_count: usize,
    pub component_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceStep5Action {
    Step5(Step5Action),
    ReturnToHome,
    OpenInstallFolder,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: WorkspaceStep5Runtime<'_>,
    success_info: WorkspaceStep5SuccessInfo,
    palette: ThemePalette,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<WorkspaceStep5Action> {
    let mut action = None;
    if clean_install_success(state) {
        render_success_banner(ui, palette, state, success_info);
        ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
        if let Some(post_action) = post_install_actions::render(ui, palette) {
            action = Some(match post_action {
                PostInstallAction::ReturnToHome => WorkspaceStep5Action::ReturnToHome,
                PostInstallAction::OpenInstallFolder => WorkspaceStep5Action::OpenInstallFolder,
            });
        }
        ui.add_space(REDESIGN_HOME_CARD_LIST_GAP_PX);
    }

    let step5_action = crate::ui::step5::page_step5::render(
        ui,
        state,
        crate::ui::step5::page_step5::Step5RenderRuntime {
            console_view: runtime.console_view,
            terminal: runtime.terminal,
            terminal_error: runtime.terminal_error,
        },
        crate::ui::step5::page_step5::Step5RenderOptions {
            dev_mode,
            exe_fingerprint,
            palette,
        },
    );
    action.or(step5_action.map(WorkspaceStep5Action::Step5))
}

fn clean_install_success(state: &WizardState) -> bool {
    !state.step5.install_running
        && state.step5.last_exit_code == Some(0)
        && !state.step5.last_install_failed
}

fn render_success_banner(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &WizardState,
    success_info: WorkspaceStep5SuccessInfo,
) {
    egui::Frame::NONE
        .fill(redesign_success(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_SETTINGS_BOX_PADDING_X_PX as i8,
            REDESIGN_SETTINGS_BOX_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = REDESIGN_HOME_GAME_LINE_GAP_PX;
                ui.label(
                    egui::RichText::new("Installed")
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .strong()
                        .color(redesign_text_on_accent(palette)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "· {} mods · {} components · no errors",
                        success_info.mod_count, success_info.component_count
                    ))
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_on_accent(palette)),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "· ran {} · finished now",
                        install_duration(state)
                    ))
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_muted(palette)),
                );
            });
        });
}

fn install_duration(state: &WizardState) -> String {
    let secs = state.step5.last_runtime_secs.unwrap_or_default();
    let minutes = secs / 60;
    let seconds = secs % 60;
    if minutes < 60 {
        format!("{minutes}:{seconds:02}")
    } else {
        let hours = minutes / 60;
        let minutes = minutes % 60;
        format!("{hours}:{minutes:02}:{seconds:02}")
    }
}
