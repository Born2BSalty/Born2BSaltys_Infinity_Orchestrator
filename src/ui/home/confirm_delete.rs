// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::registry::model::ModlistEntry;
use crate::ui::home::page_home::HomeAction;
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HOME_CONFIRM_WIDTH_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_MODAL_LIST_GAP_PX, REDESIGN_MODAL_LIST_INDENT_PX,
    REDESIGN_SETTINGS_BOX_PADDING_X_PX, REDESIGN_SETTINGS_BOX_PADDING_Y_PX, ThemePalette,
    redesign_border_strong, redesign_pill_danger, redesign_shell_bg, redesign_text_muted,
    redesign_text_primary,
};

pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    entry: &ModlistEntry,
) -> Option<HomeAction> {
    let mut action = None;

    egui::Window::new(format!("Delete \"{}\"?", entry.name))
        .collapsible(false)
        .resizable(false)
        .default_width(REDESIGN_HOME_CONFIRM_WIDTH_PX)
        .frame(
            egui::Frame::NONE
                .fill(redesign_shell_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(REDESIGN_BORDER_RADIUS_PX)
                .inner_margin(egui::Margin::symmetric(
                    REDESIGN_SETTINGS_BOX_PADDING_X_PX as i8,
                    REDESIGN_SETTINGS_BOX_PADDING_Y_PX as i8,
                )),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("This will permanently remove:")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(REDESIGN_MODAL_LIST_GAP_PX);
            ui.horizontal(|ui| {
                ui.add_space(REDESIGN_MODAL_LIST_INDENT_PX);
                ui.label(
                    egui::RichText::new("• the modlist's registry entry (it disappears from Home)")
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_muted(palette)),
                );
            });
            ui.horizontal(|ui| {
                ui.add_space(REDESIGN_MODAL_LIST_INDENT_PX);
                ui.label(
                    egui::RichText::new("• the install folder on disk:")
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_muted(palette)),
                );
            });
            ui.horizontal(|ui| {
                ui.add_space(REDESIGN_MODAL_LIST_INDENT_PX);
                ui.label(
                    egui::RichText::new(entry.destination_folder.display().to_string())
                        .family(crate::ui::shared::redesign_tokens::redesign_font_mono())
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_primary(palette)),
                );
            });
            ui.add_space(REDESIGN_MODAL_LIST_GAP_PX);
            ui.label(
                egui::RichText::new("This action cannot be undone.")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_pill_danger(palette)),
            );
            ui.horizontal(|ui| {
                if redesign_btn(ui, palette, "Cancel", BtnOpts::default()).clicked() {
                    action = Some(HomeAction::CancelDelete);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Delete",
                    BtnOpts {
                        primary: true,
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    action = Some(HomeAction::ConfirmDeleteIntent);
                }
            });
        });

    action
}
