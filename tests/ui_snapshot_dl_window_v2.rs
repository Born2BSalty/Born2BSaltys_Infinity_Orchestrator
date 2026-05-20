// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus, SkippedMod,
};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};

use eframe::egui;
use egui_kittest::Harness;

type SceneFactory = fn() -> DownloadProgress;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const CELLS: [Cell; 2] = [
    Cell {
        label: "1280x820",
        w: 1280.0,
        h: 820.0,
    },
    Cell {
        label: "1045x735",
        w: 1045.0,
        h: 735.0,
    },
];

const SCENES: [Scene; 4] = [
    Scene {
        name: "oldcode_mid",
        build: scene_oldcode_mid,
    },
    Scene {
        name: "extract_mid",
        build: scene_extract_mid,
    },
    Scene {
        name: "all_cached",
        build: scene_all_cached,
    },
    Scene {
        name: "all_staged",
        build: scene_all_staged,
    },
];

#[derive(Clone, Copy)]
struct Cell {
    label: &'static str,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy)]
struct Scene {
    name: &'static str,
    build: SceneFactory,
}

fn row(
    name: &str,
    source: &str,
    status: ModDownloadStatus,
    per_byte: Option<(u64, Option<u64>)>,
    expected_size: Option<u64>,
) -> ModDownloadRow {
    ModDownloadRow {
        name: name.to_string(),
        source: source.to_string(),
        status,
        per_byte,
        expected_size,
    }
}

fn skip(name: &str, source: &str, size: Option<u64>) -> SkippedMod {
    SkippedMod {
        name: name.to_string(),
        source: source.to_string(),
        size,
    }
}

fn scene_oldcode_mid() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:Gibberlings3/stratagems",
                ModDownloadStatus::Staged,
                None,
                None,
            ),
            row(
                "item_rev",
                "github:Gibberlings3/ItemRevisions",
                ModDownloadStatus::Extracting,
                None,
                None,
            ),
            row(
                "eet_tweaks",
                "weasel:eet_tweaks",
                ModDownloadStatus::Downloading,
                Some((4_096_000, None)),
                None,
            ),
            row(
                "cdtweaks",
                "github:Gibberlings3/cdtweaks",
                ModDownloadStatus::Queued,
                None,
                None,
            ),
            row(
                "spell_rev",
                "weasel:spell_rev",
                ModDownloadStatus::Queued,
                None,
                None,
            ),
        ],
        ..Default::default()
    }
}

fn scene_extract_mid() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Extracting,
                None,
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Extracting,
                None,
                Some(80_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Extracting,
                None,
                None,
            ),
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Extracting,
                None,
                Some(20_000_000),
            ),
            row(
                "spell_rev",
                "weasel:..",
                ModDownloadStatus::Extracting,
                None,
                Some(40_000_000),
            ),
        ],
        skipped: vec![
            skip("EET", "github:K4thos/EET", Some(500_000_000)),
            skip("EET_end", "github:K4thos/EET_end", Some(50_000_000)),
        ],
        extract_progress: Some((3, 10)),
        ..Default::default()
    }
}

fn scene_all_cached() -> DownloadProgress {
    DownloadProgress {
        skipped: vec![
            skip("stratagems", "github:..", Some(100_000_000)),
            skip("item_rev", "github:..", Some(80_000_000)),
            skip("eet_tweaks", "weasel:..", Some(60_000_000)),
        ],
        ..Default::default()
    }
}

fn scene_all_staged() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Staged,
                None,
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Staged,
                None,
                Some(80_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Staged,
                None,
                Some(60_000_000),
            ),
        ],
        skipped: vec![
            skip("EET", "github:K4thos/EET", Some(500_000_000)),
            skip("EET_end", "github:K4thos/EET_end", Some(50_000_000)),
        ],
        ..Default::default()
    }
}

#[test]
fn render_dl_window_v2_fix_scenes() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();
    for scene in SCENES {
        for cell in CELLS {
            written.push(render_scene_cell(&out_dir, scene, cell));
        }
    }

    assert_written(&written, 8, "expected 8 PNGs (4 scenes x 2 widths)");
}

fn render_scene_cell(out_dir: &Path, scene: Scene, cell: Cell) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(cell.w, cell.h))
        .with_pixels_per_point(1.0)
        .build(move |ctx| render_frame(ctx, scene.build, &mut frame));

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!("dl_window_v2_{}__{}.png", scene.name, cell.label));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}  dl-window-v2/{}  -> {}",
        cell.label,
        scene.name,
        abs.display()
    );
    path
}

fn render_frame(ctx: &egui::Context, scene_fn: SceneFactory, frame: &mut u64) {
    if *frame == 0 {
        render_font_frame(ctx);
        *frame += 1;
        return;
    }

    *frame += 1;
    let progress = scene_fn();
    render_scaffold(ctx, &progress);
}

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_scaffold(ctx: &egui::Context, progress: &DownloadProgress) {
    render_titlebar(ctx);
    render_statusbar(ctx);
    render_content(ctx, ThemePalette::Dark, progress);
}

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("scaffold_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn render_statusbar(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("scaffold_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn fill_panel(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn render_content(ctx: &egui::Context, palette: ThemePalette, progress: &DownloadProgress) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            render_rail(ui);
            render_page(ui, palette, progress);
        });
}

fn render_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("scaffold_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            fill_panel(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
        });
}

fn render_page(ui: &mut egui::Ui, palette: ThemePalette, progress: &DownloadProgress) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
        .show_inside(ui, |ui| {
            let _ = stage_downloading::render(ui, palette, DownloadScreenCopy::INSTALL, progress);
        });
}

fn assert_written(written: &[PathBuf], expected: usize, message: &str) {
    for path in written {
        let meta = std::fs::metadata(path)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", path.display()));
        assert!(meta.len() > 0, "rendered PNG empty: {}", path.display());
    }
    assert_eq!(written.len(), expected, "{message}");
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp.parent().map(Path::to_path_buf).unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
