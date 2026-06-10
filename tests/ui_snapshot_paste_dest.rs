// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::install::stage_paste;
use bio::ui::install::state_install::InstallScreenState;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};

use eframe::egui;
use egui_kittest::Harness;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

#[test]
fn render_install_paste_destination_row() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut state = InstallScreenState::default();
    state.destination = "D:\\BG2EE_install_test".to_string();

    let palette = ThemePalette::Dark;
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 820.0))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }
            render_scaffold(ctx, palette, &mut state);
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join("install_paste__1280x820.png");
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!("SNAPSHOT install_paste -> {}", abs.display());

    let meta = std::fs::metadata(&path).expect("PNG exists");
    assert!(meta.len() > 0, "rendered PNG is empty");
}

fn install_fonts_frame(ctx: &egui::Context, frame: &mut u64) -> bool {
    if *frame == 0 {
        install_redesign_fonts(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_space(ui.available_size());
        });
        *frame += 1;
        return true;
    }
    *frame += 1;
    false
}

fn render_scaffold(ctx: &egui::Context, palette: ThemePalette, state: &mut InstallScreenState) {
    egui::TopBottomPanel::top("scaffold_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
    egui::TopBottomPanel::bottom("scaffold_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            egui::SidePanel::left("scaffold_rail")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    render_flat_band(ui);
                    ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    let _ = stage_paste::render(ui, palette, state);
                });
        });
}

fn render_flat_band(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp.parent().map(Path::to_path_buf).unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
