// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::EmbeddedTerminal;
use std::io::Write;

pub(super) fn write_line_to(writer: &mut dyn Write, line: &str) -> Result<(), String> {
    if line.contains(['\r', '\n', '\0']) {
        return Err("Input must be one line".to_string());
    }
    writer
        .write_all(line.as_bytes())
        .map_err(|error| format!("stdin write failed: {error}"))?;
    writer
        .write_all(b"\r\n")
        .map_err(|error| format!("stdin write failed: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("stdin flush failed: {error}"))
}

fn send_bytes(term_state: &mut EmbeddedTerminal, data: &[u8]) {
    term_state.write_bytes(data);
}

pub(super) fn send_line(term_state: &mut EmbeddedTerminal, line: &str) {
    let mut data = line.as_bytes().to_vec();
    data.extend_from_slice(b"\r\n");
    send_bytes(term_state, &data);
}

pub(super) fn shutdown(term_state: &mut EmbeddedTerminal) {
    send_bytes(term_state, &[0x03]);
    send_line(term_state, "exit");
}
