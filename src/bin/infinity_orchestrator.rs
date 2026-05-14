// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Infinity Orchestrator binary entry point.
//
// Phase 2 P2.T7: replaces Phase 1's `PlaceholderApp` with the real
// `bio::ui::orchestrator::OrchestratorApp`. The eframe creation callback:
//   1. Installs redesign fonts FIRST (per H7).
//   2. Skips BIO's `configure_typography` — calling it would wipe the
//      redesign FontDefinitions we just installed (see Phase 1 note below).
//   3. Constructs and returns `OrchestratorApp::new(dev_mode)`, which calls
//      `bio::app::app_bootstrap_init::initialize` directly (per H5).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Result, anyhow};
use clap::Parser;
use eframe::egui;

use bio::ui::orchestrator::OrchestratorApp;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
// Note: BIO's `configure_typography` is intentionally NOT called here. It calls
// `FontDefinitions::default()` + `ctx.set_fonts()`, which fully replaces the
// font config — wiping our `install_redesign_fonts` registrations. The
// redesign uses its own Poppins family directly via `FontFamily::Name`.

const APP_TITLE: &str = concat!(
    "Infinity Orchestrator (alpha) v",
    env!("CARGO_PKG_VERSION")
);

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 820.0;
const WINDOW_MIN_WIDTH: f32 = 1024.0;
const WINDOW_MIN_HEIGHT: f32 = 700.0;

/// Minimal CLI for the orchestrator binary. The wider BIO `Cli` covers
/// non-GUI subcommands (normal/eet/scan); the orchestrator is GUI-only for
/// now so we only need `--dev-mode` and `--log-level`.
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
            // Install only the redesign FontDefinitions (Poppins + FiraCode Nerd).
            // BIO's `configure_typography` is intentionally skipped — see imports
            // for the rationale (it would wipe our font registrations).
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
