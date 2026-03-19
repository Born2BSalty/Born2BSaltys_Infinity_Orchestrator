// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::layout_tokens_global::STEP5_SECTION_GAP;
use crate::ui::state::WizardState;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::menus_step5 as menus;
use crate::ui::step5::prompt_answers_step5 as prompt_answers;
use crate::ui::step5::service_step5::apply_dev_defaults;
use crate::ui::step5::state_step5::install_in_progress;
use crate::ui::step5::status_bar_step5 as status_bar;
use crate::ui::step5::top_panels_step5 as top_panels;
use crate::ui::terminal::EmbeddedTerminal;

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step5Action> {
    apply_dev_defaults(state, dev_mode);

    let _running = install_in_progress(state);

    dev_header::render(ui, state, dev_mode);

    top_panels::render(ui, state);

    let action = install_row::render(
        ui,
        state,
        terminal.as_deref_mut(),
        terminal_error,
        dev_mode,
        exe_fingerprint,
    );

    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_console(ui, state, terminal.as_deref_mut(), terminal_error);
    section_gap(ui, STEP5_SECTION_GAP);
    status_bar::render_status_and_input(ui, state, terminal.as_deref_mut());
    prompt_answers::render_window(ui, state, terminal.as_deref());

    action
}

pub fn section_gap(ui: &mut egui::Ui, size: f32) {
    ui.add_space(size);
}

mod cancel_flow {
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
}

mod dev_header {
    use eframe::egui;

    use crate::ui::state::WizardState;

    pub(super) fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool) {
        ui.heading("Step 5: Install, Logs, Diagnostics");
        ui.label("Final execution view.");
        if dev_mode {
            let has_rust_log = state.step1.rust_log_debug || state.step1.rust_log_trace;
            let level = if state.step1.rust_log_trace {
                "TRACE"
            } else if state.step1.rust_log_debug {
                "DEBUG"
            } else {
                "OFF"
            };
            let color = if has_rust_log {
                crate::ui::shared::theme_global::success()
            } else {
                crate::ui::shared::theme_global::accent_path()
            };
            let msg = if has_rust_log {
                format!("Dev Mode: RUST_LOG={level} selected.")
            } else {
                "Dev Mode: open Diagnostics and choose RUST_LOG=DEBUG or TRACE before Install."
                    .to_string()
            };
            ui.label(crate::ui::shared::typography_global::strong(msg).color(color));
        }
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_LG);
    }
}

mod install_row {
    use eframe::egui;

    use crate::ui::state::WizardState;
    use crate::ui::terminal::EmbeddedTerminal;

    use crate::ui::step5::action_step5::Step5Action;

    use super::{cancel_flow, menus, prompt_answers};

    pub(super) fn render(
        ui: &mut egui::Ui,
        state: &mut WizardState,
        mut terminal: Option<&mut EmbeddedTerminal>,
        terminal_error: Option<&str>,
        dev_mode: bool,
        exe_fingerprint: &str,
    ) -> Option<Step5Action> {
        let mut action: Option<Step5Action> = None;
        ui.horizontal(|ui| {
        let can_install = terminal.is_some() && terminal_error.is_none();
        let diagnostics_ready = menus::diagnostics_ready_for_dev(state);
        let install_allowed = can_install && (!dev_mode || diagnostics_ready);

        if state.step5.install_running {
            ui.label(
                crate::ui::shared::typography_global::strong("Install in progress...")
                    .color(crate::ui::shared::theme_global::accent_path()),
            );
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        }

        if state.step5.install_running {
            if ui
                .add_enabled(
                    can_install,
                    egui::Button::new("Cancel Install").min_size(egui::vec2(
                        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_W,
                        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_H,
                    )),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP5_CANCEL_INSTALL)
                .clicked()
            {
                state.step5.cancel_force_checked = false;
                state.step5.cancel_confirm_open = true;
            }
        } else {
            let button_label = if state.step5.resume_available {
                "Resume Install"
            } else if state.step5.has_run_once {
                "Restart Install"
            } else {
                "Install"
            };
            let install_resp = ui
                .add_enabled(
                    install_allowed,
                    egui::Button::new(button_label).min_size(egui::vec2(
                        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_W,
                        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_H,
                    )),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP5_START_INSTALL);
            let install_resp = if dev_mode && !diagnostics_ready {
                install_resp.on_hover_text(crate::ui::shared::tooltip_global::STEP5_DEV_MODE_DIAG_REQUIRED)
            } else {
                install_resp
            };
            if install_resp.clicked() {
                if dev_mode && !diagnostics_ready {
                    state.step5.last_status_text =
                        "Dev mode install blocked: enable diagnostics (Full Debug + Raw Output + RUST_LOG DEBUG/TRACE)."
                            .to_string();
                    if let Some(term) = terminal.as_deref_mut() {
                        term.append_marker(
                            "Dev mode install blocked: enable diagnostics (Full Debug + Raw Output + RUST_LOG DEBUG/TRACE).",
                        );
                    }
                } else {
                    action = Some(Step5Action::CheckCompatBeforeInstall);
                }
            }
        }

        if state.compat.error_count > 0 || state.compat.warning_count > 0 {
            let badge_text = if state.compat.error_count > 0 {
                format!("{} errors", state.compat.error_count)
            } else {
                format!("{} warnings", state.compat.warning_count)
            };
            let badge_color = if state.compat.error_count > 0 {
                crate::ui::shared::theme_global::error()
            } else {
                crate::ui::shared::theme_global::warning_soft()
            };
            ui.label(crate::ui::shared::typography_global::strong(badge_text).color(badge_color));
        }

        if let Some(term) = terminal.as_deref_mut() {
            crate::ui::step5::service_step5::install_flow::start_if_requested(state, term);
        }

        if state.compat.error_count > 0
            && ui
                .button("Go to Step 3")
                .on_hover_text(crate::ui::shared::tooltip_global::STEP5_GO_TO_STEP3)
                .clicked()
        {
            state.current_step = 2;
        }

        menus::render_actions_menu(ui, state, terminal.as_deref_mut());
        menus::render_diagnostics_menu(ui, state, terminal.as_deref(), dev_mode, exe_fingerprint);
        prompt_answers::render_button(ui, state);

        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        let mut general_only = !state.step5.important_only && !state.step5.installed_only;
        let general_resp = ui
            .checkbox(&mut general_only, "General")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_GENERAL_OUTPUT);
        if general_resp.changed() && general_only {
            state.step5.important_only = false;
            state.step5.installed_only = false;
        }
        let important_resp = ui
            .checkbox(&mut state.step5.important_only, "Important Only")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_IMPORTANT_ONLY);
        if important_resp.changed() && state.step5.important_only {
            state.step5.installed_only = false;
        }
        let installed_resp = ui
            .checkbox(&mut state.step5.installed_only, "Installed Only")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_INSTALLED_ONLY);
        if installed_resp.changed() && state.step5.installed_only {
            state.step5.important_only = false;
        }
        ui.checkbox(&mut state.step5.auto_scroll, "Auto-scroll")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_AUTO_SCROLL);
    });
        cancel_flow::render_cancel_confirm(ui, state, terminal.as_deref_mut());
        action
    }
}
