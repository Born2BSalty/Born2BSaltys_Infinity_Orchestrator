// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step5::install_flow::step3_install_block_reason;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub(crate) fn render_install_row(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> Option<Step5Action> {
    let mut action: Option<Step5Action> = None;
    ui.horizontal(|ui| {
        let can_install = terminal.is_some() && terminal_error.is_none();
        let diagnostics_ready = crate::ui::step5::menus_step5::diagnostics_ready_for_dev(state);
        let install_block_reason = step3_install_block_reason(state);
        let install_allowed =
            can_install
                && !state.step5.prep_running
                && install_block_reason.is_none()
                && (!dev_mode || diagnostics_ready);

        if state.step5.prep_running {
            ui.label(
                crate::ui::shared::typography_global::strong("Preparing target dirs...")
                    .color(crate::ui::shared::theme_global::accent_path()),
            );
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        } else if state.step5.install_running {
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
                );
            let install_resp = if let Some(reason) = install_block_reason.as_deref() {
                install_resp.on_hover_text(reason)
            } else if dev_mode && !diagnostics_ready {
                install_resp.on_hover_text(
                    crate::ui::shared::tooltip_global::STEP5_DEV_MODE_DIAG_REQUIRED,
                )
            } else {
                install_resp.on_hover_text(crate::ui::shared::tooltip_global::STEP5_START_INSTALL)
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
                    action = Some(Step5Action::StartInstall);
                }
            }
            if let Some(reason) = install_block_reason {
                ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
                ui.label(crate::ui::shared::typography_global::weak(reason));
            }
        }

        crate::ui::step5::menus_step5::render_actions_menu(ui, state, terminal.as_deref_mut());
        crate::ui::step5::menus_step5::render_diagnostics_menu(
            ui,
            state,
            terminal.as_deref(),
            dev_mode,
            exe_fingerprint,
        );
        crate::ui::step5::prompt_answers_step5::render_button(ui, state);
        if state.step5.has_run_once
            && !state.step5.install_running
            && state.step5.last_exit_code == Some(0)
            && ui.button("Export Modlist…").clicked()
        {
            match crate::app::modlist_share::export_modlist_share_code(state) {
                Ok(code) => {
                    state.step5.modlist_share_code = code;
                    state.step5.modlist_share_error.clear();
                }
                Err(err) => {
                    state.step5.modlist_share_code.clear();
                    state.step5.modlist_share_error = err;
                }
            }
            state.step5.modlist_share_window_open = true;
        }

        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        let mut general_only = !console_view.important_only && !console_view.installed_only;
        let general_resp = ui
            .checkbox(&mut general_only, "General")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_GENERAL_OUTPUT);
        if general_resp.changed() && general_only {
            console_view.important_only = false;
            console_view.installed_only = false;
        }
        let important_resp = ui
            .checkbox(&mut console_view.important_only, "Important Only")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_IMPORTANT_ONLY);
        if important_resp.changed() && console_view.important_only {
            console_view.installed_only = false;
        }
        let installed_resp = ui
            .checkbox(&mut console_view.installed_only, "Installed Only")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_INSTALLED_ONLY);
        if installed_resp.changed() && console_view.installed_only {
            console_view.important_only = false;
        }
        ui.checkbox(&mut console_view.auto_scroll, "Auto-scroll")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP5_AUTO_SCROLL);
    });
    crate::ui::step5::content_cancel_step5::render_cancel_confirm(ui, state, terminal);
    render_modlist_share_popup(ui, state);
    action
}

fn render_modlist_share_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    let mut open = state.step5.modlist_share_window_open;
    if !open {
        return;
    }
    egui::Window::new("Export Modlist Share Code")
        .open(&mut open)
        .resizable(true)
        .default_size(egui::vec2(720.0, 360.0))
        .show(ui.ctx(), |ui| {
            if !state.step5.modlist_share_error.trim().is_empty() {
                ui.label(
                    crate::ui::shared::typography_global::plain(&state.step5.modlist_share_error)
                        .color(crate::ui::shared::theme_global::error()),
                );
            }
            ui.label(
                "Share this text with another BIO user. It contains no game files or mod archives.",
            );
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS);
            let text_height = (ui.available_height() - 32.0).max(120.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(text_height)
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), text_height],
                        egui::TextEdit::multiline(&mut state.step5.modlist_share_code)
                            .code_editor(),
                    );
                });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.step5.modlist_share_code.trim().is_empty(),
                        egui::Button::new("Copy"),
                    )
                    .clicked()
                {
                    ui.ctx().copy_text(state.step5.modlist_share_code.clone());
                    state.step5.last_status_text = "Modlist share code copied".to_string();
                }
                if ui.button("Close").clicked() {
                    state.step5.modlist_share_window_open = false;
                }
            });
        });
    state.step5.modlist_share_window_open = open && state.step5.modlist_share_window_open;
}
