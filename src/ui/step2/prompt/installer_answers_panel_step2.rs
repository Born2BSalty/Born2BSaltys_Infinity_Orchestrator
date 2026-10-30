// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{InstallerAnswerDraft, InstallerAnswersEditor, WizardState};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_text_muted};
use crate::ui::step2::installer_answers_step2::{
    current_target_available, editing_locked, save_current,
};

pub(crate) fn render(ctx: &egui::Context, state: &mut WizardState, palette: ThemePalette) -> bool {
    if state.step2.update_selected_popup_open
        || editing_locked(state)
        || !current_target_available(state)
    {
        state.step2.installer_answers = None;
        return false;
    }
    let Some(mut editor) = state.step2.installer_answers.take() else {
        return false;
    };
    let Some(target) = editor.targets.get(editor.active) else {
        return false;
    };
    if target.game_tab != state.step2.active_game_tab {
        return false;
    }
    let subtitle = format!("{} · {}", target.game_tab, target.title);
    let spec = DrawerSpec {
        id_salt: "step2_installer_answers",
        title: "Installer answers",
        subtitle: &subtitle,
        width: DrawerWidth::Wide,
        header_button: None,
        suppress_escape: false,
    };
    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| render_body(ui, &mut editor, palette),
        |ui| {
            let save = redesign_btn(
                ui,
                palette,
                "Save answers",
                BtnOpts {
                    primary: true,
                    ..Default::default()
                },
            )
            .clicked();
            ui.add_space(8.0);
            let defer = redesign_btn(
                ui,
                palette,
                "Answer during installation",
                BtnOpts::default(),
            )
            .clicked();
            (save, defer)
        },
    );
    if response.close_requested || response.footer.1 {
        return false;
    }
    state.step2.installer_answers = Some(editor);
    if response.footer.0 {
        match save_current(state) {
            Ok(()) => return true,
            Err(error) => {
                if let Some(editor) = state.step2.installer_answers.as_mut() {
                    editor.status = Some(error);
                }
            }
        }
    }
    false
}

fn render_body(ui: &mut egui::Ui, editor: &mut InstallerAnswersEditor, palette: ThemePalette) {
    if editor.targets.len() > 1 {
        let active_title = editor.targets[editor.active].title.clone();
        egui::ComboBox::from_id_salt("installer_answers_component")
            .selected_text(format!(
                "{} / {} — {}",
                editor.active + 1,
                editor.targets.len(),
                active_title
            ))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (index, target) in editor.targets.iter().enumerate() {
                    if ui
                        .selectable_value(&mut editor.active, index, &target.title)
                        .changed()
                    {
                        editor.status = None;
                    }
                }
            });
        ui.add_space(16.0);
    }
    let target = &mut editor.targets[editor.active];
    ui.label(
        egui::RichText::new(&target.title).family(egui::FontFamily::Name("poppins_medium".into())),
    );
    ui.add_space(10.0);
    ui.label("Enter the answers BIO should send when the installer asks for input. Answers are sent in order. Some questions depend on your game or earlier choices.");
    ui.add_space(18.0);
    if ui.available_width() >= 900.0 {
        let item_spacing = ui.spacing().item_spacing.x;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 40.0;
            ui.columns(2, |columns| {
                for column in columns.iter_mut() {
                    column.spacing_mut().item_spacing.x = item_spacing;
                }
                render_questions(&mut columns[0], &target.questions);
                render_answer_section(
                    &mut columns[1],
                    &mut target.answers,
                    &mut editor.status,
                    palette,
                );
            });
        });
    } else {
        render_answer_section(ui, &mut target.answers, &mut editor.status, palette);
        ui.add_space(20.0);
        ui.separator();
        ui.add_space(12.0);
        render_questions(ui, &target.questions);
    }
}

fn render_questions(ui: &mut egui::Ui, questions: &str) {
    ui.strong("Possible installer questions");
    ui.add_space(8.0);
    ui.add(egui::Label::new(questions).wrap());
}

fn render_answer_section(
    ui: &mut egui::Ui,
    answers: &mut Vec<InstallerAnswerDraft>,
    status: &mut Option<String>,
    palette: ThemePalette,
) {
    ui.strong("Answers to send, in order");
    ui.add_space(8.0);
    if render_answers(ui, answers) {
        *status = None;
    }
    if redesign_btn(ui, palette, "Add answer", BtnOpts::default()).clicked() {
        answers.push(InstallerAnswerDraft::default());
        *status = None;
    }
    ui.add_space(14.0);
    ui.label(egui::RichText::new("You can also answer during installation. BIO saves your answers after the component installs successfully.").color(redesign_text_muted(palette)));
    if let Some(status) = status {
        ui.add_space(12.0);
        ui.label(status.as_str());
    }
}

fn render_answers(ui: &mut egui::Ui, answers: &mut Vec<InstallerAnswerDraft>) -> bool {
    let mut changed = false;
    let mut remove = None;
    for (index, answer) in answers.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{}.", index + 1));
                let width = (ui.available_width() - 100.0).max(80.0);
                changed |= ui
                    .add_enabled(
                        !answer.blank_enter,
                        egui::TextEdit::singleline(&mut answer.text)
                            .desired_width(width)
                            .hint_text("Answer"),
                    )
                    .changed();
                if ui.button("Remove").clicked() {
                    remove = Some(index);
                }
            });
            changed |= ui
                .checkbox(&mut answer.blank_enter, "Send Enter without text")
                .changed();
            ui.add_space(8.0);
        });
    }
    if let Some(index) = remove {
        answers.remove(index);
        changed = true;
    }
    changed
}

#[cfg(test)]
#[path = "installer_answers_panel_step2_tests.rs"]
mod tests;
