// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_HOME_CHIP_ROW_GAP_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_PILL_FONT_SIZE_PX, REDESIGN_SETTINGS_BOX_PADDING_X_PX,
    REDESIGN_SETTINGS_BOX_PADDING_Y_PX, ThemePalette, redesign_accent, redesign_border_strong,
    redesign_chrome_bg, redesign_success, redesign_text_faint, redesign_text_on_accent,
    redesign_text_primary,
};
use crate::ui::workspace::state_workspace::{WorkspaceStep, WorkspaceViewState};

pub fn render(ui: &mut egui::Ui, palette: ThemePalette, state: &WorkspaceViewState) {
    ui.columns(WorkspaceStep::ALL.len(), |columns| {
        for (index, step) in WorkspaceStep::ALL.iter().copied().enumerate() {
            render_segment(&mut columns[index], palette, state, step);
        }
    });
}

fn render_segment(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &WorkspaceViewState,
    step: WorkspaceStep,
) {
    let active = state.current_step == step;
    let completed = state.completed_steps.contains(&step);
    let fill = if active {
        redesign_accent(palette)
    } else {
        redesign_chrome_bg(palette)
    };
    let text_color = if active {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_primary(palette)
    };

    egui::Frame::NONE
        .fill(fill)
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(REDESIGN_BORDER_RADIUS_PX)
        .inner_margin(egui::Margin::symmetric(
            REDESIGN_SETTINGS_BOX_PADDING_X_PX as i8,
            REDESIGN_SETTINGS_BOX_PADDING_Y_PX as i8,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!("STEP {}", step.number()))
                            .size(REDESIGN_PILL_FONT_SIZE_PX)
                            .color(redesign_text_faint(palette)),
                    );
                    ui.add_space(REDESIGN_HOME_CHIP_ROW_GAP_PX);
                    ui.label(
                        egui::RichText::new(step.label())
                            .size(REDESIGN_LABEL_FONT_SIZE_PX)
                            .strong()
                            .color(text_color),
                    );
                });
                if completed {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("✓")
                                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                                .strong()
                                .color(redesign_success(palette)),
                        );
                    });
                }
            });
        });
}
