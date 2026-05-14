// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_TAB_FONT_SIZE_PX, REDESIGN_WORKSPACE_NAV_GAP_PX,
    ThemePalette, redesign_accent_deep, redesign_font_bold, redesign_text_faint,
    redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WorkspaceViewState;

use std::time::Instant;

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut WorkspaceViewState,
    share_enabled: bool,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(format!("Editing {}", state.modlist_name))
                    .family(redesign_font_bold())
                    .size(REDESIGN_TAB_FONT_SIZE_PX)
                    .color(redesign_text_primary(palette)),
            );
            if let Some(fork_meta) = &state.fork_meta {
                ui.label(
                    egui::RichText::new(format!("⑂ forked from {}", fork_meta.source_name))
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .color(redesign_accent_deep(palette)),
                );
            } else {
                ui.label(
                    egui::RichText::new("workspace draft")
                        .size(REDESIGN_LABEL_FONT_SIZE_PX)
                        .color(redesign_text_faint(palette)),
                );
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if redesign_btn(
                ui,
                palette,
                "share import code",
                BtnOpts {
                    primary: share_enabled,
                    disabled: !share_enabled,
                    small: true,
                },
            )
            .on_hover_text(if share_enabled {
                "View and copy the import code for this modlist"
            } else {
                "Available after a successful install"
            })
            .clicked()
            {
                state.share_paste_open = true;
            }
            ui.add_space(REDESIGN_WORKSPACE_NAV_GAP_PX);
            if state
                .save_draft_flash_until
                .is_some_and(|until| Instant::now() >= until)
            {
                state.save_draft_flash_until = None;
            }
            let label = if state.save_draft_flash_until.is_some() {
                "✓ saved!"
            } else {
                "save draft"
            };
            if redesign_btn(
                ui,
                palette,
                label,
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                state.save_draft_requested = true;
            }
        });
    });
}
