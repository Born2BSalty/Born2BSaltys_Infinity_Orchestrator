// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::state::WizardState;
use crate::ui::step5::prompt_memory;
use crate::ui::terminal::EmbeddedTerminal;

pub(super) fn render_input(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    terminal: Option<&mut EmbeddedTerminal>,
) {
    ui.add_space(8.0);
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
