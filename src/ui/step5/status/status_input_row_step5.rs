// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::terminal::EmbeddedTerminal;
use crate::ui::step5::state_step5::Step5ConsoleViewState;

pub(crate) fn render_input_row(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    console_view: &mut Step5ConsoleViewState,
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

    let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));
    let send_now = can_send && enter_pressed && (input.has_focus() || input.lost_focus());
    let should_focus_input = std::mem::take(&mut console_view.request_input_focus);

    let keep_input_focus = if send_now {
        if let Some(term) = terminal {
            let reply = state.step5.input_line.clone();
            crate::app::step5::auto_answer::send_manual_input(state, term, &reply);
        }
        state.step5.input_line.clear();
        true
    } else {
        false
    };
    if can_send && (keep_input_focus || should_focus_input) {
        input.request_focus();
    }
}
