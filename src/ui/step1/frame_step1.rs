// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::layout::{SECTION_GAP, TOP_BOX_HEIGHT};
use crate::ui::shared::layout_tokens_global::{STEP1_ADVANCED_MIN_H, STEP1_SIDE_PANEL_MIN_H};
use crate::ui::state::Step1State;
use crate::ui::step1::content_step1;

pub fn render_top(ui: &mut egui::Ui, state: &mut Step1State, dev_mode: bool) {
    ui.columns(3, |cols| {
        cols[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            content_step1::render_game_selection_content(ui, state);
        });
        cols[0].add_space(SECTION_GAP);
        cols[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(STEP1_ADVANCED_MIN_H);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            content_step1::render_advanced_options_content(ui, state, dev_mode);
        });

        cols[1].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(STEP1_SIDE_PANEL_MIN_H + 1.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            content_step1::render_options_content(ui, state);
        });

        cols[2].group(|ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(STEP1_SIDE_PANEL_MIN_H + 1.0);
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            content_step1::render_flags_content(ui, state);
        });
    });
}

pub fn render_bottom(ui: &mut egui::Ui, state: &mut Step1State) {
    ui.columns(2, |cols| {
        cols[0].group(|ui| {
            ui.set_width(ui.available_width());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.set_min_height(TOP_BOX_HEIGHT - 20.0);
            content_step1::render_mods_folder_content(ui, state);
        });
        cols[1].group(|ui| {
            ui.set_width(ui.available_width());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.set_min_height(TOP_BOX_HEIGHT - 20.0);
            content_step1::render_tools_content(ui, state);
        });
    });

    ui.add_space(SECTION_GAP);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        content_step1::render_install_paths_content(ui, state, 0.0);
    });

    if state.weidu_log_mode_enabled {
        ui.add_space(SECTION_GAP);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            content_step1::render_weidu_log_mode_content(ui, state, 0.0);
        });
    }
}
