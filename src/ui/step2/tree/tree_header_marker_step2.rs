// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step2ModState;

pub(crate) fn render(ui: &mut egui::Ui, mod_state: &Step2ModState) {
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
        crate::ui::shared::theme_global::text_muted()
    } else {
        match marker.as_str() {
            "+" => crate::ui::shared::theme_global::success(),
            "!" => crate::ui::shared::theme_global::error(),
            _ => ui.visuals().text_color(),
        }
    };
    ui.label(crate::ui::shared::typography_global::strong(marker).color(marker_color));
}
