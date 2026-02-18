// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::step4::format::{format_step4_item, render_weidu_colored_line};
use crate::ui::step4::source_view::render_source_logs;
use crate::ui::step4::tabs::draw_tab;
use crate::ui::state::{Step3ItemState, WizardState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step4Action {
    SaveWeiduLog,
}

pub fn render(ui: &mut egui::Ui, state: &mut WizardState) -> Option<Step4Action> {
    let mut action = None;
    ui.heading("Step 4: Review");
    if state.step1.have_weidu_logs {
        ui.label("Review the source WeiDU log file(s) that will be used for install.");
        ui.label("Next continues to Step 5 without going through Step 2/3.");
    } else {
        ui.label("Verify setup and install order before running.");
        ui.label("Next will save weidu.log file(s) and continue to Step 5.");
    }
    ui.add_space(10.0);
    ui.add_space(2.0);
    if !state.step1.have_weidu_logs {
        ui.horizontal(|ui| {
            let label = match state.step1.game_install.as_str() {
                "EET" => "Save weidu.log's",
                _ => "Save weidu.log",
            };
            if ui
                .add_sized([148.0, 28.0], egui::Button::new(label))
                .on_hover_text("Write weidu.log file(s) from the current install order.")
                .clicked()
            {
                action = Some(Step4Action::SaveWeiduLog);
            }
        });
    }
    ui.add_space(8.0);
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        if state.step1.have_weidu_logs {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Source WeiDU Logs").strong().size(14.0));
            });
            ui.add_space(6.0);
            render_source_logs(ui, state);
        } else {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Install Order").strong().size(14.0));
                ui.add_space(12.0);
                let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
                let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
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
            let items = if state.step3.active_game_tab == "BG2EE" {
                &state.step3.bg2ee_items
            } else {
                &state.step3.bgee_items
            };
            let export_items: Vec<&Step3ItemState> = items.iter().filter(|i| !i.is_parent).collect();
            let nav_clearance = 26.0;
            let list_height = (ui.available_height() - nav_clearance).max(180.0);
            egui::ScrollArea::vertical().max_height(list_height).show(ui, |ui| {
                if export_items.is_empty() {
                    ui.label("No ordered components found.");
                } else {
                    for item in export_items {
                        render_weidu_colored_line(ui, &format_step4_item(item));
                    }
                }
            });
        }
    });
    action
}
