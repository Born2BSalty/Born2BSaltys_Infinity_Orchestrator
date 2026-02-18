// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use anyhow::{Result, anyhow};

use super::LanguageEntry;
use super::command::run_weidu;

pub(super) fn list_languages(weidu_binary: &Path, tp2_path: &Path) -> Result<Vec<LanguageEntry>> {
    let args = vec![
        "--nogame".to_string(),
        "--list-languages".to_string(),
        tp2_path.display().to_string(),
    ];
    let output = run_weidu(
        weidu_binary,
        &args,
        None,
    )?;
    if !output.status.success() {
        return Err(anyhow!(
            "weidu --list-languages failed for {}\nstdout:\n{}\nstderr:\n{}",
            tp2_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(parse_languages(&String::from_utf8_lossy(&output.stdout)))
}

pub(super) fn list_languages_for_game(
    weidu_binary: &Path,
    tp2_path: &Path,
    game_directory: &Path,
    working_directory: &Path,
) -> Result<Vec<LanguageEntry>> {
    let args = vec![
        "--game".to_string(),
        game_directory.display().to_string(),
        "--list-languages".to_string(),
        tp2_path.display().to_string(),
    ];
    let output = run_weidu(
        weidu_binary,
        &args,
        Some(working_directory),
    )?;
    if !output.status.success() {
        return Err(anyhow!(
            "weidu --list-languages failed for {}\nstdout:\n{}\nstderr:\n{}",
            tp2_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(parse_languages(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_languages(text: &str) -> Vec<LanguageEntry> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (num, rest) = trimmed.split_once(':')?;
            if num.chars().all(|c| c.is_ascii_digit()) {
                Some(LanguageEntry {
                    id: num.to_string(),
                    label: rest.trim().to_string(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
