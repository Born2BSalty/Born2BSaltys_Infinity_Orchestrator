// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn render_button(ui: &mut egui::Ui, state: &mut WizardState) {
    if ui
        .button("Prompt Answers")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_PROMPT_ANSWERS)
        .clicked()
    {
        state.step5.prompt_answers_window_open = true;
    }
}

pub(crate) fn render_window(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&EmbeddedTerminal>,
) {
    if !state.step5.prompt_answers_window_open {
        return;
    }

    let mut open = state.step5.prompt_answers_window_open;
    egui::Window::new("Saved Prompt Answers")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(980.0, 430.0))
        .show(ui.ctx(), |ui| {
            top_bar::render(ui, state, terminal);
            ui.separator();
            table::render(ui);
        });

    state.step5.prompt_answers_window_open = open;
}

mod top_bar {
use eframe::egui;
use rfd::FileDialog;

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState, terminal: Option<&EmbeddedTerminal>) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(terminal.is_some(), egui::Button::new("Capture Current Prompt"))
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_CAPTURE_PROMPT)
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
}

mod table {
use eframe::egui;

use crate::ui::step5::prompt_memory;
use crate::ui::step5::service_step5::group_prompt_entries;

pub(super) fn render(ui: &mut egui::Ui) {
    let entries = prompt_memory::list_entries();
    if entries.is_empty() {
        ui.label(crate::ui::shared::typography_global::weak("No saved prompt answers yet."));
        return;
    }

    let groups = group_prompt_entries(entries);
    egui::ScrollArea::vertical()
        .id_salt("step5_prompt_answers_scroll")
        .show(ui, |ui| {
            for group in groups {
                egui::CollapsingHeader::new(format!("{} ({})", group.label, group.items.len()))
                    .id_salt(("step5_prompt_answers_group", group.label.as_str()))
                    .default_open(false)
                    .show(ui, |ui| {
                        egui::Grid::new(("step5_prompt_answers_grid", group.label.as_str()))
                            .num_columns(5)
                            .striped(true)
                            .spacing([10.0, 8.0])
                            .show(ui, |ui| {
                                ui.strong("Auto");
                                ui.strong("Alias");
                                ui.strong("Preview");
                                ui.strong("Answer");
                                ui.strong("Action");
                                ui.end_row();

                                for (key, entry) in group.items {
                                    rows::render_entry_row(ui, &key, entry);
                                }
                            });
                    });
                ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_SM);
            }
        });
}

mod rows {
use eframe::egui;

use crate::ui::step5::prompt_memory;

pub(super) fn render_entry_row(ui: &mut egui::Ui, key: &str, entry: prompt_memory::PromptAnswerEntry) {
    let mut enabled = entry.enabled;
    if ui.checkbox(&mut enabled, "").changed() {
        prompt_memory::set_enabled(key, enabled);
    }

    let mut alias = entry.alias.clone();
    let alias_resp = ui.add_sized(
        [160.0, 24.0],
        egui::TextEdit::singleline(&mut alias).hint_text("optional alias"),
    );
    if alias_resp.changed() {
        prompt_memory::set_alias(key, &alias);
    }

    let preview = if entry.preview.trim().is_empty() {
        "(no preview)".to_string()
    } else {
        entry.preview.clone()
    };
    ui.label(
        crate::ui::shared::typography_global::weak(preview.clone())
            .text_style(egui::TextStyle::Small),
    )
    .on_hover_text(format!("{preview}\n\nKey: {key}"));

    let mut answer = entry.answer.clone();
    let answer_resp = ui.add_sized([170.0, 24.0], egui::TextEdit::singleline(&mut answer));
    if answer_resp.changed() {
        prompt_memory::set_answer(key, &answer);
    }

    if ui.button("Delete").clicked() {
        prompt_memory::delete_entry(key);
    }
    ui.end_row();
}
}
}
