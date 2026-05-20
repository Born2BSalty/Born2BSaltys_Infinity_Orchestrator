// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::render_screen_title;
use crate::ui::settings::state_settings::SettingsTab;
use crate::ui::settings::widgets::tab_strip;
use crate::ui::settings::{tab_accounts, tab_advanced, tab_general, tab_paths, tab_tools};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp, _ctx: &egui::Context) {
    let palette = orchestrator.theme_palette;
    render_screen_title(ui, palette, "Settings", None);

    let mut current = orchestrator.settings_screen_state.active_tab;
    let active = current;
    tab_strip::render(
        ui,
        palette,
        SettingsTab::all(),
        &mut current,
        |ui| match active {
            SettingsTab::General => tab_general::render(ui, orchestrator),
            SettingsTab::Paths => tab_paths::render(ui, orchestrator),
            SettingsTab::Tools => tab_tools::render(ui, orchestrator),
            SettingsTab::Accounts => tab_accounts::render(ui, orchestrator),
            SettingsTab::Advanced => tab_advanced::render(ui, orchestrator),
        },
    );
    orchestrator.settings_screen_state.active_tab = current;
}
