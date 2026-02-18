// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

use super::timefmt::{fmt_duration, now_unix_secs};

pub(super) fn render_process_runtime_inline(
    ui: &mut egui::Ui,
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
) {
    if let Some(term) = terminal {
        if state.step5.install_running {
            if let Some(pid) = term.process_id() {
                ui.label(egui::RichText::new(format!("| pid={pid}")).monospace().weak());
            }
        } else if let Some(code) = state.step5.last_exit_code {
            ui.label(egui::RichText::new(format!("| exit={code}")).monospace().weak());
        }
    }

    if state.step5.install_running
        && let Some(start) = state.step5.install_started_unix_secs
    {
        let elapsed = now_unix_secs().saturating_sub(start);
        ui.label(
            egui::RichText::new(format!("| elapsed={}", fmt_duration(elapsed)))
                .monospace()
                .weak(),
        );
    } else if let Some(last_runtime) = state.step5.last_runtime_secs {
        ui.label(
            egui::RichText::new(format!("| last={}", fmt_duration(last_runtime)))
                .monospace()
                .weak(),
        );
    }
}

pub(super) fn render_error_copy(
    ui: &mut egui::Ui,
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
) {
    if state.step5.last_install_failed
        && let Some(term) = terminal
        && ui
            .button("Copy error block")
            .on_hover_text("Copy recent error/fatal lines from console output.")
            .clicked()
    {
        ui.ctx().copy_text(term.extract_error_block());
    }
}
