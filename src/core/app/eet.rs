// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use crate::config::options::EetConfig;
use crate::install::plan::InstallPlan;
use crate::install::runner;
use crate::mods::log_file::LogFile;
use tracing::info;

pub fn run(config: &EetConfig) -> Result<()> {
    info!("command=eet {:?}", config);
    ensure_existing_file(&config.bg1_log_file)?;
    ensure_existing_dir(&config.bg1_game_directory)?;
    ensure_existing_file(&config.bg2_log_file)?;
    ensure_existing_dir(&config.bg2_game_directory)?;

    let bg1_plan = build_plan(
        &config.bg1_log_file,
        &config.bg1_game_directory,
        config.options.skip_installed,
        config.options.strict_matching,
        "pre-eet",
    )?;
    let bg2_plan = build_plan(
        &config.bg2_log_file,
        &config.bg2_game_directory,
        config.options.skip_installed,
        config.options.strict_matching,
        "eet",
    )?;

    info!("running pre-eet plan");
    runner::run_plan(&bg1_plan, &config.options, &config.bg1_game_directory)?;
    info!("running eet plan");
    runner::run_plan(&bg2_plan, &config.options, &config.bg2_game_directory)?;
    Ok(())
}

fn build_plan(
    log_path: &PathBuf,
    game_directory: &PathBuf,
    skip_installed: bool,
    strict_matching: bool,
    label: &str,
) -> Result<InstallPlan> {
    let target_log = LogFile::from_path(log_path)?;
    let mut plan = InstallPlan::from_log_file(&target_log);
    let pre_filter_count = plan.components.len();
    let installed_log_path = game_directory.join("weidu.log");

    let installed_log = LogFile::from_path(&installed_log_path).ok();
    if let Some(installed) = installed_log.as_ref() {
        plan.filter_installed(installed, skip_installed, strict_matching);
        info!(
            "{label} filter installed: target={} installed={} planned={}",
            pre_filter_count,
            installed.len(),
            plan.components.len()
        );
    } else {
        info!(
            "{label} installed log not found at {}; skipping installed filter",
            installed_log_path.display()
        );
    }
    info!("{label} install plan contains {} component(s)", plan.components.len());
    Ok(plan)
}

fn ensure_existing_file(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("expected file does not exist: {}", path.display());
    }
    Ok(())
}

fn ensure_existing_dir(path: &Path) -> Result<()> {
    if !path.is_dir() {
        bail!("expected directory does not exist: {}", path.display());
    }
    Ok(())
}
