// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::terminal::EmbeddedTerminal;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step5Action> {
    crate::ui::step5::content_step5::render(
        ui,
        state,
        terminal,
        terminal_error,
        dev_mode,
        exe_fingerprint,
    )
}
