// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::orchestrator::left_rail;
use bio::ui::orchestrator::nav_destination::NavDestination;
use bio::ui::orchestrator::nav_status::PathValidationSummary;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::shell::shell_statusbar;
use bio::ui::shell::shell_titlebar;

use eframe::egui;
use egui_kittest::Harness;

const WINDOW_W: f32 = 1280.0;
const WINDOW_H: f32 = 820.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .map_or_else(|| PathBuf::from("target/tmp"), Path::to_path_buf)
        .join("render_gate/rebrand")
}

fn snap(out_dir: &Path, name: &str, palette: ThemePalette) -> PathBuf {
    let mut frame: u32 = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(WINDOW_W, WINDOW_H))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if frame == 0 {
                install_redesign_fonts(ctx);
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.allocate_space(ui.available_size());
                });
                frame += 1;
                return;
            }
            frame += 1;
            render_shell(ctx, palette);
        });

    for _ in 0..4 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = out_dir.join(format!("{name}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {palette:?}  rebrand rail+titlebar  -> {}",
        abs.display()
    );

    path
}

fn render_shell(ctx: &egui::Context, palette: ThemePalette) {
    let mut nav = NavDestination::Home;
    let validation = PathValidationSummary::default();

    egui::TopBottomPanel::top("rebrand_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            shell_titlebar::render(ui, palette);
        });

    egui::TopBottomPanel::bottom("rebrand_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let _ = shell_statusbar::render(ui, palette, 0, None, false, false);
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            egui::SidePanel::left("rebrand_rail")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    left_rail::render(ui, palette, &mut nav, false, &validation, None);
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.allocate_space(ui.available_size());
                });
        });
}

#[test]
fn render_rebrand_rail_and_titlebar() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create render_gate/rebrand dir");

    let mut written: Vec<PathBuf> = Vec::new();
    for (label, palette) in PALETTES {
        written.push(snap(&dir, &format!("rebrand__{label}"), palette));
    }

    assert_eq!(written.len(), 2, "expected 2 PNGs (dark + light)");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG is empty: {}", path.display());
    }
}
