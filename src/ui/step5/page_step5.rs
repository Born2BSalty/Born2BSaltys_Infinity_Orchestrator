// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<crate::ui::step5::action_step5::Step5Action> {
    crate::ui::step5::frame_step5::render(
        ui,
        state,
        console_view,
        terminal,
        terminal_error,
        dev_mode,
        exe_fingerprint,
    )
}
