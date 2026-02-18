// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::BOTTOM_BOX_HEIGHT;
use crate::ui::state::Step1State;
use crate::ui::step1::validation::sync_weidu_log_mode;
use crate::ui::step1::widgets::{path_row_dir, section_title};

pub(super) fn render_weidu_log_mode_section(ui: &mut egui::Ui, s: &mut Step1State) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        ui.set_min_height(BOTTOM_BOX_HEIGHT);
        section_title(ui, "WeiDU Log Mode");
        egui::ScrollArea::vertical()
            .id_salt("weidu_log_mode_scroll")
            .max_height(BOTTOM_BOX_HEIGHT - 24.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(180.0);
                    ui.scope(|ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        ui.checkbox(&mut s.weidu_log_autolog, "autolog");
                        ui.checkbox(&mut s.weidu_log_logapp, "logapp");
                        ui.checkbox(&mut s.weidu_log_logextern, "log-extern");
                        ui.checkbox(&mut s.weidu_log_log_component, "log (per-component)");
                    });
                });
                ui.add_space(4.0);
                path_row_dir(ui, "Per-component folder", &mut s.weidu_log_folder);
                sync_weidu_log_mode(s);
            });
    });
}
