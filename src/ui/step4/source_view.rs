// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step4::format::render_weidu_colored_line;
use crate::ui::step4::tabs::draw_tab;
use crate::ui::step5::log_files::source_log_infos;

pub fn render_source_logs(ui: &mut egui::Ui, state: &mut WizardState) {
    let infos = source_log_infos(&state.step1);
    if infos.is_empty() {
        ui.label("No source WeiDU logs configured.");
        return;
    }

    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    ui.horizontal(|ui| {
        if show_bgee && show_bg2ee {
            draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
            draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
        } else if show_bgee {
            ui.label(egui::RichText::new("BGEE").monospace());
        } else if show_bg2ee {
            ui.label(egui::RichText::new("BG2EE").monospace());
        }
    });
    ui.add_space(6.0);

    let active_tag = match state.step1.game_install.as_str() {
        "BG2EE" => "bg2ee",
        "EET" if state.step3.active_game_tab == "BG2EE" => "bg2ee",
        _ => "bgee",
    };
    let Some(info) = infos.into_iter().find(|i| i.tag == active_tag) else {
        ui.label("No source WeiDU log configured for this tab.");
        return;
    };

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Source").strong());
        ui.label(egui::RichText::new(info.path.to_string_lossy().to_string()).monospace());
    });
    let status = if info.exists { "Found" } else { "Missing" };
    ui.horizontal(|ui| {
        ui.label(format!("Status: {status}"));
        if let Some(sz) = info.size_bytes {
            ui.label(format!("Size: {sz} bytes"));
        }
    });
    ui.add_space(4.0);

    if !info.exists {
        return;
    }

    let content = match fs::read_to_string(&info.path) {
        Ok(v) => v,
        Err(err) => {
            ui.label(format!("Failed to read file: {err}"));
            return;
        }
    };
    let lines: Vec<&str> = content.lines().collect();
    let nav_clearance = 26.0;
    let list_height = (ui.available_height() - nav_clearance).max(180.0);
    egui::ScrollArea::vertical()
        .id_salt(("step4_source_logs_scroll", active_tag))
        .max_height(list_height)
        .show(ui, |ui| {
            if lines.is_empty() {
                ui.label("Selected source log is empty.");
                return;
            }
            for line in lines {
                render_weidu_colored_line(ui, line);
            }
        });
}
