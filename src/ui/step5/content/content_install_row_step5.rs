// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step5::install_flow::step3_install_block_reason;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_accent_path, redesign_error};
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::page_step5::{Step5RenderOptions, Step5RenderRuntime};

pub(crate) fn render_install_row(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: &mut Step5RenderRuntime<'_>,
    options: Step5RenderOptions<'_>,
) -> Option<Step5Action> {
    let terminal_error = runtime.terminal_error;
    let dev_mode = options.dev_mode;
    let exe_fingerprint = options.exe_fingerprint;
    let palette = options.palette;
    let mut action: Option<Step5Action> = None;
    ui.horizontal(|ui| {
        let can_install = runtime.terminal.is_some() && terminal_error.is_none();
        let diagnostics_ready = crate::ui::step5::menus_step5::diagnostics_ready_for_dev(state);
        let install_block_reason = step3_install_block_reason(state);
        let install_allowed = can_install
            && !state.step5.prep_running
            && install_block_reason.is_none()
            && (!dev_mode || diagnostics_ready);

        render_install_status_label(ui, state, palette);

        if state.step5.install_running {
            render_cancel_button(ui, state, can_install);
        } else {
            action = render_start_button(
                ui,
                state,
                runtime,
                install_allowed,
                install_block_reason,
                dev_mode,
                diagnostics_ready,
            );
        }

        render_step5_menus(ui, state, runtime, dev_mode, exe_fingerprint);

        render_console_filters(ui, runtime);
    });
    crate::ui::step5::content_cancel_step5::render_cancel_confirm(
        ui,
        state,
        runtime.terminal.as_deref_mut(),
        palette,
    );
    render_modlist_share_popup(ui, state, palette);
    action
}

fn render_install_status_label(ui: &mut egui::Ui, state: &WizardState, palette: ThemePalette) {
    if state.step5.prep_running {
        ui.label(
            crate::ui::shared::typography_global::strong("Preparing target dirs...")
                .color(redesign_accent_path(palette)),
        );
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
    } else if state.step5.install_running {
        ui.label(
            crate::ui::shared::typography_global::strong("Install in progress...")
                .color(redesign_accent_path(palette)),
        );
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
    }
}

const fn install_button_size() -> egui::Vec2 {
    egui::vec2(
        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_W,
        crate::ui::shared::layout_tokens_global::STEP5_INSTALL_BTN_H,
    )
}

fn render_cancel_button(ui: &mut egui::Ui, state: &mut WizardState, can_install: bool) {
    if ui
        .add_enabled(
            can_install,
            egui::Button::new("Cancel Install").min_size(install_button_size()),
        )
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_CANCEL_INSTALL)
        .clicked()
    {
        state.step5.cancel_force_checked = false;
        state.step5.cancel_confirm_open = true;
    }
}

fn render_start_button(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: &mut Step5RenderRuntime<'_>,
    install_allowed: bool,
    install_block_reason: Option<String>,
    dev_mode: bool,
    diagnostics_ready: bool,
) -> Option<Step5Action> {
    let install_resp = ui.add_enabled(
        install_allowed,
        egui::Button::new(install_button_label(state)).min_size(install_button_size()),
    );
    let install_resp = if let Some(reason) = install_block_reason.as_deref() {
        install_resp.on_hover_text(reason)
    } else if dev_mode && !diagnostics_ready {
        install_resp.on_hover_text(crate::ui::shared::tooltip_global::STEP5_DEV_MODE_DIAG_REQUIRED)
    } else {
        install_resp.on_hover_text(crate::ui::shared::tooltip_global::STEP5_START_INSTALL)
    };
    let action = if install_resp.clicked() {
        handle_install_click(state, runtime, dev_mode, diagnostics_ready)
    } else {
        None
    };
    if let Some(reason) = install_block_reason {
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        ui.label(crate::ui::shared::typography_global::weak(reason));
    }
    action
}

const fn install_button_label(state: &WizardState) -> &'static str {
    if state.step5.resume_available {
        "Resume Install"
    } else if state.step5.has_run_once {
        "Restart Install"
    } else {
        "Install"
    }
}

fn handle_install_click(
    state: &mut WizardState,
    runtime: &mut Step5RenderRuntime<'_>,
    dev_mode: bool,
    diagnostics_ready: bool,
) -> Option<Step5Action> {
    if dev_mode && !diagnostics_ready {
        state.step5.last_status_text =
            "Dev mode install blocked: enable diagnostics (Full Debug + Raw Output + RUST_LOG DEBUG/TRACE)."
                .to_string();
        if let Some(term) = runtime.terminal.as_deref_mut() {
            term.append_marker(
                "Dev mode install blocked: enable diagnostics (Full Debug + Raw Output + RUST_LOG DEBUG/TRACE).",
            );
        }
        None
    } else {
        Some(Step5Action::StartInstall)
    }
}

fn render_step5_menus(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    runtime: &mut Step5RenderRuntime<'_>,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    crate::ui::step5::menus_step5::render_actions_menu(ui, state, runtime.terminal.as_deref_mut());
    crate::ui::step5::menus_step5::render_diagnostics_menu(
        ui,
        state,
        runtime.terminal.as_deref(),
        dev_mode,
        exe_fingerprint,
    );
    crate::ui::step5::prompt_answers_step5::render_button(ui, state);
    render_modlist_export_button(ui, state);
}

fn render_modlist_export_button(ui: &mut egui::Ui, state: &mut WizardState) {
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
}

fn render_console_filters(ui: &mut egui::Ui, runtime: &mut Step5RenderRuntime<'_>) {
    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
    let mut general_only =
        !runtime.console_view.important_only && !runtime.console_view.installed_only;
    let general_resp = ui
        .checkbox(&mut general_only, "General")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_GENERAL_OUTPUT);
    if general_resp.changed() && general_only {
        runtime.console_view.important_only = false;
        runtime.console_view.installed_only = false;
    }
    let important_resp = ui
        .checkbox(&mut runtime.console_view.important_only, "Important Only")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_IMPORTANT_ONLY);
    if important_resp.changed() && runtime.console_view.important_only {
        runtime.console_view.installed_only = false;
    }
    let installed_resp = ui
        .checkbox(&mut runtime.console_view.installed_only, "Installed Only")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_INSTALLED_ONLY);
    if installed_resp.changed() && runtime.console_view.installed_only {
        runtime.console_view.important_only = false;
    }
    ui.checkbox(&mut runtime.console_view.auto_scroll, "Auto-scroll")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_AUTO_SCROLL);
}

fn render_modlist_share_popup(ui: &egui::Ui, state: &mut WizardState, palette: ThemePalette) {
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
                        .color(redesign_error(palette)),
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
