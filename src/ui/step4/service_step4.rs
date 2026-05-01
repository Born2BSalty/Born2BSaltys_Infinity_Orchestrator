// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;
use std::{fs, io};

pub fn read_source_log_lines(path: &Path) -> io::Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().map(|line| line.to_string()).collect())
}
