// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::preview_tabs;
use crate::ui::install::state_install::{InstallAction, InstallScreenState, InstallStage};
use crate::ui::orchestrator::widgets::btn::{BtnOpts, redesign_btn};
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_LABEL_FONT_SIZE_PX, REDESIGN_SETTINGS_ROW_GAP_PX,
    REDESIGN_SUBFLOW_FOOTER_GAP_PX, REDESIGN_SUBFLOW_SECTION_GAP_PX, ThemePalette,
    redesign_pill_warn, redesign_text_muted, redesign_text_primary,
};

pub(super) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    state: &mut InstallScreenState,
) -> Option<InstallAction> {
    let Some(preview) = state.preview.clone() else {
        state.stage = InstallStage::Paste;
        return None;
    };
    let mut action = None;

    screen_title::render(
        ui,
        palette,
        "Shared modlist preview",
        Some("review what will be installed before BIO downloads anything"),
    );

    if !preview.allow_auto_install {
        render_auto_install_disabled_banner(ui, palette);
        ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);
    }

    render_overview(ui, palette, &preview);
    ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);
    preview_tabs::render(ui, palette, &mut state.preview_tab, &preview);
    ui.add_space(REDESIGN_SUBFLOW_SECTION_GAP_PX);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_SUBFLOW_FOOTER_GAP_PX;
        if redesign_btn(ui, palette, "<- Back", BtnOpts::default()).clicked() {
            state.stage = InstallStage::Paste;
        }
        if redesign_btn(
            ui,
            palette,
            if state.reinstall_modlist_id.is_some() {
                "Reinstall ->"
            } else {
                "Import Modlist ->"
            },
            BtnOpts {
                primary: true,
                disabled: !preview.allow_auto_install,
                ..Default::default()
            },
        )
        .clicked()
        {
            action = Some(InstallAction::BeginInstallPreviewAccepted);
        }
    });
    ui.label(
        egui::RichText::new("downloads and extraction are not wired in this batch")
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_text_muted(palette)),
    );
    action
}

fn render_auto_install_disabled_banner(ui: &mut egui::Ui, palette: ThemePalette) {
    redesign_box(ui, palette, None, |ui| {
        ui.label(
            egui::RichText::new(
                "Draft modlist code - review and customize it in Create before installing.",
            )
            .size(REDESIGN_HINT_FONT_SIZE_PX)
            .color(redesign_pill_warn(palette)),
        );
    });
}

fn render_overview(ui: &mut egui::Ui, palette: ThemePalette, preview: &ModlistSharePreview) {
    redesign_box(ui, palette, Some("overview"), |ui| {
        ui.spacing_mut().item_spacing.y = REDESIGN_SETTINGS_ROW_GAP_PX;
        ui.horizontal_wrapped(|ui| {
            render_stat(ui, palette, "Game", &preview.game_install);
            render_stat(ui, palette, "Mode", &preview.install_mode);
            render_stat(
                ui,
                palette,
                "BGEE entries",
                &preview.bgee_entries.to_string(),
            );
            render_stat(
                ui,
                palette,
                "BG2EE entries",
                &preview.bg2ee_entries.to_string(),
            );
        });
    });
}

fn render_stat(ui: &mut egui::Ui, palette: ThemePalette, label: &str, value: &str) {
    ui.label(
        egui::RichText::new(format!("{label}: "))
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .color(redesign_text_muted(palette)),
    );
    ui.label(
        egui::RichText::new(value)
            .size(REDESIGN_LABEL_FONT_SIZE_PX)
            .strong()
            .color(redesign_text_primary(palette)),
    );
}
