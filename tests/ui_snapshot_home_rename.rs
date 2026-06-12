// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::home::modlist_card;
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

fn sample_entry() -> ModlistEntry {
    let mut e = ModlistEntry::default();
    e.id = "SNAPTEST0001".to_string();
    e.name = "Tactical EET 2026".to_string();
    e.game = Game::EET;
    e.state = ModlistState::InProgress;
    e.mod_count = 12;
    e.component_count = 84;
    e
}

#[test]
fn render_home_card_normal_and_rename() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let written = vec![render_normal(&out_dir), render_renaming(&out_dir)];

    for path in &written {
        let meta = std::fs::metadata(path)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", path.display()));
        assert!(meta.len() > 0, "rendered PNG is empty: {}", path.display());
    }
    assert_eq!(written.len(), 2, "expected 2 home-rename snapshot PNGs");
}

fn render_normal(out_dir: &Path) -> PathBuf {
    let entry = sample_entry();
    let palette = ThemePalette::Dark;
    let mut frame = 0u64;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 820.0))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }
            render_scaffold(ctx, |ui| {
                let _ = modlist_card::render(ui, palette, &entry, None);
            });
        });

    settle(&mut harness);

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join("home_card_rename__normal__1280x820.png");
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!("SNAPSHOT home_card normal -> {}", abs.display());
    path
}

fn render_renaming(out_dir: &Path) -> PathBuf {
    let entry = sample_entry();
    let palette = ThemePalette::Dark;
    let mut frame = 0u64;
    let mut rename_buf = "Tactical EET 2026".to_string();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 820.0))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }
            render_scaffold(ctx, |ui| {
                let _ = modlist_card::render(ui, palette, &entry, Some(&mut rename_buf));
            });
        });

    settle(&mut harness);

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join("home_card_rename__renaming__1280x820.png");
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!("SNAPSHOT home_card renaming -> {}", abs.display());
    path
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

fn render_scaffold(ctx: &egui::Context, content: impl FnOnce(&mut egui::Ui)) {
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
                .show_inside(ui, content);
        });
}

fn render_flat_band(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn settle(harness: &mut Harness<'_>) {
    for _ in 0..8 {
        harness.run();
    }
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp.parent().map(Path::to_path_buf).unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
