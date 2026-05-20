// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::app::state::WizardState;
use bio::registry::model::ModlistEntry;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::step5::page_step5;
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::{post_install_actions, success_banner};

use eframe::egui;
use egui_kittest::Harness;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

struct Cell {
    w: u16,
    h: u16,
}

#[test]
fn render_workspace_step5_pre_install_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for cell in &[
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 680 },
    ] {
        written.push(render_cell(&out_dir, cell));
    }

    assert_written(
        &written,
        3,
        "expected 3 matrix-cell PNGs (the Phase-7 Run-1 widths 1280/1045/960)",
    );
}

fn render_cell(out_dir: &Path, cell: &Cell) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(f32::from(cell.w), f32::from(cell.h)))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }

            render_scaffold(ctx);
        });

    settle(&mut harness);

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!(
        "workspace_step5_preinstall__{}x{}.png",
        cell.w, cell.h
    ));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}x{}  pre-install  -> {}",
        cell.w,
        cell.h,
        abs.display()
    );

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

fn render_scaffold(ctx: &egui::Context) {
    render_titlebar(ctx);
    render_statusbar(ctx);
    render_body(ctx);
}

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("scaffold_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_statusbar(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("scaffold_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_body(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            render_rail(ui);
            render_page(ui);
        });
}

fn render_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("scaffold_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            render_flat_band(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
        });
}

fn render_page(ui: &mut egui::Ui) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, render_step5);
        });
}

fn render_step5(ui: &mut egui::Ui) {
    let mut wizard_state = WizardState::default();
    let mut console_view = Step5ConsoleViewState::default();
    let exe_fingerprint = String::new();
    let entry = ModlistEntry::default();
    let palette = ThemePalette::Dark;

    success_banner::render(ui, palette, &wizard_state, &entry);
    let _ = post_install_actions::render(ui, palette, &wizard_state, &entry);
    let _ = page_step5::render(
        ui,
        &mut wizard_state,
        &mut console_view,
        None,
        None,
        false,
        &exe_fingerprint,
    );
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

fn assert_written(written: &[PathBuf], expected: usize, message: &str) {
    for path in written {
        let meta = std::fs::metadata(path)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", path.display()));
        assert!(
            meta.len() > 0,
            "rendered PNG is empty (renderer produced no pixels): {}",
            path.display()
        );
    }

    assert_eq!(written.len(), expected, "{message}");
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp.parent().map(Path::to_path_buf).unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
