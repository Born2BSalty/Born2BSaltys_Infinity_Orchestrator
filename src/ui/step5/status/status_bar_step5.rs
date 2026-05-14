// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub(crate) fn render_console(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
    palette: ThemePalette,
) {
    crate::ui::step5::status_console_step5::render_console_panel(
        ui,
        state,
        console_view,
        terminal,
        terminal_error,
        palette,
    );
}

pub(crate) fn render_status_and_input(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
    mut terminal: Option<&mut EmbeddedTerminal>,
    palette: ThemePalette,
) {
    let waiting_for_input_before = terminal
        .as_deref()
        .map(|t| t.likely_input_needed_visible() || t.current_prompt_info().is_some())
        .unwrap_or(false);
    let prev_auto_key = state.step5.last_auto_prompt_key.clone();
    let prev_scripted_cycle = state.step5.last_scripted_cycle_signature.clone();
    let prev_scripted_send_ms = state.step5.last_scripted_send_unix_ms;

    crate::app::step5::auto_answer::try_auto_answer_prompt(
        state,
        terminal.as_deref_mut(),
        waiting_for_input_before,
    );

    let waiting_for_input_after = terminal
        .as_deref()
        .map(|t| t.likely_input_needed_visible() || t.current_prompt_info().is_some())
        .unwrap_or(false);
    let auto_answer_sent_this_tick = state.step5.last_auto_prompt_key != prev_auto_key
        || state.step5.last_scripted_cycle_signature != prev_scripted_cycle
        || state.step5.last_scripted_send_unix_ms != prev_scripted_send_ms;

    if state.step1.prompt_required_sound_enabled {
        if waiting_for_input_after
            && !auto_answer_sent_this_tick
            && !state.step5.prompt_required_sound_latched
        {
            crate::ui::step5::service_sound_step5::play_prompt_required_sound_once();
            state.step5.prompt_required_sound_latched = true;
        } else if !waiting_for_input_after {
            state.step5.prompt_required_sound_latched = false;
        }
    } else {
        state.step5.prompt_required_sound_latched = false;
    }

    let phase_info = crate::ui::step5::status_phase_step5::compute_phase(
        state,
        waiting_for_input_after,
        palette,
    );

    ui.horizontal(|ui| {
        crate::ui::step5::status_input_row_step5::render_input_row(
            ui,
            state,
            console_view,
            terminal.as_deref_mut(),
        );
    });

    ui.horizontal_centered(|ui| {
        crate::ui::step5::status_phase_step5::render_phase(ui, state, &phase_info, palette);
        crate::ui::step5::service_process_line_step5::render_process_runtime_inline(
            ui,
            state,
            terminal.as_deref(),
        );
        crate::ui::step5::service_process_line_step5::render_error_copy(
            ui,
            state,
            terminal.as_deref(),
        );
    });

    if !state.step5.last_status_text.trim().is_empty() {
        let status = state.step5.last_status_text.clone();
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_XS * 0.5);
        ui.horizontal(|ui| {
            let width = ui.available_width().max(320.0);
            ui.add_sized(
                [width, 0.0],
                egui::Label::new(crate::ui::shared::typography_global::weak(status)),
            );
        });
    }
}
