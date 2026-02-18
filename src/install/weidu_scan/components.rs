// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use anyhow::{Result, anyhow};

use crate::mods::component::Component;

use super::command::run_weidu;

pub(super) fn list_components(
    weidu_binary: &Path,
    game_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<Component>> {
    let args = vec![
        "--game".to_string(),
        game_directory.display().to_string(),
        "--list-components".to_string(),
        tp2_path.display().to_string(),
        language.to_string(),
    ];
    let output = run_weidu(
        weidu_binary,
        &args,
        Some(game_directory),
    )?;

    if !output.status.success() {
        return Err(anyhow!(
            "weidu --list-components failed for {}\nstdout:\n{}\nstderr:\n{}",
            tp2_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| l.starts_with('~')) {
        out.push(Component::parse_weidu_line(line)?);
    }
    Ok(out)
}

pub(super) fn list_components_lines(
    weidu_binary: &Path,
    game_directory: &Path,
    working_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<String>> {
    let args = vec![
        "--game".to_string(),
        game_directory.display().to_string(),
        "--list-components".to_string(),
        tp2_path.display().to_string(),
        language.to_string(),
    ];
    let output = run_weidu(
        weidu_binary,
        &args,
        Some(working_directory),
    )?;

    if !output.status.success() {
        return Err(anyhow!(
            "weidu --list-components failed for {}\nstdout:\n{}\nstderr:\n{}",
            tp2_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Keep all non-empty lines and let parse_component_line decide.
        // Over-filtering here can drop valid WeiDU formats for some mods.
        out.push(trimmed.to_string());
    }
    Ok(out)
}
