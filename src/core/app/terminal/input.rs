// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::EmbeddedTerminal;

fn send_bytes(term_state: &mut EmbeddedTerminal, data: Vec<u8>) {
    term_state.write_bytes(&data);
}

pub(super) fn send_line(term_state: &mut EmbeddedTerminal, line: &str) {
    let mut data = line.as_bytes().to_vec();
    data.extend_from_slice(b"\r\n");
    send_bytes(term_state, data);
}

pub(super) fn focus(term_state: &mut EmbeddedTerminal) {
    term_state.request_focus = true;
    term_state.active = true;
}

pub(super) fn shutdown(term_state: &mut EmbeddedTerminal) {
    send_bytes(term_state, vec![0x03]);
    send_line(term_state, "exit");
}
