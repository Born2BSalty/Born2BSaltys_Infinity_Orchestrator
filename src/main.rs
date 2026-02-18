// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;

mod app;
mod cli;
mod compat;
mod config;
mod install;
mod logging;
mod mods;
mod platform_defaults;
mod settings;
mod ui;

use app::dispatch::run;
use cli::args::{Cli, Command};
use config::options;
use logging::setup;

fn main() -> Result<()> {
    let mut cli = Cli::parse();
    if cli.command.is_none() && cli.help.is_none() && cli.version.is_none() {
        cli.command = Some(Command::Gui);
    }
    setup::init(&cli.log_level)?;
    if let Some(command) = options::from_cli(&cli) {
        run(&command)?;
    }
    Ok(())
}
