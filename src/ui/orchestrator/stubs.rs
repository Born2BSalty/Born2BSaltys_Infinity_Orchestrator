// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::nav_destination::NavDestination;
use crate::ui::orchestrator::page_router::PageAction;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

pub fn render_home_stub(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    dev_mode: bool,
    dev_seed_message: Option<&str>,
) -> Option<PageAction> {
    render_stub_title(
        ui,
        palette,
        "Welcome back, adventurer",
        "Coming in Phase 5 — SPEC §3",
    );
    if let Some(message) = dev_seed_message {
        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(message)
                .size(13.0)
                .color(redesign_text_muted(palette)),
        );
    }

    if dev_mode {
        ui.add_space(12.0);
        if ui.button("Open workspace stub (dev)").clicked() {
            return Some(PageAction::Navigate(NavDestination::Workspace {
                modlist_id: None,
            }));
        }
        if ui.button("Seed test modlist (dev)").clicked() {
            return Some(PageAction::SeedTestModlist);
        }
    }

    None
}

pub fn render_install_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_stub_title(
        ui,
        palette,
        "Install shared modlist",
        "Coming in Phase 5 — SPEC §4",
    );
}

pub fn render_create_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_stub_title(
        ui,
        palette,
        "Create / edit modlist",
        "Coming in Phase 6 — SPEC §5",
    );
}

pub fn render_settings_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_stub_title(ui, palette, "Settings", "Coming in Phase 4 — SPEC §11");
}

pub fn render_workspace_stub(ui: &mut egui::Ui, palette: ThemePalette) {
    render_stub_title(ui, palette, "Workspace", "Coming in Phase 6 — SPEC §2.2");
}

fn render_stub_title(ui: &mut egui::Ui, palette: ThemePalette, title: &str, subtitle: &str) {
    ui.add_space(24.0);
    ui.label(
        egui::RichText::new(title)
            .size(22.0)
            .color(redesign_text_primary(palette)),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(subtitle)
            .size(13.0)
            .color(redesign_text_muted(palette)),
    );
}
