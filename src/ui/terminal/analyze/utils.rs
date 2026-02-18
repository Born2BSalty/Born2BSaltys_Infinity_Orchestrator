// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn has_level_token(line: &str, token: &str) -> bool {
    let u = line.to_ascii_uppercase();
    let t = token.to_ascii_uppercase();
    if u.contains(&format!(" {t} ")) || u.contains(&format!("] {t} ")) || u.ends_with(&format!(" {t}")) {
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
    let mut hash = 0xcbf29ce484222325u64;
    for &b in value.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
