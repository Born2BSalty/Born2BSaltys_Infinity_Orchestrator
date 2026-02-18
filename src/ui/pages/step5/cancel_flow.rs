// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render_cancel_confirm(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
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
            .on_hover_text(
                "Immediate stop. May leave game/mod state unrecoverable.",
            );
            if state.step5.cancel_force_checked {
                ui.label(
                    egui::RichText::new(
                        "Warning: force cancel can leave installation in a broken state.",
                    )
                    .color(egui::Color32::from_rgb(214, 168, 96)),
                );
            } else {
                ui.label(
                    egui::RichText::new(
                        "Safe cancel: wait for current component boundary, then stop.",
                    )
                    .weak(),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Yes, cancel").clicked() {
                    state.step5.cancel_requested = true;
                    if state.step5.cancel_force_checked {
                        if let Some(term) = terminal.as_mut() {
                            term.force_terminate();
                            term.focus();
                        }
                        state.step5.last_status_text = "Force cancel requested".to_string();
                        state.step5.cancel_pending = false;
                        state.step5.cancel_pending_started_unix_secs = None;
                        state.step5.cancel_pending_output_len = None;
                        state.step5.cancel_pending_boundary_count = None;
                        state.step5.cancel_signal_sent_unix_secs = None;
                        state.step5.cancel_attempt_count = 0;
                        state.step5.cancel_was_graceful = false;
                        state.step5.resume_available = false;
                    } else {
                        state.step5.cancel_pending = true;
                        state.step5.cancel_pending_started_unix_secs = Some(now_unix_secs());
                        state.step5.cancel_pending_output_len =
                            terminal.as_deref().map(|t| t.console_text().len());
                        state.step5.cancel_pending_boundary_count =
                            terminal.as_deref().map(|t| t.boundary_event_count());
                        state.step5.cancel_signal_sent_unix_secs = None;
                        state.step5.cancel_attempt_count = 0;
                        state.step5.last_status_text =
                            "Cancel pending (waiting for component boundary)".to_string();
                    }
                    state.step5.cancel_confirm_open = false;
                    state.step5.cancel_force_checked = false;
                }
                if ui.button("No").clicked() {
                    state.step5.cancel_confirm_open = false;
                    state.step5.cancel_force_checked = false;
                    state.step5.cancel_requested = false;
                }
            });
        });
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
