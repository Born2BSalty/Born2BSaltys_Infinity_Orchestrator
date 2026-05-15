// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use eframe::egui;

use crate::registry::errors::RegistryError;
use crate::ui::orchestrator::widgets::r_box;
use crate::ui::orchestrator::widgets::screen_title;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_PAGE_PADDING_X_PX, REDESIGN_PAGE_PADDING_Y_PX, ThemePalette, redesign_pill_danger,
    redesign_text_muted, redesign_text_primary,
};

pub fn render_registry_error(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    err: &RegistryError,
    backup_path: Option<&Path>,
) {
    ui.add_space(REDESIGN_PAGE_PADDING_Y_PX);
    ui.horizontal(|ui| {
        ui.add_space(REDESIGN_PAGE_PADDING_X_PX);
        ui.vertical(|ui| {
            screen_title::render(
                ui,
                palette,
                "modlist registry is corrupt or unreadable",
                Some("Restore from backup or delete the file to continue"),
            );
            r_box::redesign_box(ui, palette, Some("terminal error"), |ui| {
                let (path, details) = registry_error_parts(err);
                render_error_row(ui, palette, "path", path.display().to_string());
                ui.add_space(10.0);
                render_error_row(ui, palette, "error details", details);
                if let Some(backup_path) = backup_path {
                    ui.add_space(10.0);
                    render_error_row(ui, palette, "backup", backup_path.display().to_string());
                }
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("No automatic recovery is available for this state file.")
                        .size(13.0)
                        .color(redesign_pill_danger(palette)),
                );
            });
        });
    });
}

fn registry_error_parts(err: &RegistryError) -> (&Path, String) {
    match err {
        RegistryError::Io { path, source } => (path.as_path(), source.to_string()),
        RegistryError::Corrupt { path, message } | RegistryError::Serialize { path, message } => {
            (path.as_path(), message.clone())
        }
    }
}

fn render_error_row(ui: &mut egui::Ui, palette: ThemePalette, label: &str, value: String) {
    ui.label(
        egui::RichText::new(label)
            .size(11.0)
            .color(redesign_text_muted(palette)),
    );
    ui.label(
        egui::RichText::new(value)
            .monospace()
            .size(13.0)
            .color(redesign_text_primary(palette)),
    );
}
