// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::super::{EmbeddedTerminal, analyze};

impl EmbeddedTerminal {
    pub fn likely_failure_visible(&self) -> bool {
        analyze::likely_failure_visible(&self.output_buffer)
    }

    pub fn likely_input_needed_visible(&self) -> bool {
        analyze::likely_input_needed_visible(&self.output_buffer)
    }

    pub fn prompt_headers_ready(&self) -> bool {
        analyze::prompt_headers_ready(&self.output_buffer)
    }

    pub fn current_prompt_info(&self) -> Option<analyze::PromptInfo> {
        analyze::current_prompt_info(&self.output_buffer)
    }

    pub fn prompt_kind_name(&self, prompt: &analyze::PromptInfo) -> &'static str {
        analyze::prompt_kind_name(prompt)
    }

    pub fn extract_error_block(&self) -> String {
        analyze::extract_error_block(&self.output_buffer)
    }

    pub fn console_excerpt(&self, max_chars: usize) -> String {
        if self.output_buffer.chars().count() <= max_chars {
            return self.output_buffer.clone();
        }
        let total = self.output_buffer.chars().count();
        let skip = total.saturating_sub(max_chars);
        self.output_buffer.chars().skip(skip).collect()
    }

    pub fn console_text(&self) -> String {
        self.output_buffer.clone()
    }

    pub fn output_len(&self) -> usize {
        self.output_buffer.len()
    }

    pub fn current_scripted_component_key(&self) -> Option<String> {
        self.current_component_key.clone()
    }

    pub fn current_scripted_component_tp2(&self) -> Option<String> {
        self.current_component_tp2.clone()
    }

    pub fn current_scripted_component_id(&self) -> Option<String> {
        self.current_component_id.clone()
    }

    pub fn current_scripted_component_name(&self) -> Option<String> {
        self.current_component_name.clone()
    }

    pub fn scripted_inputs_loaded_count(&self) -> usize {
        self.scripted_inputs_loaded_count
    }

    pub fn boundary_event_count(&self) -> u64 {
        self.boundary_event_count
    }

    pub(in crate::ui::terminal) fn display_text(&self) -> &str {
        if self.show_installed_only {
            &self.installed_buffer
        } else if self.show_important_only {
            &self.important_buffer
        } else {
            &self.output_buffer
        }
    }
}
