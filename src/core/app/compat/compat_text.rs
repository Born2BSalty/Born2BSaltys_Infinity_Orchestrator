// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::io;
use std::path::Path;

pub(super) fn read_compat_text(path: &Path) -> io::Result<String> {
    decode_compat_text(&fs::read(path)?)
}

pub(super) fn decode_compat_text(bytes: &[u8]) -> io::Result<String> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "Unsupported compatibility text");
    if bytes.starts_with(&[0xff, 0xfe, 0, 0]) || bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        return Err(invalid());
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let payload = &bytes[2..];
        if !payload.len().is_multiple_of(2) {
            return Err(invalid());
        }
        let little_endian = bytes[0] == 0xff;
        let units = payload.as_chunks::<2>().0.iter().copied().map(|pair| {
            if little_endian {
                u16::from_le_bytes(pair)
            } else {
                u16::from_be_bytes(pair)
            }
        });
        char::decode_utf16(units)
            .collect::<Result<String, _>>()
            .map_err(|_| invalid())?
    } else {
        let payload = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        std::str::from_utf8(payload).map_or_else(
            |_| payload.iter().copied().map(char::from).collect(),
            str::to_owned,
        )
    };
    Ok(text)
}

#[cfg(test)]
#[path = "compat_text_tests.rs"]
mod tests;
