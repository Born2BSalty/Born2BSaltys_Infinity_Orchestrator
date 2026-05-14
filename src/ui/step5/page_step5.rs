// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::redesign_tokens::{ThemePalette, apply_redesign_bio_visuals};
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub struct Step5RenderRuntime<'a> {
    pub console_view: &'a mut Step5ConsoleViewState,
    pub terminal: Option<&'a mut EmbeddedTerminal>,
    pub terminal_error: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub struct Step5RenderOptions<'a> {
    pub dev_mode: bool,
    pub exe_fingerprint: &'a str,
    pub palette: ThemePalette,
}

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: Step5RenderRuntime<'_>,
    options: Step5RenderOptions<'_>,
) -> Option<crate::ui::step5::action_step5::Step5Action> {
    let mut action = None;
    ui.scope(|ui| {
        apply_redesign_bio_visuals(ui, options.palette);
        action = crate::ui::step5::frame_step5::render(ui, state, runtime, options);
    });
    action
}
