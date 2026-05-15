// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod accessors {
    use super::super::{EmbeddedTerminal, analyze};

    impl EmbeddedTerminal {
        #[must_use]
        pub fn likely_failure_visible(&self) -> bool {
            analyze::likely_failure_visible(&self.output_buffer)
        }

        #[must_use]
        pub fn likely_input_needed_visible(&self) -> bool {
            analyze::likely_input_needed_visible(&self.output_buffer)
        }

        #[must_use]
        pub fn prompt_headers_ready(&self) -> bool {
            analyze::prompt_headers_ready(&self.output_buffer)
        }

        pub(crate) fn current_prompt_info(&self) -> Option<analyze::PromptInfo> {
            analyze::current_prompt_info(&self.output_buffer)
        }

        pub(crate) const fn prompt_kind_name(prompt: &analyze::PromptInfo) -> &'static str {
            analyze::prompt_kind_name(prompt)
        }

        #[must_use]
        pub fn extract_error_block(&self) -> String {
            analyze::extract_error_block(&self.output_buffer)
        }

        #[must_use]
        pub fn console_excerpt(&self, max_chars: usize) -> String {
            if self.output_buffer.chars().count() <= max_chars {
                return self.output_buffer.clone();
            }
            let total = self.output_buffer.chars().count();
            let skip = total.saturating_sub(max_chars);
            self.output_buffer.chars().skip(skip).collect()
        }

        #[must_use]
        pub fn console_text(&self) -> String {
            self.output_buffer.clone()
        }

        #[must_use]
        pub fn output_text(&self) -> &str {
            &self.output_buffer
        }

        #[must_use]
        pub fn important_text(&self) -> &str {
            &self.important_buffer
        }

        #[must_use]
        pub fn installed_text(&self) -> &str {
            &self.installed_buffer
        }

        #[must_use]
        pub const fn output_len(&self) -> usize {
            self.output_buffer.len()
        }

        #[must_use]
        pub fn current_scripted_component_key(&self) -> Option<String> {
            self.current_component_key.clone()
        }

        #[must_use]
        pub fn current_scripted_component_tp2(&self) -> Option<String> {
            self.current_component_tp2.clone()
        }

        #[must_use]
        pub fn current_scripted_component_id(&self) -> Option<String> {
            self.current_component_id.clone()
        }

        #[must_use]
        pub fn current_scripted_component_name(&self) -> Option<String> {
            self.current_component_name.clone()
        }

        #[must_use]
        pub const fn scripted_inputs_loaded_count(&self) -> usize {
            self.scripted_inputs_loaded_count
        }

        #[must_use]
        pub const fn boundary_event_count(&self) -> u64 {
            self.boundary_event_count
        }
    }
}
mod buffers {
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

        pub(in crate::app::terminal) fn append_output(&mut self, text: &str) {
            self.output_buffer.push_str(text);
            if self.output_buffer.chars().count() > self.max_buffer_chars {
                let to_trim = self.output_buffer.chars().count() - self.max_buffer_chars;
                let byte_idx = self
                    .output_buffer
                    .char_indices()
                    .nth(to_trim)
                    .map_or(0, |(idx, _)| idx);
                self.output_buffer.drain(..byte_idx);
            }
        }

        pub(in crate::app::terminal) fn update_boundary_events(&mut self, new_text: &str) {
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

        pub(in crate::app::terminal) fn log_bio_debug(&mut self, message: &str) {
            if let Some(file) = self.bio_debug_log_file.as_mut() {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let _ = writeln!(file, "[{ts}] {message}");
                let _ = file.flush();
            }
        }
    }
}
mod capture {
    use super::super::{EmbeddedTerminal, analyze};

    impl EmbeddedTerminal {
        pub(in crate::app::terminal) fn update_important_lines(&mut self, new_text: &str) {
            if new_text.is_empty() {
                return;
            }
            let mut combined = String::new();
            combined.push_str(&self.important_scan_tail);
            combined.push_str(new_text);

            let has_trailing_newline = combined.ends_with('\n');
            let mut parts: Vec<&str> = combined.split('\n').collect();
            if has_trailing_newline {
                self.important_scan_tail.clear();
                if matches!(parts.last(), Some(last) if last.is_empty()) {
                    let _ = parts.pop();
                }
            } else {
                self.important_scan_tail = parts.pop().unwrap_or_default().to_string();
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
                            self.warning_capture_lines =
                                self.warning_capture_lines.saturating_add(1);
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
}
