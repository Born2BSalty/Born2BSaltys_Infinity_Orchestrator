// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::install::preview_counts;
use crate::ui::orchestrator::widgets::redesign_box;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_text_muted, redesign_text_primary,
};

pub(crate) fn render(ui: &mut egui::Ui, palette: ThemePalette, preview: &ModlistSharePreview) {
    redesign_box(ui, palette, None, |ui| {
        let total_w = ui.available_width();
        let col_gap = 16.0;
        let col_w = ((total_w - col_gap * 3.0) / 4.0).max(60.0);

        let components = preview.bgee_entries + preview.bg2ee_entries;
        let mods =
            preview_counts::distinct_mod_count(&preview.bgee_log_text, &preview.bg2ee_log_text);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = col_gap;
            cell(ui, palette, col_w, "Game", &preview.game_install);
            cell(ui, palette, col_w, "Mods", &mods.to_string());
            cell(ui, palette, col_w, "Components", &components.to_string());
            cell(
                ui,
                palette,
                col_w,
                "BGEE/BG2EE entries",
                &format!("{}/{}", preview.bgee_entries, preview.bg2ee_entries),
            );
        });
    });
}

fn cell(ui: &mut egui::Ui, palette: ThemePalette, width: f32, label: &str, value: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 22.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(
                egui::RichText::new(format!("{label}:"))
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.label(
                egui::RichText::new(value)
                    .size(14.0)
                    .family(egui::FontFamily::Name("poppins_bold".into()))
                    .color(redesign_text_primary(palette)),
            );
        },
    );
}
