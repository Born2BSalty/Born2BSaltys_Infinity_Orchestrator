// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::EmbeddedTerminal;

impl EmbeddedTerminal {
    pub fn clear_console(&mut self) {
        self.output_buffer.clear();
        self.important_buffer.clear();
        self.installed_buffer.clear();
        self.important_scan_tail.clear();
        self.prompt_capture_active = false;
        self.prompt_capture_lines = 0;
        self.prompt_capture_after_send = false;
        self.warning_capture_active = false;
        self.warning_capture_lines = 0;
        self.has_new_data = true;
    }

    pub fn append_marker(&mut self, text: &str) {
        self.append_output(&format!("\n=== {text} ===\n"));
        self.has_new_data = true;
    }

    pub(in crate::ui::terminal) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
        if self.output_buffer.chars().count() > self.max_buffer_chars {
            let to_trim = self.output_buffer.chars().count() - self.max_buffer_chars;
            let byte_idx = self
                .output_buffer
                .char_indices()
                .nth(to_trim)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.output_buffer.drain(..byte_idx);
        }
    }

    pub(in crate::ui::terminal) fn update_boundary_events(&mut self, new_text: &str) {
        if new_text.is_empty() {
            return;
        }
        let mut combined = String::new();
        combined.push_str(&self.boundary_scan_tail);
        combined.push_str(new_text);
        let upper = combined.to_ascii_uppercase();
        let needle = "SUCCESSFULLY INSTALLED";
        let count = upper.match_indices(needle).count() as u64;
        if count > 0 {
            self.boundary_event_count = self.boundary_event_count.saturating_add(count);
        }
        let keep_chars = 96usize;
        let total_chars = combined.chars().count();
        if total_chars <= keep_chars {
            self.boundary_scan_tail = combined;
        } else {
            let skip = total_chars - keep_chars;
            self.boundary_scan_tail = combined.chars().skip(skip).collect();
        }
    }

    pub(in crate::ui::terminal) fn log_bio_debug(&mut self, message: &str) {
        if let Some(file) = self.bio_debug_log_file.as_mut() {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{ts}] {message}");
            let _ = file.flush();
        }
    }
}
