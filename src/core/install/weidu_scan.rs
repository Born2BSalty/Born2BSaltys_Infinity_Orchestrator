// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

mod command {
    use std::path::Path;
    use std::process::{Command, Output, Stdio};

    use anyhow::{Context, Result};

    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;

    #[cfg(target_os = "windows")]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    fn hidden_command(program: &Path) -> Command {
        let mut cmd = Command::new(program);
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }

    pub(super) fn run_weidu(
        weidu_binary: &Path,
        args: &[String],
        working_directory: Option<&Path>,
    ) -> Result<Output> {
        let mut cmd = hidden_command(weidu_binary);
        if let Some(wd) = working_directory {
            cmd.current_dir(wd);
        }
        for arg in args {
            cmd.arg(arg);
        }
        cmd.arg("--no-exit-pause")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to run weidu: {}", weidu_binary.display()))
    }
}
mod components {
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
        let output = run_weidu(weidu_binary, &args, Some(game_directory))?;

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
        let output = run_weidu(weidu_binary, &args, Some(working_directory))?;

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
}
mod language {
    use std::path::Path;

    use anyhow::{Result, anyhow};

    use super::LanguageEntry;
    use super::command::run_weidu;

    pub(super) fn list_languages(
        weidu_binary: &Path,
        tp2_path: &Path,
    ) -> Result<Vec<LanguageEntry>> {
        let args = vec![
            "--nogame".to_string(),
            "--list-languages".to_string(),
            tp2_path.display().to_string(),
        ];
        let output = run_weidu(weidu_binary, &args, None)?;
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
        let output = run_weidu(weidu_binary, &args, Some(working_directory))?;
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
}

use std::path::Path;

use anyhow::Result;

use crate::mods::component::Component;

#[derive(Debug, Clone)]
pub struct LanguageEntry {
    pub id: String,
    pub label: String,
}

pub fn list_languages(weidu_binary: &Path, tp2_path: &Path) -> Result<Vec<LanguageEntry>> {
    language::list_languages(weidu_binary, tp2_path)
}

pub fn list_languages_for_game(
    weidu_binary: &Path,
    tp2_path: &Path,
    game_directory: &Path,
    working_directory: &Path,
) -> Result<Vec<LanguageEntry>> {
    language::list_languages_for_game(weidu_binary, tp2_path, game_directory, working_directory)
}

pub fn list_components(
    weidu_binary: &Path,
    game_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<Component>> {
    components::list_components(weidu_binary, game_directory, tp2_path, language)
}

pub fn list_components_lines(
    weidu_binary: &Path,
    game_directory: &Path,
    working_directory: &Path,
    tp2_path: &Path,
    language: &str,
) -> Result<Vec<String>> {
    components::list_components_lines(
        weidu_binary,
        game_directory,
        working_directory,
        tp2_path,
        language,
    )
}
