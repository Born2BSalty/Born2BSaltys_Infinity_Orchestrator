// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::home::page_home::HomeAction;
use crate::ui::home::state_home::HomeScreenState;
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HOME_CONFIRM_WIDTH_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_MODAL_LIST_GAP_PX, REDESIGN_SETTINGS_BOX_PADDING_X_PX,
    REDESIGN_SETTINGS_BOX_PADDING_Y_PX, ThemePalette, redesign_border_strong, redesign_input_bg,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};

pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    state: &mut HomeScreenState,
) -> Option<HomeAction> {
    let mut action = None;

    egui::Window::new("Rename modlist")
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
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_SETTINGS_BOX_PADDING_X_PX,
                    ),
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_SETTINGS_BOX_PADDING_Y_PX,
                    ),
                )),
        )
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Modlist name")
                    .size(REDESIGN_LABEL_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            ui.label(
                egui::RichText::new(
                    "Only the registry name changes. The install folder is not renamed.",
                )
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_muted(palette)),
            );
            ui.add_space(REDESIGN_MODAL_LIST_GAP_PX);
            egui::Frame::NONE
                .fill(redesign_input_bg(palette))
                .stroke(egui::Stroke::new(
                    REDESIGN_BORDER_WIDTH_PX,
                    redesign_border_strong(palette),
                ))
                .corner_radius(REDESIGN_BORDER_RADIUS_PX)
                .inner_margin(egui::Margin::symmetric(
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_SETTINGS_BOX_PADDING_X_PX,
                    ),
                    crate::ui::shared::redesign_tokens::redesign_i8_px(
                        REDESIGN_SETTINGS_BOX_PADDING_Y_PX,
                    ),
                ))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.rename_value)
                            .desired_width(ui.available_width()),
                    );
                });
            ui.horizontal(|ui| {
                if redesign_btn(ui, palette, "Cancel", BtnOpts::default()).clicked() {
                    action = Some(HomeAction::CancelRename);
                }
                if redesign_btn(
                    ui,
                    palette,
                    "Save",
                    BtnOpts {
                        primary: true,
                        disabled: state.rename_value.trim().is_empty(),
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    action = Some(HomeAction::ConfirmRenameIntent);
                }
            });
        });

    action
}
