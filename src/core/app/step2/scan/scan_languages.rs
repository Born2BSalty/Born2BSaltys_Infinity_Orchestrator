// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use crate::app::state_validation_exec as exec_validation;
use crate::app::state_validation_fs as fs_validation;
use crate::config::options::ScanConfig;
use crate::install::weidu_scan;
use anyhow::{Result, anyhow, bail};
use tracing::info;
use walkdir::WalkDir;

pub fn run(config: &ScanConfig) -> Result<()> {
    if let ScanConfig::Languages {
        game_directory,
        filter_by_selected_language,
        options,
    } = config
    {
        info!(
            "command=scan languages game_directory={game_directory:?} filter_by_selected_language={filter_by_selected_language} options={options:?}"
        );

        ensure_binary("WeiDU binary", &options.weidu_binary)?;
        ensure_existing_dir(&options.mod_directories)?;
        ensure_game_directory("Game Directory", game_directory)?;

        let tp2_files = collect_tp2_files(&options.mod_directories, options.depth)?;
        let total_tp2 = tp2_files.len();
        let mut scanned_tp2 = 0usize;
        let mut matched_entries = 0usize;
        let needle = filter_by_selected_language.to_ascii_lowercase();
        for tp2 in tp2_files {
            scanned_tp2 += 1;
            info!(
                "scan languages progress: {scanned_tp2}/{total_tp2} {:?}",
                tp2
            );
            let working_directory = tp2.parent().unwrap_or(options.mod_directories.as_path());
            let langs = weidu_scan::list_languages_for_game(
                &options.weidu_binary,
                &tp2,
                game_directory,
                working_directory,
            )?;
            let filtered = langs
                .into_iter()
                .filter(|entry| entry.label.to_ascii_lowercase().contains(&needle))
                .collect::<Vec<_>>();
            if filtered.is_empty() {
                continue;
            }
            matched_entries += filtered.len();
            for entry in filtered {
                println!("{:?} lang={} label={}", tp2, entry.id, entry.label);
            }
        }
        info!(
            "scan languages summary: mods_scanned={} language_matches={}",
            scanned_tp2, matched_entries
        );
    }
    Ok(())
}

fn ensure_binary(label: &str, path: &Path) -> Result<()> {
    let value = path.to_string_lossy();
    if value.trim().is_empty() {
        bail!("{label} is required");
    }
    let mut checked = 0usize;
    let mut errors = Vec::new();
    exec_validation::check_file(label, value.as_ref(), &mut checked, &mut errors);
    ensure_validation_passed(errors)
}

fn ensure_game_directory(label: &str, path: &Path) -> Result<()> {
    let value = path.to_string_lossy();
    if value.trim().is_empty() {
        bail!("{label} is required");
    }
    let mut checked = 0usize;
    let mut errors = Vec::new();
    fs_validation::check_game_dir(label, value.as_ref(), &mut checked, &mut errors);
    ensure_validation_passed(errors)
}

fn ensure_validation_passed(errors: Vec<String>) -> Result<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        bail!("{}", errors.join(" | "))
    }
}

fn ensure_existing_dir(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("expected directory does not exist: {}", path.display());
    }
    Ok(())
}

fn collect_tp2_files(mod_root: &Path, depth: usize) -> Result<Vec<PathBuf>> {
    let mut tp2_files = Vec::new();
    for entry in WalkDir::new(mod_root).follow_links(true).max_depth(depth) {
        let entry = entry
            .map_err(|err| anyhow!("failed to scan mods folder {}: {err}", mod_root.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.ends_with(".tp2") {
            tp2_files.push(entry.path().to_path_buf());
        }
    }
    Ok(tp2_files)
}
