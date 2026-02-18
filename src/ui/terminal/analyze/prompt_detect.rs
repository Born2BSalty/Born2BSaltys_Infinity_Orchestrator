// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::utils::recent_lines;

fn has_yes_no_prompt_tokens(line_upper: &str) -> bool {
    line_upper.contains("[Y]ES") && line_upper.contains("[N]O")
}

fn likely_proceed_prompt(line_upper: &str) -> bool {
    (line_upper.contains("PROCE") || line_upper.contains("PROCEE"))
        && has_yes_no_prompt_tokens(line_upper)
}

pub(crate) fn is_prompt_line(line: &str) -> bool {
    let u = line.to_ascii_uppercase();
    u.contains("USER INPUT REQUIRED")
        || u.contains("PLEASE CHOOSE")
        || u.contains("IS THIS CORRECT?")
        || u.contains("[Y]ES OR [N]O")
        || has_yes_no_prompt_tokens(&u)
        || likely_proceed_prompt(&u)
}

pub(crate) fn prompt_capture_start(line: &str) -> bool {
    let u = line.to_ascii_uppercase();
    u.contains("USER INPUT REQUIRED") || u.contains("QUESTION IS")
}

pub(crate) fn prompt_capture_end(line: &str) -> bool {
    let u = line.to_ascii_uppercase();
    u.contains("[SENT]")
        || u.contains("SUCCESSFULLY INSTALLED")
        || u.contains("INSTALLING MOD COMPONENT")
        || u.contains("INSTALLED MOD COMPONENT")
        || u.contains("=== RUN #")
        || u.contains("=== FORCE TERMINATE REQUESTED ===")
        || u.contains("=== GRACEFUL TERMINATE REQUESTED ===")
}

pub(crate) fn likely_input_needed_visible(output: &str) -> bool {
    recent_lines(output, 40)
        .iter()
        .any(|line| is_prompt_line(line))
}

pub(crate) fn prompt_headers_ready(output: &str) -> bool {
    let mut has_input_required = false;
    let mut has_question_is = false;
    for line in recent_lines(output, 120) {
        let u = line.to_ascii_uppercase();
        if u.contains("USER INPUT REQUIRED") {
            has_input_required = true;
        }
        if u.contains("QUESTION IS") {
            has_question_is = true;
        }
    }
    has_input_required && has_question_is
}
