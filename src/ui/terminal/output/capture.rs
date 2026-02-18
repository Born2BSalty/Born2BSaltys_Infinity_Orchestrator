// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::{EmbeddedTerminal, analyze};

impl EmbeddedTerminal {
    pub(in crate::ui::terminal) fn update_important_lines(&mut self, new_text: &str) {
        if new_text.is_empty() {
            return;
        }
        let mut combined = String::new();
        combined.push_str(&self.important_scan_tail);
        combined.push_str(new_text);

        let has_trailing_newline = combined.ends_with('\n');
        let mut parts: Vec<&str> = combined.split('\n').collect();
        if !has_trailing_newline {
            self.important_scan_tail = parts.pop().unwrap_or_default().to_string();
        } else {
            self.important_scan_tail.clear();
            if matches!(parts.last(), Some(last) if last.is_empty()) {
                let _ = parts.pop();
            }
        }

        for line in parts {
            let expanded = expand_escaped_newlines(line);
            for sub in expanded.lines() {
                if analyze::prompt_capture_start(sub) {
                    self.prompt_capture_active = true;
                    self.prompt_capture_lines = 0;
                    self.prompt_capture_after_send = false;
                }
                if self.prompt_capture_active {
                    if self.prompt_capture_after_send && analyze::parser_timestamp_line(sub) {
                        self.prompt_capture_active = false;
                        self.prompt_capture_lines = 0;
                        self.prompt_capture_after_send = false;
                    }
                    self.important_buffer.push_str(sub);
                    self.important_buffer.push('\n');
                    self.prompt_capture_lines = self.prompt_capture_lines.saturating_add(1);
                    if analyze::prompt_capture_end(sub) || self.prompt_capture_lines >= 5000 {
                        self.prompt_capture_active = false;
                        self.prompt_capture_lines = 0;
                    }
                    continue;
                }
                if analyze::warning_capture_start(sub) {
                    self.warning_capture_active = true;
                    self.warning_capture_lines = 0;
                }
                if self.warning_capture_active {
                    if analyze::warning_capture_end(sub) || self.warning_capture_lines >= 200 {
                        self.warning_capture_active = false;
                        self.warning_capture_lines = 0;
                    } else {
                        self.important_buffer.push_str(sub);
                        self.important_buffer.push('\n');
                        self.warning_capture_lines = self.warning_capture_lines.saturating_add(1);
                        continue;
                    }
                }
                if self.prompt_capture_after_send && analyze::parser_timestamp_line(sub) {
                    self.prompt_capture_after_send = false;
                }
                if analyze::important_line(sub) {
                    self.important_buffer.push_str(sub);
                    self.important_buffer.push('\n');
                }
                if analyze::installed_line(sub) {
                    self.installed_buffer.push_str(sub);
                    self.installed_buffer.push('\n');
                }
            }
        }
    }
}

fn expand_escaped_newlines(value: &str) -> String {
    value
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\\r", "\n")
}
