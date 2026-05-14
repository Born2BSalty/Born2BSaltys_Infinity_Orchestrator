// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step2ModState;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_error, redesign_success, redesign_text_muted, redesign_text_primary,
};

pub(crate) fn render(ui: &mut egui::Ui, mod_state: &Step2ModState, palette: ThemePalette) {
    let marker = if mod_state.update_locked {
        Some("\u{1F512}".to_string())
    } else {
        mod_state.package_marker.map(|value| value.to_string())
    };
    let Some(marker) = marker else {
        return;
    };
    ui.add_space(4.0);
    let marker_color = if mod_state.update_locked {
        redesign_text_muted(palette)
    } else {
        match marker.as_str() {
            "+" => redesign_success(palette),
            "!" => redesign_error(palette),
            _ => redesign_text_primary(palette),
        }
    };
    ui.label(crate::ui::shared::typography_global::strong(marker).color(marker_color));
}
