// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render_console(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.label(egui::RichText::new("Console").strong().size(14.0));
        ui.add_space(6.0);
        let console_w = ui.available_width();
        let reserved_for_input = 56.0;
        let console_h = (ui.available_height() - reserved_for_input).max(220.0);
        if let Some(term) = terminal {
            term.set_show_important_only(state.step5.important_only);
            term.set_show_installed_only(state.step5.installed_only);
            term.set_auto_scroll(state.step5.auto_scroll);
            term.render(ui, egui::vec2(console_w, console_h));
        } else {
            ui.add_sized(
                [console_w, console_h],
                egui::Label::new(terminal_error.unwrap_or("Initializing terminal...")),
            );
        }
    });
}
