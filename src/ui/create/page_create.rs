// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::create::load_draft_dialog;
use crate::ui::create::stage_choose;
use crate::ui::create::stage_fork_download;
use crate::ui::create::stage_fork_paste;
use crate::ui::create::stage_fork_preview;
use crate::ui::create::state_create::{CreateAction, CreateScreenState, CreateStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_PAGE_PADDING_X_PX, REDESIGN_PAGE_PADDING_Y_PX, ThemePalette,
};

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut CreateScreenState,
) -> Option<CreateAction> {
    let mut action = None;

    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_X_PX),
            crate::ui::shared::redesign_tokens::redesign_i8_px(REDESIGN_PAGE_PADDING_Y_PX),
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    screen_title::render(
                        ui,
                        palette,
                        "Create / edit modlist",
                        Some(
                            "name your modlist, set destination + mods paths, then pick a starting point",
                        ),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if redesign_btn(
                        ui,
                        palette,
                        "load draft",
                        BtnOpts {
                            small: true,
                            ..Default::default()
                        },
                    )
                    .clicked()
                    {
                        state.load_draft_open = true;
                        action = Some(CreateAction::LoadDraftRequested);
                    }
                });
            });

            match state.stage {
                CreateStage::Choose => {
                    action = stage_choose::render(ui, palette, state).or(action);
                }
                CreateStage::ForkPaste => {
                    stage_fork_paste::render(ui, palette, state);
                }
                CreateStage::ForkPreview => {
                    stage_fork_preview::render(ui, palette, state);
                }
                CreateStage::ForkDownload => {
                    stage_fork_download::render(ui, palette, state);
                }
            }

            if state.load_draft_open {
                load_draft_dialog::render(ui, palette, state);
            }
        });

    action
}
