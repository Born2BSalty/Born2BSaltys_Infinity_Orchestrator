// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use anyhow::{bail, Result};
use crate::config::options::NormalConfig;
use crate::install::plan::InstallPlan;
use crate::install::runner;
use crate::mods::log_file::LogFile;
use tracing::info;

pub fn run(config: &NormalConfig) -> Result<()> {
    info!("command=normal {:?}", config);

    ensure_existing_file(&config.log_file)?;
    ensure_existing_dir(&config.game_directory)?;
    ensure_existing_file(&config.options.weidu_binary)?;
    ensure_existing_dir(&config.options.mod_directories)?;

    let log_file = LogFile::from_path(&config.log_file)?;
    let mut plan = InstallPlan::from_log_file(&log_file);
    let pre_filter_count = plan.components.len();

    let installed_log_path = config.game_directory.join("weidu.log");
    let installed_log = LogFile::from_path(&installed_log_path).ok();
    if let Some(installed) = installed_log.as_ref() {
        plan.filter_installed(
            installed,
            config.options.skip_installed,
            config.options.strict_matching,
        );
        info!(
            "normal filter installed: target={} installed={} planned={}",
            pre_filter_count,
            installed.len(),
            plan.components.len()
        );
    } else {
        info!(
            "normal installed log not found at {}; skipping installed filter",
            installed_log_path.display()
        );
    }

    info!("normal install plan contains {} component(s)", plan.components.len());
    runner::run_plan(&plan, &config.options, &config.game_directory)?;
    Ok(())
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
