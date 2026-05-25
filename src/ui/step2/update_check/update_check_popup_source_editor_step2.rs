// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::action_step2::Step2Action;

pub(super) fn render_source_editor_popup(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
    if !state.step2.mod_download_source_editor_open {
        return;
    }
    let mut open = state.step2.mod_download_source_editor_open;
    egui::Window::new("Edit Mod Download Source")
        .open(&mut open)
        .collapsible(true)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(620.0, 420.0))
        .min_width(420.0)
        .min_height(240.0)
        .show(ctx, |ui| {
            ui.set_min_size(ui.available_size());
            ui.label(format!(
                "{} ({})",
                state.step2.mod_download_source_editor_label,
                state.step2.mod_download_source_editor_source_id
            ));
            if let Some(err) = state.step2.mod_download_source_editor_error.as_ref() {
                ui.label(err);
            }
            ui.add(
                egui::TextEdit::multiline(&mut state.step2.mod_download_source_editor_text)
                    .desired_width(f32::INFINITY)
                    .desired_rows(16),
            );
            ui.horizontal_wrapped(|ui| {
                if redesign_btn(
                    ui,
                    palette,
                    "Save",
                    BtnOpts {
                        primary: true,
                        small: true,
                        ..Default::default()
                    },
                )
                .clicked()
                    && action.is_none()
                {
                    *action = Some(Step2Action::SaveModDownloadSourceEditor);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Cancel",
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    state.step2.mod_download_source_editor_open = false;
                    state.step2.mod_download_source_editor_error = None;
                }
            });
        });
    state.step2.mod_download_source_editor_open =
        open && state.step2.mod_download_source_editor_open;
}
