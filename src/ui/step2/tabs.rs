// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str) {
    let is_active = active == value;
    let fill = if is_active {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let stroke = if is_active {
        ui.visuals().widgets.active.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    let text_color = if is_active {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };

    let button = egui::Button::new(egui::RichText::new(value).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(4));

    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}

pub fn normalize_active_tab(state: &mut WizardState) {
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let active_is_visible =
        (state.step2.active_game_tab == "BGEE" && show_bgee)
            || (state.step2.active_game_tab == "BG2EE" && show_bg2ee);
    if active_is_visible {
        return;
    }
    if show_bgee {
        state.step2.active_game_tab = "BGEE".to_string();
    } else if show_bg2ee {
        state.step2.active_game_tab = "BG2EE".to_string();
    }
}
