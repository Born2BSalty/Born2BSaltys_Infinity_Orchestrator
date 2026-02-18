// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use eframe::egui;

use super::{EmbeddedTerminal, input, scripted_inputs, view};

impl EmbeddedTerminal {
    pub fn has_new_data(&self) -> bool {
        self.has_new_data
    }

    pub fn take_exit_event(&mut self) -> bool {
        let had_exit = self.saw_exit_event;
        self.saw_exit_event = false;
        had_exit
    }

    pub fn take_exit_code(&mut self) -> Option<i32> {
        let code = self.last_exit_code;
        self.last_exit_code = None;
        code
    }

    pub fn render(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        view::render(self, ui, size);
    }

    pub fn set_show_important_only(&mut self, enabled: bool) {
        self.show_important_only = enabled;
    }

    pub fn set_show_installed_only(&mut self, enabled: bool) {
        self.show_installed_only = enabled;
    }

    pub fn set_auto_scroll(&mut self, enabled: bool) {
        self.stick_to_bottom = enabled;
    }

    pub fn send_line(&mut self, line: &str) {
        self.log_bio_debug(&format!("send_line=\"{}\"", line));
        input::send_line(self, line);
    }

    pub fn echo_sent(&mut self, line: &str) {
        let shown = if line.is_empty() { "<blank>" } else { line };
        let sent_line = format!("\n[sent] {shown}\n");
        self.append_output(&sent_line);
        self.important_buffer.push_str(sent_line.trim_start_matches('\n'));
        self.prompt_capture_active = false;
        self.prompt_capture_lines = 0;
        self.prompt_capture_after_send = true;
        self.warning_capture_active = false;
        self.warning_capture_lines = 0;
        self.has_new_data = true;
    }

    pub fn focus(&mut self) {
        input::focus(self);
    }

    pub fn shutdown(&mut self) {
        input::shutdown(self);
    }

    pub fn set_scripted_inputs(&mut self, entries: HashMap<String, Vec<String>>) -> usize {
        scripted_inputs::set_scripted_inputs(self, entries)
    }

    pub fn take_next_scripted_input_for_current(&mut self) -> Option<String> {
        scripted_inputs::take_next_scripted_input_for_current(self)
    }

    pub fn peek_next_scripted_input_for_current(&self) -> Option<&str> {
        scripted_inputs::peek_next_scripted_input_for_current(self)
    }
}
