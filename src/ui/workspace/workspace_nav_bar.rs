// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_WORKSPACE_NAV_GAP_PX, ThemePalette, redesign_text_faint,
};
use crate::ui::workspace::state_workspace::WorkspaceViewState;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut WorkspaceViewState,
    disable_prev: bool,
) {
    ui.horizontal(|ui| {
        let previous_disabled = disable_prev || state.current_step.previous().is_none();
        let previous_response = redesign_btn(
            ui,
            palette,
            "← Previous",
            BtnOpts {
                disabled: previous_disabled,
                ..Default::default()
            },
        );
        let previous_response = if disable_prev {
            previous_response
                .on_hover_text("Disabled while install is running or after a successful install")
        } else {
            previous_response
        };
        if previous_response.clicked()
            && !previous_disabled
            && let Some(previous) = state.current_step.previous()
        {
            state.current_step = previous;
        }

        ui.add_space(REDESIGN_WORKSPACE_NAV_GAP_PX);
        ui.label(
            egui::RichText::new(format!(
                "on Step {} · {} · step {} of {}",
                state.current_step.number(),
                state.current_step.label(),
                state.current_step.number() - 1,
                crate::ui::workspace::state_workspace::WorkspaceStep::ALL.len()
            ))
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(redesign_text_faint(palette)),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(
                ui,
                palette,
                "Next →",
                BtnOpts {
                    primary: true,
                    disabled: state.current_step.next().is_none(),
                    ..Default::default()
                },
            )
            .clicked()
                && let Some(next) = state.current_step.next()
            {
                state.mark_current_complete();
                state.current_step = next;
            }
        });
    });
}
