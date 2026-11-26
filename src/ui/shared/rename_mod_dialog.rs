// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_error, redesign_input_bg, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RenameTarget {
    #[default]
    UserFile,
    ActiveModlist,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RenameModState {
    pub(crate) tp2: String,
    pub(crate) buffer: String,
    pub(crate) target: RenameTarget,
    pub(crate) focus_pending: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RenameOutcome {
    #[default]
    Pending,
    Saved,
    Cancelled,
}

const MAX_WIDTH_PX: f32 = 420.0;
const MAX_NAME_CHARS: usize = 80;

#[must_use]
pub(crate) fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut RenameModState,
) -> RenameOutcome {
    let mut outcome = RenameOutcome::Pending;
    let dialog_id = egui::Id::new("orchestrator_rename_mod_dialog");

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new("Rename mod")
        .id(dialog_id)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);
            render_header(ui, palette, &state.tp2);
            ui.add_space(12.0);
            let enter_pressed = render_name_field(ui, palette, dialog_id, state);
            if let Some(error) = &state.error {
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(error)
                        .size(12.0)
                        .color(redesign_error(palette)),
                );
            }
            ui.add_space(14.0);
            if let Some(footer_outcome) = render_footer(ui, palette, enter_pressed) {
                outcome = footer_outcome;
            }
        });

    outcome
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette, tp2: &str) {
    ui.label(
        egui::RichText::new("Rename mod")
            .size(15.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(tp2)
            .size(12.0)
            .color(redesign_text_muted(palette)),
    );
}

fn render_name_field(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    dialog_id: egui::Id,
    state: &mut RenameModState,
) -> bool {
    let response = redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(&mut state.buffer)
                .id(dialog_id.with("name_field"))
                .char_limit(MAX_NAME_CHARS)
                .hint_text("leave empty to use the catalog name")
                .font(egui::FontId::new(
                    14.0,
                    egui::FontFamily::Name("poppins_light".into()),
                ))
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(egui::Margin::symmetric(8, 4)),
            margin: egui::Margin::symmetric(8, 4),
            size: egui::vec2(ui.available_width(), 30.0),
            border: None,
        },
    );
    if state.focus_pending {
        response.request_focus();
        state.focus_pending = false;
    }
    (response.has_focus() || response.lost_focus()) && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

fn render_footer(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    enter_pressed: bool,
) -> Option<RenameOutcome> {
    let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let mut outcome = None;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let save_clicked = redesign_btn(
                ui,
                palette,
                "Save",
                BtnOpts {
                    small: true,
                    primary: true,
                    ..Default::default()
                },
            )
            .clicked();
            let cancel_clicked = redesign_btn(
                ui,
                palette,
                "Cancel",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked();
            if save_clicked || enter_pressed {
                outcome = Some(RenameOutcome::Saved);
            } else if cancel_clicked || escape_pressed {
                outcome = Some(RenameOutcome::Cancelled);
            }
        },
    );
    outcome
}

#[cfg(test)]
#[path = "rename_mod_dialog_tests.rs"]
mod tests;
