#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "infinity_orchestrator")]
struct OrchestratorCli {
    #[arg(long, default_value = "info")]
    log_level: String,

    #[arg(short = 'd', long)]
    dev_mode: bool,
}

fn main() -> Result<()> {
    let cli = OrchestratorCli::parse();
    bio::logging::setup::init(&cli.log_level)?;

    let mut options = bio::ui::frame::frame_window::native_options();
    options.viewport = options.viewport.with_decorations(false);
    eframe::run_native(
        "Infinity Orchestrator",
        options,
        Box::new(|cc| {
            bio::ui::shared::redesign_fonts::install_redesign_fonts(&cc.egui_ctx);
            Ok(Box::new(
                bio::ui::orchestrator::orchestrator_app::OrchestratorApp::new(cli.dev_mode),
            ))
        }),
    )
    .map_err(|err| anyhow::anyhow!("{err}"))?;

    Ok(())
}
