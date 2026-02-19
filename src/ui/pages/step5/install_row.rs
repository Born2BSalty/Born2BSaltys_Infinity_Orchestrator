// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

use super::{Step5Action, cancel_flow, install_flow, menus, prompt_answers};

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
                egui::RichText::new("Install in progress...")
                    .color(egui::Color32::from_rgb(224, 196, 156))
                    .strong(),
            );
            ui.add_space(8.0);
        }

        if state.step5.install_running {
            if ui
                .add_enabled(
                    can_install,
                    egui::Button::new("Cancel Install").min_size(egui::vec2(130.0, 28.0)),
                )
                .on_hover_text("Request cancel. Confirmation required.")
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
                    egui::Button::new(button_label).min_size(egui::vec2(130.0, 28.0)),
                )
                .on_hover_text("Start installer with current configuration.");
            let install_resp = if dev_mode && !diagnostics_ready {
                install_resp.on_hover_text(
                    "Dev mode requires diagnostics: enable Full Debug + Raw Output and set RUST_LOG to DEBUG or TRACE.",
                )
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
                egui::Color32::from_rgb(220, 100, 100)
            } else {
                egui::Color32::from_rgb(220, 180, 100)
            };
            ui.label(egui::RichText::new(badge_text).color(badge_color).strong());
        }

        if let Some(term) = terminal.as_deref_mut() {
            install_flow::start_if_requested(state, term);
        }

        if state.compat.error_count > 0
            && ui
                .button("Go to Step 3")
                .on_hover_text("Open Step 3 to review and fix blocking compatibility errors.")
                .clicked()
        {
            state.current_step = 2;
        }

        menus::render_actions_menu(ui, state, terminal.as_deref_mut());
        menus::render_diagnostics_menu(ui, state, terminal.as_deref(), dev_mode, exe_fingerprint);
        prompt_answers::render_button(ui, state);

        ui.add_space(8.0);
        let mut general_only = !state.step5.important_only && !state.step5.installed_only;
        let general_resp = ui
            .checkbox(&mut general_only, "General")
            .on_hover_text("Show full output (no filtering).");
        if general_resp.changed() && general_only {
            state.step5.important_only = false;
            state.step5.installed_only = false;
        }
        let important_resp = ui
            .checkbox(&mut state.step5.important_only, "Important Only")
            .on_hover_text("Show only important lines (warn/error/fatal/prompts).");
        if important_resp.changed() && state.step5.important_only {
            state.step5.installed_only = false;
        }
        let installed_resp = ui
            .checkbox(&mut state.step5.installed_only, "Installed Only")
            .on_hover_text("Show only installation progress lines.");
        if installed_resp.changed() && state.step5.installed_only {
            state.step5.important_only = false;
        }
        ui.checkbox(&mut state.step5.auto_scroll, "Auto-scroll")
            .on_hover_text("Follow new output automatically.");
    });
    cancel_flow::render_cancel_confirm(ui, state, terminal.as_deref_mut());
    action
}
