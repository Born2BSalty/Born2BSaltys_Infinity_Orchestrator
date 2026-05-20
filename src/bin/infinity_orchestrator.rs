// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Result, anyhow};
use clap::Parser;
use eframe::egui;

use bio::ui::orchestrator::OrchestratorApp;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;

const APP_TITLE: &str = concat!("Infinity Orchestrator (alpha) v", env!("CARGO_PKG_VERSION"));

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 820.0;
const WINDOW_MIN_WIDTH: f32 = 1024.0;
const WINDOW_MIN_HEIGHT: f32 = 700.0;

#[derive(Parser, Debug)]
#[command(name = "infinity_orchestrator")]
#[command(version)]
#[command(about = "Infinity Orchestrator — the redesigned BIO frontend (alpha)")]
struct OrchestratorCli {
    #[arg(long, default_value = "info")]
    log_level: String,
    #[arg(short = 'd', long, default_value_t = false)]
    dev_mode: bool,
}

fn main() -> Result<()> {
    let cli = OrchestratorCli::parse();
    bio::logging::setup::init(&cli.log_level)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_icon(app_icon())
            .with_decorations(false)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            install_redesign_fonts(&cc.egui_ctx);
            Ok(Box::new(OrchestratorApp::new(cli.dev_mode)))
        }),
    )
    .map_err(|err| anyhow!("failed to launch Infinity Orchestrator: {err}"))?;

    Ok(())
}

fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png"))
        .expect("assets/icon.png must be a valid PNG icon")
}
