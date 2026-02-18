// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
