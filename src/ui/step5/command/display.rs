// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step1State;

use super::invocation::build_install_command;

pub fn build_command_preview_lines(step1: &Step1State) -> Vec<String> {
    build_install_command(step1)
        .split(" --")
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else {
                format!("--{part}")
            }
        })
        .collect()
}

pub fn wrap_display_line(line: &str, max_cols: usize) -> Vec<String> {
    if line.chars().count() <= max_cols {
        return vec![line.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for token in line.split_whitespace() {
        if current.is_empty() {
            current.push_str(token);
            continue;
        }
        if current.chars().count() + 1 + token.chars().count() <= max_cols {
            current.push(' ');
            current.push_str(token);
        } else {
            out.push(current);
            current = token.to_string();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}
