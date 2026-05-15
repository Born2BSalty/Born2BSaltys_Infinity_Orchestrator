// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::state_install::InstallPreviewTab;
use crate::ui::orchestrator::widgets::r_box::redesign_box;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_HINT_FONT_SIZE_PX, REDESIGN_HOME_CHIP_ROW_GAP_PX, REDESIGN_INPUT_MIN_HEIGHT_PX,
    REDESIGN_LABEL_FONT_SIZE_PX, ThemePalette, redesign_accent, redesign_input_bg,
    redesign_text_muted, redesign_text_on_accent, redesign_text_primary,
};

pub(super) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    active: &mut InstallPreviewTab,
    preview: &ModlistSharePreview,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = REDESIGN_HOME_CHIP_ROW_GAP_PX;
        for (tab, label) in tabs() {
            let selected = *active == tab;
            let response = egui::Frame::NONE
                .fill(if selected {
                    redesign_accent(palette)
                } else {
                    redesign_input_bg(palette)
                })
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(label)
                            .size(REDESIGN_HINT_FONT_SIZE_PX)
                            .color(if selected {
                                redesign_text_on_accent(palette)
                            } else {
                                redesign_text_primary(palette)
                            }),
                    );
                })
                .response
                .interact(egui::Sense::click());
            if response.clicked() {
                *active = tab;
            }
        }
    });

    redesign_box(ui, palette, None, |ui| {
        ui.set_min_height(REDESIGN_INPUT_MIN_HEIGHT_PX);
        ui.label(
            egui::RichText::new(tab_text(active, preview))
                .size(REDESIGN_LABEL_FONT_SIZE_PX)
                .color(redesign_text_muted(palette))
                .monospace(),
        );
    });
}

const fn tabs() -> [(InstallPreviewTab, &'static str); 6] {
    [
        (InstallPreviewTab::Summary, "Summary"),
        (InstallPreviewTab::BgeeWeidu, "BGEE WeiDU"),
        (InstallPreviewTab::Bg2eeWeidu, "BG2EE WeiDU"),
        (InstallPreviewTab::UserDownloads, "User Downloads"),
        (InstallPreviewTab::InstalledRefs, "Installed Refs"),
        (InstallPreviewTab::ModConfigs, "Mod Configs"),
    ]
}

fn tab_text(active: &InstallPreviewTab, preview: &ModlistSharePreview) -> String {
    match active {
        InstallPreviewTab::Summary => format!(
            "BIO version: {}\nGame install: {}\nInstall mode: {}\nBGEE entries: {}\nBG2EE entries: {}\nSource overrides: {}\nInstalled refs: {}\nMod config files: {}\nAllow auto-install: {}",
            preview.bio_version,
            preview.game_install,
            preview.install_mode,
            preview.bgee_entries,
            preview.bg2ee_entries,
            preview.has_source_overrides,
            preview.has_installed_refs,
            preview.mod_config_count,
            preview.allow_auto_install
        ),
        InstallPreviewTab::BgeeWeidu => preview.bgee_log_text.clone(),
        InstallPreviewTab::Bg2eeWeidu => preview.bg2ee_log_text.clone(),
        InstallPreviewTab::UserDownloads => preview.source_overrides_text.clone(),
        InstallPreviewTab::InstalledRefs => preview.installed_refs_text.clone(),
        InstallPreviewTab::ModConfigs => preview.mod_configs_text.clone(),
    }
}
