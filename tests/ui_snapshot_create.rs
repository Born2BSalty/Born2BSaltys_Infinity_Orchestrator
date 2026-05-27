// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::create::stage_choose;
use bio::ui::create::state_create::{CreateScreenState, StartingPoint};
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

struct Cell {
    w: u16,
    h: u16,
    starting_point: StartingPoint,
    sp_tag: &'static str,
}

#[test]
fn render_create_choose_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for cell in &create_matrix() {
        written.push(render_cell(&out_dir, cell));
    }

    assert_written(
        &written,
        8,
        "expected 8 matrix-cell PNGs (the Fix-Run 6 render matrix)",
    );
}

const fn create_matrix() -> [Cell; 8] {
    [
        Cell {
            w: 1280,
            h: 820,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        Cell {
            w: 1280,
            h: 820,
            starting_point: StartingPoint::Import,
            sp_tag: "import",
        },
        Cell {
            w: 1045,
            h: 735,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        Cell {
            w: 1021,
            h: 680,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        Cell {
            w: 1021,
            h: 680,
            starting_point: StartingPoint::Import,
            sp_tag: "import",
        },
        Cell {
            w: 1024,
            h: 680,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        Cell {
            w: 960,
            h: 680,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        Cell {
            w: 960,
            h: 680,
            starting_point: StartingPoint::Import,
            sp_tag: "import",
        },
    ]
}

fn render_cell(out_dir: &Path, cell: &Cell) -> PathBuf {
    let mut state = CreateScreenState::new();
    state.starting_point = cell.starting_point;

    let palette = ThemePalette::Dark;
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(f32::from(cell.w), f32::from(cell.h)))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }

            render_scaffold(ctx, palette, &mut state);
        });

    settle(&mut harness);

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!(
        "create_choose__{}x{}__{}.png",
        cell.w, cell.h, cell.sp_tag
    ));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}x{}  {:?}  -> {}",
        cell.w,
        cell.h,
        cell.starting_point,
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

fn render_scaffold(ctx: &egui::Context, palette: ThemePalette, state: &mut CreateScreenState) {
    render_titlebar(ctx);
    render_statusbar(ctx);
    render_body(ctx, palette, state);
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

fn render_body(ctx: &egui::Context, palette: ThemePalette, state: &mut CreateScreenState) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            render_rail(ui);
            render_page(ui, palette, state);
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

fn render_page(ui: &mut egui::Ui, palette: ThemePalette, state: &mut CreateScreenState) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
        .show_inside(ui, |ui| {
            let _ = stage_choose::render(ui, palette, state, false);
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
