// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub struct InstallStep5Runtime<'a> {
    pub console_view: &'a mut Step5ConsoleViewState,
    pub terminal: Option<&'a mut EmbeddedTerminal>,
    pub terminal_error: Option<&'a str>,
}

pub(super) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut WizardState,
    runtime: InstallStep5Runtime<'_>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step5Action> {
    screen_title::render(
        ui,
        palette,
        "Installing modlist",
        Some("live install console"),
    );
    crate::ui::step5::page_step5::render(
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
    )
}
