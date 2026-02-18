// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;
use chrono::{DateTime, Local};

use crate::ui::state::WizardState;
use crate::ui::step5::command::{
    build_command_preview_lines, build_install_command, wrap_display_line,
};
use crate::ui::step5::log_files::source_log_infos;

pub(super) fn render(ui: &mut egui::Ui, state: &WizardState) {
    let top_h = 190.0;
    if state.step5.hide_top_frames_after_install {
        return;
    }

    ui.columns(2, |columns| {
        columns[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(top_h);
            ui.set_max_height(top_h);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Command").strong().size(14.0));
                if ui.button("Copy Command").clicked() {
                    ui.ctx().copy_text(build_install_command(&state.step1));
                }
            });
            ui.add_space(6.0);
            let max_cols = ((ui.available_width() / 7.4).floor() as usize).max(36);
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

        columns[1].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(top_h);
            ui.set_max_height(top_h);
            ui.label(egui::RichText::new("Summary").strong().size(14.0));
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("step5_summary_scroll")
                .auto_shrink([false, false])
                        .show(ui, |ui| {
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

                            let source_logs = source_log_infos(&state.step1);
                            for info in source_logs {
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
                                    .map(format_system_time)
                                    .unwrap_or_else(|| "missing".to_string());
                                ui.monospace(modified_text);
                                ui.end_row();

                                ui.label("Log size:");
                                let size_text = if info.exists {
                                    info.size_bytes
                                        .map(|n| format!("{n} bytes"))
                                        .unwrap_or_else(|| "unknown".to_string())
                                } else {
                                    "missing".to_string()
                                };
                                ui.monospace(size_text);
                                ui.end_row();
                            }
                        });
                });
        });
    });
    ui.add_space(8.0);
}

fn format_system_time(time: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
