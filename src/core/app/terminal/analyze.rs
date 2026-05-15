// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod filters {
    use super::prompt::is_prompt_line;
    use super::utils::has_level_token;

    pub(crate) fn likely_failure_visible(output: &str) -> bool {
        let joined = output.to_ascii_uppercase();
        joined.contains("NOT INSTALLED DUE TO ERRORS")
            || joined.contains("ERROR INSTALLING")
            || joined.contains("PARSE ERROR")
            || joined.contains("WEIDU COMMAND FAILED")
    }

    pub(crate) fn important_line(line: &str) -> bool {
        let u = line.to_ascii_uppercase();
        let t = u.trim();
        if t.is_empty() {
            return false;
        }
        if has_level_token(t, "ERROR")
            || has_level_token(t, "FATAL")
            || has_level_token(t, "WARN")
            || has_level_token(t, "WARNING")
            || t.contains("FATAL ERROR")
            || t.contains("NOT INSTALLED DUE TO ERRORS")
            || t.contains("WEIDU COMMAND FAILED")
            || t.contains("PARSE ERROR")
            || t.contains("INSTALLED WITH WARNINGS")
            || t.contains("WITH WARNINGS")
            || t.contains("COMPLETEDWITHWARNINGS")
            || t.contains("WARNING:")
            || t.contains("] WARN ")
            || t.contains("[SENT]")
            || is_prompt_line(t)
        {
            return true;
        }
        false
    }

    pub(crate) fn installed_line(line: &str) -> bool {
        let u = line.to_ascii_uppercase();
        let t = u.trim();
        if t.is_empty() {
            return false;
        }
        t.contains("SUCCESSFULLY INSTALLED") || t.contains("INSTALLED MOD COMPONENT")
    }

    pub(crate) fn parser_timestamp_line(line: &str) -> bool {
        let t = line.trim_start();
        t.starts_with('[') && t.contains("mod_installer::weidu_parser")
    }

    pub(crate) fn warning_capture_start(line: &str) -> bool {
        let t = line.trim_start().to_ascii_uppercase();
        t.contains("WARNING:")
            || t.contains("INSTALLED WITH WARNINGS")
            || t.contains("] WARN ")
            || t.contains("] WARNING ")
    }

    pub(crate) fn warning_capture_end(line: &str) -> bool {
        let t = line.trim_start().to_ascii_uppercase();
        if t.contains("SUCCESSFULLY INSTALLED")
            || t.contains("INSTALLING [")
            || t.contains("INSTALLING MOD COMPONENT")
            || t.contains("USER INPUT REQUIRED")
            || t.contains("QUESTION IS")
        {
            return true;
        }
        if t.contains("] INFO ") && !t.contains("WARNING") && !t.contains("WARN") {
            return true;
        }
        false
    }

    pub(crate) fn extract_error_block(output: &str) -> String {
        let mut out = Vec::new();
        for line in output.lines().rev() {
            let u = line.to_ascii_uppercase();
            if u.contains("ERROR")
                || u.contains("FATAL")
                || u.contains("NOT INSTALLED DUE TO ERRORS")
                || u.contains("PARSE ERROR")
                || u.contains("WEIDU COMMAND FAILED")
            {
                out.push(line.to_string());
                if out.len() >= 30 {
                    break;
                }
            }
        }
        out.reverse();
        if out.is_empty() {
            "No error lines found in current console output.".to_string()
        } else {
            out.join("\n")
        }
    }
}
pub mod model {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum PromptKind {
        YesNo,
        Path,
        Number,
        FreeText,
    }

    pub(crate) struct PromptInfo {
        pub key: String,
        pub legacy_key: Option<String>,
        pub preview_line: String,
        pub kind: PromptKind,
        pub option_count: usize,
        pub line_count: usize,
        pub char_count: usize,
    }
}

#[path = "analyze_prompt_block.rs"]
pub mod prompt_block;
pub mod prompt_detect {
    use super::utils::recent_lines;

    fn has_yes_no_prompt_tokens(line_upper: &str) -> bool {
        line_upper.contains("[Y]ES") && line_upper.contains("[N]O")
    }

    fn likely_proceed_prompt(line_upper: &str) -> bool {
        (line_upper.contains("PROCE") || line_upper.contains("PROCEE"))
            && has_yes_no_prompt_tokens(line_upper)
    }

    pub(super) fn is_prompt_line(line: &str) -> bool {
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
}
pub mod prompt_match {
    use super::model::{PromptInfo, PromptKind};

    pub(crate) const fn prompt_kind_name(prompt: &PromptInfo) -> &'static str {
        match prompt.kind {
            PromptKind::YesNo => "yes/no",
            PromptKind::Path => "path",
            PromptKind::Number => "number",
            PromptKind::FreeText => "text",
        }
    }
}
pub mod prompt {
    pub(crate) use super::prompt_block::current_prompt_info;
    pub(super) use super::prompt_detect::is_prompt_line;
    pub(crate) use super::prompt_detect::likely_input_needed_visible;
    pub(crate) use super::prompt_detect::prompt_capture_end;
    pub(crate) use super::prompt_detect::prompt_capture_start;
    pub(crate) use super::prompt_detect::prompt_headers_ready;
    pub(crate) use super::prompt_match::prompt_kind_name;
}
mod utils {
    pub(super) fn has_level_token(line: &str, token: &str) -> bool {
        let u = line.to_ascii_uppercase();
        let t = token.to_ascii_uppercase();
        if u.contains(&format!(" {t} "))
            || u.contains(&format!("] {t} "))
            || u.ends_with(&format!(" {t}"))
        {
            return true;
        }
        false
    }

    pub(super) fn recent_lines(output: &str, max_lines: usize) -> Vec<&str> {
        let lines: Vec<&str> = output.lines().collect();
        let start = lines.len().saturating_sub(max_lines);
        lines[start..].to_vec()
    }

    pub(super) fn strip_log_prefix(line: &str) -> &str {
        if let Some(idx) = line.find("mod_installer::weidu_parser]") {
            let start = idx + "mod_installer::weidu_parser]".len();
            return line[start..].trim_start();
        }
        if let Some(idx) = line.find("mod_installer::weidu]") {
            let start = idx + "mod_installer::weidu]".len();
            return line[start..].trim_start();
        }
        line
    }

    pub(super) fn normalize_prompt_key(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut prev_space = false;
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !prev_space {
                    out.push(' ');
                }
                prev_space = true;
            } else {
                out.push(ch.to_ascii_uppercase());
                prev_space = false;
            }
        }
        out.trim().to_string()
    }

    pub(super) fn fnv1a64(value: &str) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for &b in value.as_bytes() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }
}

pub(super) use model::PromptInfo;

pub(super) use filters::extract_error_block;
pub(super) use filters::important_line;
pub(super) use filters::installed_line;
pub(super) use filters::likely_failure_visible;
pub(super) use filters::parser_timestamp_line;
pub(super) use filters::warning_capture_end;
pub(super) use filters::warning_capture_start;

pub(super) use prompt::current_prompt_info;
pub(super) use prompt::likely_input_needed_visible;
pub(super) use prompt::prompt_capture_end;
pub(super) use prompt::prompt_capture_start;
pub(super) use prompt::prompt_headers_ready;
pub(super) use prompt::prompt_kind_name;
