// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::orchestrator::widgets::{BtnOpts, clipboard, redesign_btn};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::service_diagnostics_support_step5::{
    open_console_logs_folder, open_last_log_file, save_console_log,
};

pub(crate) fn render_actions_menu(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    palette: ThemePalette,
) {
    let btn = redesign_btn(
        ui,
        palette,
        "Actions",
        BtnOpts {
            small: true,
            ..Default::default()
        },
    );
    let popup_id = ui.make_persistent_id("step5_actions_menu");
    if btn.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }
    egui::popup::popup_below_widget(
        ui,
        popup_id,
        &btn,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(180.0);
            if ui
                .add_enabled(terminal.is_some(), egui::Button::new("Copy Console"))
                .clicked()
            {
                if let Some(term) = terminal.as_ref() {
                    clipboard::copy(ui.ctx(), term.console_text());
                }
                ui.memory_mut(egui::Memory::close_popup);
            }
            if ui
                .add_enabled(terminal.is_some(), egui::Button::new("Save Console Log"))
                .clicked()
            {
                if let Some(term) = terminal.as_ref() {
                    match save_console_log(state, &term.console_text()) {
                        Ok(path) => {
                            state.step5.last_status_text =
                                format!("Saved console log: {}", path.display());
                        }
                        Err(err) => {
                            state.step5.last_status_text =
                                format!("Save console log failed: {err}");
                        }
                    }
                }
                ui.memory_mut(egui::Memory::close_popup);
            }
            if ui
                .add_enabled(terminal.is_some(), egui::Button::new("Open Logs Folder"))
                .clicked()
            {
                if let Err(err) = open_console_logs_folder() {
                    state.step5.last_status_text = format!("Open logs folder failed: {err}");
                }
                ui.memory_mut(egui::Memory::close_popup);
            }
            if ui
                .add_enabled(terminal.is_some(), egui::Button::new("Clear Console"))
                .clicked()
            {
                if let Some(term) = terminal.as_mut() {
                    term.clear_console();
                }
                ui.memory_mut(egui::Memory::close_popup);
            }
            if state.step5.last_install_failed && ui.button("Open last log file").clicked() {
                let _ = open_last_log_file(&state.step1);
                ui.memory_mut(egui::Memory::close_popup);
            }
        },
    );
}
