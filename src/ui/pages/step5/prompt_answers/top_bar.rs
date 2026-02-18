// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use rfd::FileDialog;

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState, terminal: Option<&EmbeddedTerminal>) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Capture Current Prompt"))
            .on_hover_text("Create/update entry for currently detected prompt key.")
            .clicked()
            && let Some(term) = terminal
            && let Some(prompt) = term.current_prompt_info()
        {
            prompt_memory::ensure_prompt_entry(&prompt.key, &prompt.preview_line);
            state.step5.last_status_text = format!("Captured prompt key: {}", prompt.key);
        }

        if ui.button("Import JSON").clicked()
            && let Some(path) = FileDialog::new().add_filter("JSON", &["json"]).pick_file()
        {
            match prompt_memory::import_json(&path) {
                Ok(count) => {
                    state.step5.last_status_text =
                        format!("Imported {count} prompt entries from {}", path.display());
                }
                Err(err) => {
                    state.step5.last_status_text = format!("Import prompt answers failed: {err}");
                }
            }
        }

        if ui.button("Export JSON").clicked()
            && let Some(path) = FileDialog::new()
                .set_file_name("prompt_answers_export.json")
                .add_filter("JSON", &["json"])
                .save_file()
        {
            match prompt_memory::export_json(&path) {
                Ok(count) => {
                    state.step5.last_status_text =
                        format!("Exported {count} prompt entries to {}", path.display());
                }
                Err(err) => {
                    state.step5.last_status_text = format!("Export prompt answers failed: {err}");
                }
            }
        }
    });
}
