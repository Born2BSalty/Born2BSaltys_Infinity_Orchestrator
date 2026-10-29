// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::step5::answer_capture::{
    AnswerOrigin, CaptureError, CaptureSession, ConfirmedAnswers, PromptOccurrence, ReplayPlan,
};

use super::{EmbeddedTerminal, input, scripted_inputs};

impl EmbeddedTerminal {
    #[must_use]
    pub const fn has_new_data(&self) -> bool {
        self.events.has_new_data
    }

    pub const fn take_exit_event(&mut self) -> bool {
        let had_exit = self.events.saw_exit_event;
        self.events.saw_exit_event = false;
        had_exit
    }

    pub const fn take_exit_code(&mut self) -> Option<i32> {
        let code = self.last_exit_code;
        self.last_exit_code = None;
        code
    }

    pub fn send_line(&mut self, line: &str) {
        self.log_bio_debug(&format!("send_line=\"{line}\""));
        input::send_line(self, line);
    }

    pub(crate) fn try_send_line(&mut self, line: &str) -> Result<(), String> {
        let result = self
            .stdin
            .as_mut()
            .ok_or_else(|| "Installer stdin is unavailable".to_string())
            .and_then(|stdin| input::write_line_to(stdin, line));
        if let Err(message) = &result {
            self.record_runtime_error(message.clone());
        }
        result
    }
    pub(crate) fn set_capture_session(&mut self, session: CaptureSession) {
        self.capture_session = Some(session);
    }
    pub(crate) fn invalidate_capture(&mut self) {
        if let Some(session) = self.capture_session.as_mut() {
            session.observe_event(crate::app::step5::answer_capture::CaptureEvent::Invalidate);
        }
    }
    pub(crate) fn take_confirmed_answers(&mut self) -> Vec<ConfirmedAnswers> {
        self.capture_session
            .as_mut()
            .map_or_else(Vec::new, CaptureSession::take_confirmed_answers)
    }
    pub(crate) fn take_capture_warnings(&mut self) -> Vec<String> {
        self.capture_session
            .as_mut()
            .map_or_else(Vec::new, CaptureSession::take_warnings)
    }
    pub(crate) fn current_capture_occurrence(&self) -> Option<PromptOccurrence> {
        self.capture_session
            .as_ref()
            .and_then(CaptureSession::current_occurrence)
    }
    pub(crate) fn record_capture_sent(
        &mut self,
        occurrence: &PromptOccurrence,
        value: &str,
        origin: AnswerOrigin,
    ) -> Result<bool, CaptureError> {
        self.capture_session.as_mut().map_or(Ok(false), |session| {
            session.record_sent(occurrence, value, origin)
        })
    }
    pub fn echo_sent(&mut self, line: &str) {
        let shown = if line.is_empty() { "<blank>" } else { line };
        let sent_line = format!("\n[sent] {shown}\n");
        self.append_output(&sent_line);
        self.push_important(sent_line.trim_start_matches('\n'));
        self.prompt_capture.active = false;
        self.prompt_capture.lines = 0;
        self.prompt_capture.after_send = true;
        self.warning_capture.active = false;
        self.warning_capture.lines = 0;
        self.events.has_new_data = true;
    }

    pub fn shutdown(&mut self) {
        input::shutdown(self);
    }

    pub(crate) fn set_scripted_inputs(&mut self, entries: ReplayPlan) -> usize {
        scripted_inputs::set_scripted_inputs(self, entries)
    }

    pub fn take_next_scripted_input_for_current(&mut self) -> Option<String> {
        scripted_inputs::take_next_scripted_input_for_current(self)
    }

    #[must_use]
    pub fn peek_next_scripted_input_for_current(&self) -> Option<&str> {
        scripted_inputs::peek_next_scripted_input_for_current(self)
    }
}
