// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn render_console(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
    terminal_error: Option<&str>,
) {
    console::render_console(ui, state, terminal, terminal_error);
}

pub(crate) fn render_status_and_input(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    mut terminal: Option<&mut EmbeddedTerminal>,
) {
    let waiting_for_input_before = terminal
        .as_deref()
        .map(|t| t.likely_input_needed_visible() || t.current_prompt_info().is_some())
        .unwrap_or(false);
    let prev_auto_key = state.step5.last_auto_prompt_key.clone();
    let prev_scripted_cycle = state.step5.last_scripted_cycle_signature.clone();
    let prev_scripted_send_ms = state.step5.last_scripted_send_unix_ms;

    crate::ui::step5::service_step5::auto_answer::try_auto_answer_prompt(
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
            crate::ui::step5::service_step5::play_prompt_required_sound_once();
            state.step5.prompt_required_sound_latched = true;
        } else if !waiting_for_input_after {
            state.step5.prompt_required_sound_latched = false;
        }
    } else {
        state.step5.prompt_required_sound_latched = false;
    }

    let phase_info = phase::compute_phase(state, waiting_for_input_after);

    // Chat bar directly under console frame.
    ui.horizontal(|ui| {
        input_row::render_input(ui, state, terminal.as_deref_mut());
    });

    // Single-line phase + runtime row.
    ui.horizontal_centered(|ui| {
        phase::render_phase(ui, state, &phase_info);
        crate::ui::step5::service_step5::process_line::render_process_runtime_inline(
            ui,
            state,
            terminal.as_deref(),
        );
        crate::ui::step5::service_step5::process_line::render_error_copy(
            ui,
            state,
            terminal.as_deref(),
        );
    });

    // Show detailed status text in its own wrapped row so it never pushes controls outside layout.
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

mod console {
    use eframe::egui;

    use crate::ui::state::WizardState;
    use crate::ui::terminal::EmbeddedTerminal;

    pub(crate) fn render_console(
        ui: &mut egui::Ui,
        state: &mut WizardState,
        terminal: Option<&mut EmbeddedTerminal>,
        terminal_error: Option<&str>,
    ) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(crate::ui::shared::typography_global::section_title(
                "Console",
            ));
            ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_SM);
            let console_w = ui.available_width();
            let reserved_for_input = 56.0;
            let console_h = (ui.available_height() - reserved_for_input).max(220.0);
            if let Some(term) = terminal {
                term.set_show_important_only(state.step5.important_only);
                term.set_show_installed_only(state.step5.installed_only);
                term.set_auto_scroll(state.step5.auto_scroll);
                term.render(ui, egui::vec2(console_w, console_h));
            } else {
                ui.add_sized(
                    [console_w, console_h],
                    egui::Label::new(terminal_error.unwrap_or("Initializing terminal...")),
                );
            }
        });
    }
}

mod input_row {
    use eframe::egui;

    use crate::ui::state::WizardState;
    use crate::ui::step5::prompt_memory;
    use crate::ui::terminal::EmbeddedTerminal;

    pub(super) fn render_input(
        ui: &mut egui::Ui,
        state: &mut WizardState,
        terminal: Option<&mut EmbeddedTerminal>,
    ) {
        ui.add_space(crate::ui::shared::layout_tokens_global::SPACE_MD);
        let can_send = terminal.is_some();
        let input_w = (ui.available_width() - 250.0).max(240.0);
        let input = ui
            .add_enabled_ui(can_send, |ui| {
                ui.add_sized(
                    [input_w, 27.0],
                    egui::TextEdit::singleline(&mut state.step5.input_line)
                        .hint_text("Type response (e.g. Y, N, 1) and press Enter"),
                )
            })
            .inner;

        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
        let send_now = can_send && enter_pressed && (input.has_focus() || input.lost_focus());

        let mut keep_input_focus = false;
        if send_now {
            if let Some(term) = terminal {
                let reply = state.step5.input_line.clone();
                if let Some(prompt_info) = term.current_prompt_info() {
                    let ctx = prompt_memory::PromptAnswerContext {
                        component_key: term.current_scripted_component_key().unwrap_or_default(),
                        tp2_file: term.current_scripted_component_tp2().unwrap_or_default(),
                        component_id: term.current_scripted_component_id().unwrap_or_default(),
                        component_name: term.current_scripted_component_name().unwrap_or_default(),
                        prompt_kind: term.prompt_kind_name(&prompt_info).to_string(),
                        source: "manual".to_string(),
                    };
                    prompt_memory::remember_answer_with_context(
                        &prompt_info.key,
                        &reply,
                        &prompt_info.preview_line,
                        &ctx,
                    );
                    state.step5.last_auto_prompt_key = Some(prompt_info.key);
                }
                term.send_line(&reply);
                term.echo_sent(&reply);
                term.focus();
            }
            state.step5.input_line.clear();
            keep_input_focus = true;
        }
        if keep_input_focus {
            input.request_focus();
        }
    }
}

mod phase {
    use eframe::egui;

    use crate::ui::state::WizardState;

    pub(super) struct PhaseInfo {
        pub label: &'static str,
        pub color: egui::Color32,
    }

    pub(super) fn compute_phase(state: &WizardState, waiting_for_input: bool) -> PhaseInfo {
        if state.step5.install_running {
            if state.step5.cancel_pending {
                return PhaseInfo {
                    label: "Cancelling",
                    color: crate::ui::shared::theme_global::warning(),
                };
            }
            if waiting_for_input {
                return PhaseInfo {
                    label: "Waiting Input",
                    color: crate::ui::shared::theme_global::accent_path(),
                };
            }
            return PhaseInfo {
                label: "Running",
                color: crate::ui::shared::theme_global::status_running(),
            };
        }
        if state.step5.last_status_text.starts_with("Preflight")
            || state.step5.last_status_text.starts_with("Target prep")
            || state.step5.last_status_text.starts_with("Backup target")
        {
            return PhaseInfo {
                label: "Preparing",
                color: crate::ui::shared::theme_global::status_preparing(),
            };
        }
        if state.step5.has_run_once {
            return PhaseInfo {
                label: "Finished",
                color: crate::ui::shared::theme_global::text_muted(),
            };
        }
        PhaseInfo {
            label: "Idle",
            color: crate::ui::shared::theme_global::status_idle(),
        }
    }

    pub(super) fn render_phase(ui: &mut egui::Ui, state: &WizardState, phase: &PhaseInfo) {
        let phase_state = if state.step5.cancel_pending {
            "Pending".to_string()
        } else {
            phase.label.to_string()
        };
        let status_tooltip = if !state.step5.last_status_text.is_empty()
            && state.step5.last_status_text != phase.label
            && state.step5.last_status_text != "Running"
            && state.step5.last_status_text != "Idle"
        {
            Some(state.step5.last_status_text.clone())
        } else {
            None
        };
        let phase_text = format!("Phase: {phase_state}");
        let phase_resp = ui.add(
            egui::Label::new(
                crate::ui::shared::typography_global::strong(phase_text).color(phase.color),
            )
            .wrap_mode(egui::TextWrapMode::Extend),
        );
        if let Some(tip) = status_tooltip.as_deref() {
            phase_resp.on_hover_text(tip);
        }
    }
}
