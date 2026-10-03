// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod accessors {
    use super::super::{EmbeddedTerminal, PromptInfo, analyze, analyze_error_block};

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

        #[must_use]
        pub fn current_prompt_info(&self) -> Option<PromptInfo> {
            analyze::current_prompt_info(&self.output_buffer)
        }

        #[must_use]
        pub const fn prompt_kind_name(&self, prompt: &PromptInfo) -> &'static str {
            analyze::prompt_kind_name(prompt)
        }

        #[must_use]
        pub fn extract_error_block(&self) -> String {
            analyze_error_block::extract_error_block(&self.output_buffer)
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
        pub const fn output_text(&self) -> &str {
            self.output_buffer.as_str()
        }

        #[must_use]
        pub const fn important_text(&self) -> &str {
            self.important_buffer.as_str()
        }

        #[must_use]
        pub const fn installed_text(&self) -> &str {
            self.installed_buffer.as_str()
        }

        #[must_use]
        pub const fn output_len(&self) -> usize {
            self.output_buffer.len()
        }

        #[must_use]
        pub const fn output_revision(&self) -> u64 {
            self.output_revision
        }

        #[must_use]
        pub const fn important_revision(&self) -> u64 {
            self.important_revision
        }

        #[must_use]
        pub const fn installed_revision(&self) -> u64 {
            self.installed_revision
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
            self.current_batch.clear();
            self.pending_display_patches.clear();
            self.prompt_capture.active = false;
            self.prompt_capture.lines = 0;
            self.prompt_capture.after_send = false;
            self.warning_capture.active = false;
            self.warning_capture.lines = 0;
            self.events.has_new_data = true;
            self.output_revision = self.output_revision.wrapping_add(1);
            self.important_revision = self.important_revision.wrapping_add(1);
            self.installed_revision = self.installed_revision.wrapping_add(1);
        }

        pub fn append_marker(&mut self, text: &str) {
            self.append_output(&format!("\n=== {text} ===\n"));
            self.events.has_new_data = true;
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
            self.output_revision = self.output_revision.wrapping_add(1);
        }

        pub(in crate::app::terminal) fn apply_pending_display_patches(&mut self) {
            let mut patched = false;
            for (raw, display) in std::mem::take(&mut self.pending_display_patches)
                .into_iter()
                .rev()
            {
                if let Some(pos) = self.output_buffer.rfind(raw.as_str()) {
                    self.output_buffer
                        .replace_range(pos..pos + raw.len(), &display);
                    patched = true;
                }
            }
            if patched {
                self.output_revision = self.output_revision.wrapping_add(1);
            }
        }

        pub(in crate::app::terminal) fn push_important(&mut self, text: &str) {
            self.important_buffer.push_str(text);
            self.important_revision = self.important_revision.wrapping_add(1);
        }

        pub(in crate::app::terminal) fn push_installed(&mut self, text: &str) {
            self.installed_buffer.push_str(text);
            self.installed_revision = self.installed_revision.wrapping_add(1);
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
    use super::super::success_prefix::{is_batch_line, parse_batch_line, prefixed_success_line};
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
                    if is_batch_line(sub) {
                        self.current_batch = parse_batch_line(sub).unwrap_or_default();
                    }
                    let display = if analyze::installed_line(sub) {
                        prefixed_success_line(sub, &self.current_batch)
                    } else {
                        None
                    };
                    let shown = display.as_deref().unwrap_or(sub);
                    if let Some(display) = display.as_ref() {
                        self.pending_display_patches
                            .push((sub.to_string(), display.clone()));
                    }
                    if analyze::prompt_capture_start(sub) {
                        self.prompt_capture.active = true;
                        self.prompt_capture.lines = 0;
                        self.prompt_capture.after_send = false;
                    }
                    if self.prompt_capture.active {
                        if self.prompt_capture.after_send && analyze::parser_timestamp_line(sub) {
                            self.prompt_capture.active = false;
                            self.prompt_capture.lines = 0;
                            self.prompt_capture.after_send = false;
                        }
                        self.push_important(&format!("{shown}\n"));
                        self.prompt_capture.lines = self.prompt_capture.lines.saturating_add(1);
                        if analyze::prompt_capture_end(sub) || self.prompt_capture.lines >= 5000 {
                            self.prompt_capture.active = false;
                            self.prompt_capture.lines = 0;
                        }
                        continue;
                    }
                    if analyze::warning_capture_start(sub) {
                        self.warning_capture.active = true;
                        self.warning_capture.lines = 0;
                    }
                    if self.warning_capture.active {
                        if analyze::warning_capture_end(sub) || self.warning_capture.lines >= 200 {
                            self.warning_capture.active = false;
                            self.warning_capture.lines = 0;
                        } else {
                            self.push_important(&format!("{shown}\n"));
                            self.warning_capture.lines =
                                self.warning_capture.lines.saturating_add(1);
                            continue;
                        }
                    }
                    if self.prompt_capture.after_send && analyze::parser_timestamp_line(sub) {
                        self.prompt_capture.after_send = false;
                    }
                    if analyze::important_line(sub) {
                        self.push_important(&format!("{shown}\n"));
                    }
                    if analyze::installed_line(sub) {
                        self.push_installed(&format!("{shown}\n"));
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

#[cfg(test)]
mod tests {
    use super::super::EmbeddedTerminal;

    #[test]
    fn output_revision_moves_when_capped_buffer_is_full() {
        let mut term = EmbeddedTerminal::new().expect("terminal");
        term.max_buffer_chars = 10;
        term.append_output("01234567890123456789");
        let revision_before = term.output_revision();
        let len_before = term.output_len();
        term.append_output("abcde");
        assert_eq!(term.output_len(), len_before);
        assert_ne!(term.output_revision(), revision_before);
    }

    #[test]
    fn output_revision_moves_on_clear() {
        let mut term = EmbeddedTerminal::new().expect("terminal");
        term.append_output("hello");
        let output_before = term.output_revision();
        let important_before = term.important_revision();
        let installed_before = term.installed_revision();
        term.clear_console();
        assert_ne!(term.output_revision(), output_before);
        assert_ne!(term.important_revision(), important_before);
        assert_ne!(term.installed_revision(), installed_before);
    }

    #[test]
    fn installed_revision_moves_on_installed_line() {
        let mut term = EmbeddedTerminal::new().expect("terminal");
        let revision_before = term.installed_revision();
        term.update_important_lines("SUCCESSFULLY INSTALLED\n");
        assert_ne!(term.installed_revision(), revision_before);
    }

    #[test]
    fn filtered_revisions_quiet_on_plain_output() {
        let mut term = EmbeddedTerminal::new().expect("terminal");
        let important_before = term.important_revision();
        let installed_before = term.installed_revision();
        term.update_important_lines("just a plain line of output\n");
        term.append_output("just a plain line of output\n");
        let output_before = term.output_revision();
        term.append_output("more plain output\n");
        assert_eq!(term.important_revision(), important_before);
        assert_eq!(term.installed_revision(), installed_before);
        assert_ne!(term.output_revision(), output_before);
    }

    #[test]
    fn clear_console_drops_the_batch_and_pending_patches() {
        let batch_line = "[2026-09-29T03:30:35Z INFO  mod_installer::installers] Installing mod WeiduBatchedComponents([WeiduComponent { tp_file: \"DLCMERGER.TP2\", name: \"DlcMerger\", lang: \"0\", component: \"1\", component_name: \"Merge DLC into game\", sub_component: \"Siege of Dragonspear\", version: \"1.8\" }])\n";
        let success_line = "[2026-09-29T03:31:14Z INFO  mod_installer::parser] SUCCESSFULLY INSTALLED      Merge DLC into game\n";
        let mut term = EmbeddedTerminal::new().expect("terminal");
        term.update_important_lines(batch_line);
        term.update_important_lines(success_line);
        assert_ne!(term.current_batch.len(), 0);
        assert_ne!(term.pending_display_patches.len(), 0);
        term.clear_console();
        assert_eq!(term.current_batch.len(), 0);
        assert_eq!(term.pending_display_patches.len(), 0);
        term.append_output(success_line);
        term.apply_pending_display_patches();
        assert_eq!(term.output_text(), success_line);
    }
}
