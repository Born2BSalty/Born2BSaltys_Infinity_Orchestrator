// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn render_cancel_confirm(
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
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_FORCE_CANCEL);
            if state.step5.cancel_force_checked {
                ui.label(
                    crate::ui::shared::typography_global::plain(
                        "Warning: force cancel can leave installation in a broken state.",
                    )
                    .color(crate::ui::shared::theme_global::warning()),
                );
            } else {
                ui.label(crate::ui::shared::typography_global::weak(
                    "Safe cancel: wait for current component boundary, then stop.",
                ));
            }
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
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
                        state.step5.last_cancel_mode = "force".to_string();
                        state.step5.resume_available = false;
                        state.step5.resume_targets = crate::ui::state::ResumeTargets::default();
                    } else {
                        state.step5.cancel_pending = true;
                        state.step5.cancel_pending_started_unix_secs = Some(now_unix_secs());
                        state.step5.cancel_pending_output_len =
                            terminal.as_deref().map(|term| term.console_text().len());
                        state.step5.cancel_pending_boundary_count =
                            terminal.as_deref().map(|term| term.boundary_event_count());
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
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
