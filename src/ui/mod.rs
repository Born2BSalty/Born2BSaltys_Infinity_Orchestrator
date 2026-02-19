// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub mod app;
pub mod controller;
pub mod layout;
pub mod pages;
pub mod scan;
pub mod step1;
pub mod step2;
pub mod step3;
pub mod step4;
pub mod step5;
pub mod step2_worker;
pub mod state;
pub mod state_convert;
pub mod state_nav;
pub mod state_validation;
pub mod terminal;

use anyhow::{anyhow, Result};
use eframe::{NativeOptions, egui};
use layout::{WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH};
use std::fs;
use std::path::Path;

pub fn run(dev_mode: bool) -> Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT]),
        ..Default::default()
    };
    eframe::run_native(
        "Born2BSalty's Infinity Orchestrator (BIO)",
        options,
        Box::new(move |cc| {
            configure_fonts(&cc.egui_ctx);
            if !cfg!(debug_assertions) {
                cc.egui_ctx.set_visuals(egui::Visuals::dark());
            }
            Ok(Box::new(app::WizardApp::new(dev_mode)))
        }),
    )
    .map_err(|err| anyhow!("failed to launch GUI: {err}"))?;
    Ok(())
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let ui_candidates = [
        "C:/Windows/Fonts/FiraCodeNerdFont-Regular.ttf",
        "C:/Windows/Fonts/FiraCodeNerdFontMono-Regular.ttf",
        "C:/Windows/Fonts/FiraCodeNerdFont.ttf",
        "/usr/share/fonts/truetype/firacode/FiraCodeNerdFont-Regular.ttf",
        "/usr/share/fonts/TTF/FiraCodeNerdFont-Regular.ttf",
    ];
    let terminal_candidates = [
        "C:/Windows/Fonts/FiraCode-Regular.ttf",
        "C:/Windows/Fonts/FiraCode-Medium.ttf",
        "/usr/share/fonts/truetype/firacode/FiraCode-Regular.ttf",
        "/usr/share/fonts/TTF/FiraCode-Regular.ttf",
    ];
    let ui_bytes = ui_candidates
        .iter()
        .find_map(|p| fs::read(Path::new(p)).ok());
    let terminal_bytes = terminal_candidates
        .iter()
        .find_map(|p| fs::read(Path::new(p)).ok());

    if let Some(bytes) = ui_bytes {
        let name = "FiraCode Nerd Font".to_string();
        fonts.font_data.insert(
            name.clone(),
            egui::FontData::from_owned(bytes).into(),
        );
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, name.clone());
        }
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.insert(0, name);
        }
    }

    let terminal_font_name = if let Some(bytes) = terminal_bytes {
        let name = "FiraCode Terminal".to_string();
        fonts.font_data.insert(
            name.clone(),
            egui::FontData::from_owned(bytes).into(),
        );
        name
    } else {
        "Hack".to_string()
    };
    fonts.families.insert(
        egui::FontFamily::Name("TerminalMono".into()),
        vec![terminal_font_name, "Hack".to_string()],
    );

    ctx.set_fonts(fonts);
}