// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;

use bio::cli::args::{Cli, Command};
use bio::config::options;
use bio::logging::setup;

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    if cli.command.is_none() && cli.help.is_none() && cli.version.is_none() {
        cli.command = Some(Command::Gui);
    }
    setup::init(&cli.log_level)?;
    if let Some(command) = options::from_cli(&cli) {
        match &command {
            options::AppCommandConfig::Gui { dev_mode } => bio::ui::run(*dev_mode)?,
            _ => bio::app::dispatch::run(&command)?,
        }
    }
    Ok(())
}
