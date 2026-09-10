// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step5::install_flow::step3_install_block_reason;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_glyph};
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::action_step5::Step5Action;
use crate::ui::step5::content_step5::Step5RenderCtx;
use crate::ui::step5::state_step5::{ConsoleOutputFilter, Step5ConsoleViewState};

pub(crate) fn render_install_row(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    ctx: Step5RenderCtx<'_>,
) -> Option<Step5Action> {
    let mut action: Option<Step5Action> = None;
    ui.horizontal_wrapped(|ui| {
        render_progress_label(ui, state, ctx.palette);
        action = render_install_control(
            ui,
            state,
            &mut terminal,
            terminal_error,
            ctx.dev_mode,
            ctx.palette,
        );
        render_step5_menus(
            ui,
            state,
            &mut terminal,
            ctx.dev_mode,
            ctx.exe_fingerprint,
            ctx.palette,
        );
        if ctx.dev_mode {
            crate::ui::step5::prompt_answers_step5::render_button(ui, state, ctx.palette);
        }
        render_console_filters(ui, console_view);
    });
    crate::ui::step5::content_cancel_step5::render_cancel_confirm(ui, state, terminal, ctx.palette);
    action
}

fn render_progress_label(ui: &mut egui::Ui, state: &WizardState, palette: ThemePalette) {
    if state.step5.prep_running {
        redesign_btn(
            ui,
            palette,
            "Preparing target dirs...",
            BtnOpts {
                primary: true,
                disabled: true,
                small: true,
                ..Default::default()
            },
        );
    } else if state.step5.install_running {
        redesign_btn(
            ui,
            palette,
            "Install in progress...",
            BtnOpts {
                primary: true,
                disabled: true,
                small: true,
                ..Default::default()
            },
        );
    }
}

fn render_install_control(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: &mut Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    dev_mode: bool,
    palette: ThemePalette,
) -> Option<Step5Action> {
    let can_install = terminal.is_some() && terminal_error.is_none();
    let diagnostics_ready = crate::ui::step5::menus_step5::diagnostics_ready_for_dev(state);
    let install_block_reason = step3_install_block_reason(state);
    let install_allowed = can_install
        && !state.step5.prep_running
        && install_block_reason.is_none()
        && (!dev_mode || diagnostics_ready);

    if state.step5.install_running {
        render_cancel_button(ui, state, can_install, palette);
        return None;
    }

    if state.step5.has_run_once
        && !state.step5.resume_available
        && state.step5.last_exit_code == Some(0)
    {
        let _ = redesign_btn_glyph(
            ui,
            palette,
            "\u{2713}",
            " Installed",
            BtnOpts {
                disabled: true,
                small: true,
                ..Default::default()
            },
        );
        return None;
    }

    let (button_label, is_primary) = install_button_label(state);
    let install_resp = redesign_btn(
        ui,
        palette,
        button_label,
        BtnOpts {
            primary: is_primary,
            disabled: !install_allowed,
            small: true,
            ..Default::default()
        },
    );
    let install_resp = install_hover_response(
        install_resp,
        install_block_reason.as_deref(),
        dev_mode,
        diagnostics_ready,
    );
    let mut action = None;
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
    action
}

fn render_cancel_button(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    can_install: bool,
    palette: ThemePalette,
) {
    if redesign_btn(
        ui,
        palette,
        "Cancel Install",
        BtnOpts {
            danger: true,
            disabled: !can_install,
            small: true,
            ..Default::default()
        },
    )
    .on_hover_text(crate::ui::shared::tooltip_global::STEP5_CANCEL_INSTALL)
    .clicked()
    {
        state.step5.cancel_force_checked = false;
        state.step5.cancel_confirm_open = true;
    }
}

const fn install_button_label(state: &WizardState) -> (&'static str, bool) {
    if state.step5.resume_available {
        ("Resume Install", true)
    } else if state.step5.has_run_once {
        ("Restart Install", true)
    } else {
        ("Install", true)
    }
}

fn install_hover_response(
    response: egui::Response,
    install_block_reason: Option<&str>,
    dev_mode: bool,
    diagnostics_ready: bool,
) -> egui::Response {
    if let Some(reason) = install_block_reason {
        response.on_hover_text(reason)
    } else if dev_mode && !diagnostics_ready {
        response.on_hover_text(crate::ui::shared::tooltip_global::STEP5_DEV_MODE_DIAG_REQUIRED)
    } else {
        response.on_hover_text(crate::ui::shared::tooltip_global::STEP5_START_INSTALL)
    }
}

fn render_step5_menus(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: &mut Option<&mut EmbeddedTerminal>,
    dev_mode: bool,
    exe_fingerprint: &str,
    palette: ThemePalette,
) {
    crate::ui::step5::menus_step5::render_actions_menu(ui, state, terminal.as_deref_mut(), palette);
    crate::ui::step5::menus_step5::render_diagnostics_menu(
        ui,
        state,
        terminal.as_deref(),
        dev_mode,
        exe_fingerprint,
        palette,
    );
}

fn render_console_filters(ui: &mut egui::Ui, console_view: &mut Step5ConsoleViewState) {
    ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
    let mut general_only = console_view.filter == ConsoleOutputFilter::General;
    let general_resp = ui
        .checkbox(&mut general_only, "General")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_GENERAL_OUTPUT);
    if general_resp.changed() && general_only {
        console_view.filter = ConsoleOutputFilter::General;
    }

    let mut important_only = console_view.filter == ConsoleOutputFilter::Important;
    let important_resp = ui
        .checkbox(&mut important_only, "Important Only")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_IMPORTANT_ONLY);
    if important_resp.changed() {
        console_view.filter = if important_only {
            ConsoleOutputFilter::Important
        } else {
            ConsoleOutputFilter::General
        };
    }

    let mut installed_only = console_view.filter == ConsoleOutputFilter::Installed;
    let installed_resp = ui
        .checkbox(&mut installed_only, "Installed Only")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_INSTALLED_ONLY);
    if installed_resp.changed() {
        console_view.filter = if installed_only {
            ConsoleOutputFilter::Installed
        } else {
            ConsoleOutputFilter::General
        };
    }
    ui.checkbox(&mut console_view.auto_scroll, "Auto-scroll")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP5_AUTO_SCROLL);
}
