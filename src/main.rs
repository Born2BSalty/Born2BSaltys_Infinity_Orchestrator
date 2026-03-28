// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use clap::Parser;

#[path = "core/app/mod.rs"]
mod app;
#[path = "core/cli/mod.rs"]
mod cli;
#[path = "core/config/mod.rs"]
mod config;
#[path = "core/install/mod.rs"]
mod install;
#[path = "core/logging/mod.rs"]
mod logging;
#[path = "core/mods/mod.rs"]
mod mods;
#[path = "core/platform_defaults.rs"]
mod platform_defaults;
#[path = "core/parser/mod.rs"]
mod parser;
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
