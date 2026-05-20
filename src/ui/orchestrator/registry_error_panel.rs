// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use eframe::egui;

use crate::registry::errors::RegistryError;
use crate::ui::orchestrator::widgets::{redesign_box, redesign_label, render_screen_title};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub fn render_registry_error(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    err: &RegistryError,
    backup_path: Option<&PathBuf>,
) {
    render_screen_title(
        ui,
        palette,
        "Modlist registry is corrupt or unreadable",
        Some("Infinity Orchestrator cannot start until the registry file is fixed."),
    );

    redesign_box(ui, palette, Some("registry error"), |ui| {
        ui.add_space(2.0);
        let (path, friendly) = error_summary(err);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("file ")
                    .size(13.0)
                    .family(egui::FontFamily::Proportional)
                    .color(redesign_text_muted(palette)),
            );
            ui.label(
                egui::RichText::new(path)
                    .size(13.0)
                    .family(egui::FontFamily::Monospace)
                    .color(redesign_text_primary(palette)),
            );
        });

        ui.add_space(8.0);
        let _ = redesign_label(ui, palette, &friendly);

        if let Some(backup) = backup_path {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("backup ")
                        .size(13.0)
                        .family(egui::FontFamily::Proportional)
                        .color(redesign_text_muted(palette)),
                );
                ui.label(
                    egui::RichText::new(backup.display().to_string())
                        .size(13.0)
                        .family(egui::FontFamily::Monospace)
                        .color(redesign_text_primary(palette)),
                );
            });
        }

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "Restore from backup or delete the file to continue. Then restart Infinity Orchestrator.",
            )
            .size(12.0)
            .family(egui::FontFamily::Proportional)
            .color(redesign_text_faint(palette)),
        );
        ui.add_space(2.0);
    });
}

fn error_summary(err: &RegistryError) -> (String, String) {
    match err {
        RegistryError::Corrupt { path, message } => (
            path.display().to_string(),
            format!("parse failed: {message}"),
        ),
        RegistryError::Io(io_err) => (String::from("(see log)"), format!("IO error: {io_err}")),
        RegistryError::Parse(parse_err) => (
            String::from("(see log)"),
            format!("parse error: {parse_err}"),
        ),
    }
}
