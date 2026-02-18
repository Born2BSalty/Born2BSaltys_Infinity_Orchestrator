// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::Result;
use crate::config::options::AppCommandConfig;
use tracing::info;

use super::{eet, normal, scan_components, scan_languages};

pub fn run(command: &AppCommandConfig) -> Result<()> {
    info!("BIO started");
    info!("command = {:?}", command);
    match command {
        AppCommandConfig::Gui { dev_mode } => crate::ui::run(*dev_mode)?,
        AppCommandConfig::Normal(config) => normal::run(config)?,
        AppCommandConfig::Eet(config) => eet::run(config)?,
        AppCommandConfig::Scan(scan) => match scan {
            crate::config::options::ScanConfig::Components { .. } => scan_components::run(scan)?,
            crate::config::options::ScanConfig::Languages { .. } => scan_languages::run(scan)?,
        },
    }
    Ok(())
}
