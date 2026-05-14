// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_warning};

pub(crate) fn render_cancel_confirm(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    palette: ThemePalette,
) {
    if !state.step5.cancel_confirm_open {
        return;
    }

    egui::Window::new("Confirm Cancel")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ui.ctx(), |ui| {
            ui.label("Cancel active install?");
            ui.checkbox(
                &mut state.step5.cancel_force_checked,
                "Force cancel (emergency)",
            )
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_FORCE_CANCEL);
            if state.step5.cancel_force_checked {
                ui.label(
                    crate::ui::shared::typography_global::plain(
                        "Warning: force cancel can leave installation in a broken state.",
                    )
                    .color(redesign_warning(palette)),
                );
            } else {
                ui.label(crate::ui::shared::typography_global::weak(
                    "Safe cancel: wait for current component boundary, then stop.",
                ));
            }
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
            ui.horizontal(|ui| {
                if ui.button("Yes, cancel").clicked() {
                    crate::app::step5::install_flow::confirm_cancel_request(state, terminal);
                }
                if ui.button("No").clicked() {
                    crate::app::step5::install_flow::dismiss_cancel_request(state);
                }
            });
        });
}
