// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use chrono::{DateTime, Local};
use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::orchestrator::widgets::clipboard;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::service_diagnostics_support_step5::source_log_infos;
use crate::ui::step5::service_step5_command_step5::{
    build_command_preview_lines, build_install_command, wrap_display_line,
};

pub(crate) fn render(ui: &mut egui::Ui, state: &WizardState, _palette: ThemePalette) {
    let top_h = 190.0;
    if state.step5.hide_top_frames_after_install {
        return;
    }

    ui.columns(2, |columns| {
        render_command_panel(&mut columns[0], state, top_h);
        render_summary_panel(&mut columns[1], state, top_h);
    });
    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
}

fn render_command_panel(ui: &mut egui::Ui, state: &WizardState, top_h: f32) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(top_h);
        ui.set_max_height(top_h);
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::section_title(
                "Command",
            ));
            if ui.button("Copy Command").clicked() {
                clipboard::copy(ui.ctx(), build_install_command(&state.step1));
            }
        });
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_SM);
        let max_cols = floored_columns(ui.available_width(), 7.4, 36);
        egui::ScrollArea::vertical()
            .id_salt("step5_command_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for line in build_command_preview_lines(&state.step1) {
                    for wrapped in wrap_display_line(&line, max_cols) {
                        ui.monospace(wrapped);
                    }
                }
            });
    });
}

fn render_summary_panel(ui: &mut egui::Ui, state: &WizardState, top_h: f32) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.set_min_height(top_h);
        ui.set_max_height(top_h);
        ui.label(crate::ui::shared::typography_global::section_title(
            "Summary",
        ));
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        egui::ScrollArea::vertical()
            .id_salt("step5_summary_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| render_summary_grid(ui, state));
    });
}

fn render_summary_grid(ui: &mut egui::Ui, state: &WizardState) {
    egui::Grid::new("step5_summary_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Game Install:");
            ui.monospace(&state.step1.game_install);
            ui.end_row();
            ui.label("Mods Folder:");
            ui.monospace(&state.step1.mods_folder);
            ui.end_row();
            ui.label("WeiDU binary:");
            ui.monospace(&state.step1.weidu_binary);
            ui.end_row();
            ui.label("Language:");
            ui.monospace(&state.step1.language);
            ui.end_row();
            ui.label("Skip Installed:");
            ui.monospace(state.step1.skip_installed.to_string());
            ui.end_row();
            ui.label("Strict Matching:");
            ui.monospace(state.step1.strict_matching.to_string());
            ui.end_row();

            for info in source_log_infos(&state.step1) {
                render_source_log_info(ui, &info);
            }
        });
}

fn render_source_log_info(ui: &mut egui::Ui, info: &crate::ui::step5::log_files::SourceLogInfo) {
    let tag = match info.tag {
        "bgee" => "BGEE log:",
        "bg2ee" => "BG2EE log:",
        other => other,
    };
    ui.label(tag);
    ui.scope(|ui| {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.monospace(info.path.display().to_string())
            .on_hover_text(info.path.display().to_string());
    });
    ui.end_row();

    ui.label("Log modified:");
    let modified_text = info
        .modified
        .map_or_else(|| "missing".to_string(), format_system_time);
    ui.monospace(modified_text);
    ui.end_row();

    ui.label("Log size:");
    let size_text = if info.exists {
        info.size_bytes
            .map_or_else(|| "unknown".to_string(), |n| format!("{n} bytes"))
    } else {
        "missing".to_string()
    };
    ui.monospace(size_text);
    ui.end_row();
}

fn floored_columns(width: f32, column_width: f32, minimum: usize) -> usize {
    let estimate = (width / column_width).floor();
    if estimate.is_finite() {
        estimate
            .to_string()
            .parse::<usize>()
            .unwrap_or(minimum)
            .max(minimum)
    } else {
        minimum
    }
}

fn format_system_time(time: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
