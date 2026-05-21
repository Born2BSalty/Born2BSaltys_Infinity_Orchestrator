// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::create::stage_choose;
use bio::ui::create::stage_fork_download::{self};
use bio::ui::create::stage_fork_paste;
use bio::ui::create::state_create::{CreateScreenState, StartingPoint};
use bio::ui::install::destination_not_empty;
use bio::ui::install::stage_downloading::{self, DownloadProgress, DownloadScreenCopy};
use bio::ui::install::state_install::DestChoice;
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

const WIDTHS: [(u16, u16); 3] = [(1280, 820), (1045, 735), (960, 680)];

#[test]
fn render_fork_pipeline_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for (w, h) in WIDTHS {
        written.push(render_choose_with_warning(&out_dir, w, h));
        written.push(render_fork_paste_fresh(&out_dir, w, h));
        written.push(render_fork_download_chassis(&out_dir, w, h));
        written.push(render_fork_download_populated(&out_dir, w, h));
    }

    assert_written(
        &written,
        WIDTHS.len() * 4,
        "expected 4 fork-pipeline scenes × 3 widths = 12 PNGs",
    );
}

fn render_choose_with_warning(out_dir: &Path, w: u16, h: u16) -> PathBuf {
    let palette = ThemePalette::Dark;
    let path = out_dir.join(format!("fork__create_choose_dest_warn__{w}x{h}.png"));
    let mut state = CreateScreenState::new();
    state.starting_point = StartingPoint::Scratch;
    state.modlist_name = "My modlist".to_string();
    state.destination_choice = Some(DestChoice::Clear);
    let body = move |ui: &mut egui::Ui, palette: ThemePalette| {
        // Real Create Setup Box (renders the same warning the live screen does)
        // plus a hand-painted destination-not-empty box to ensure the warning
        // surface is captured visually even on the headless harness (which has
        // no real directory to enumerate).
        let _ = stage_choose::render(ui, palette, &mut state);
        ui.add_space(8.0);
        let _ = destination_not_empty::render(ui, palette, Some(DestChoice::Clear), false);
    };
    render_with_scaffold(w, h, palette, body, &path);
    path
}

fn render_fork_paste_fresh(out_dir: &Path, w: u16, h: u16) -> PathBuf {
    let palette = ThemePalette::Dark;
    let path = out_dir.join(format!("fork__paste_fresh__{w}x{h}.png"));
    let mut state = state_for_import("My fork", "");
    let body = move |ui: &mut egui::Ui, palette: ThemePalette| {
        let _ = stage_fork_paste::render(ui, palette, &mut state);
    };
    render_with_scaffold(w, h, palette, body, &path);
    path
}

fn render_fork_download_chassis(out_dir: &Path, w: u16, h: u16) -> PathBuf {
    let palette = ThemePalette::Dark;
    let path = out_dir.join(format!("fork__download_empty__{w}x{h}.png"));
    let progress = DownloadProgress::default();
    let body = move |ui: &mut egui::Ui, palette: ThemePalette| {
        let _ = stage_fork_download::render(ui, palette, &progress);
    };
    render_with_scaffold(w, h, palette, body, &path);
    path
}

fn render_fork_download_populated(out_dir: &Path, w: u16, h: u16) -> PathBuf {
    let palette = ThemePalette::Dark;
    let path = out_dir.join(format!("fork__download_populated__{w}x{h}.png"));
    let progress = populated_progress();
    let body = move |ui: &mut egui::Ui, palette: ThemePalette| {
        // Render the same chassis with synthetic download progress so the
        // mid-download grid + overall progress bars are captured.
        let copy = DownloadScreenCopy {
            title: "Downloading fork",
            sub: "fetching the parent's mods — Step 2 opens automatically when ready",
            hint: Some(
                "after download: components auto-selected · order applied · lands on Step 2",
            ),
        };
        let _ = stage_downloading::render(ui, palette, copy, &progress);
    };
    render_with_scaffold(w, h, palette, body, &path);
    path
}

fn populated_progress() -> DownloadProgress {
    use bio::app::state::WizardState;
    let mut ws = WizardState::default();
    ws.step2.update_selected_update_assets = vec![
        demo_asset("EET/EET.TP2", "EET — base"),
        demo_asset("EEUITWEAKS/EEUITWEAKS.TP2", "UI tweaks"),
        demo_asset("EEEX/EEEX.TP2", "EEex"),
    ];
    ws.step2.update_selected_download_running = true;
    let bytes = std::collections::BTreeMap::from([
        (0usize, (1024u64 * 1024 * 18, Some(1024u64 * 1024 * 64))),
        (1usize, (1024u64 * 1024 * 8, Some(1024u64 * 1024 * 12))),
    ]);
    let skipped: Vec<bio::ui::install::stage_downloading::SkippedMod> = Vec::new();
    let expected_sizes = std::collections::BTreeMap::from([
        (0usize, 1024u64 * 1024 * 64),
        (1usize, 1024u64 * 1024 * 12),
        (2usize, 1024u64 * 1024 * 4),
    ]);
    DownloadProgress::from_wizard_state_full(&ws, &bytes, &skipped, &expected_sizes, None)
}

fn demo_asset(tp_file: &str, label: &str) -> bio::app::state::Step2UpdateAsset {
    bio::app::state::Step2UpdateAsset {
        game_tab: "BGEE".to_string(),
        tp_file: tp_file.to_string(),
        label: label.to_string(),
        source_id: "github".to_string(),
        tag: "v1.0".to_string(),
        asset_name: format!("{label}.zip"),
        asset_url: format!("http://localhost/{tp_file}"),
        installed_source_ref: None,
    }
}

fn state_for_import(modlist_name: &str, fork_code: &str) -> CreateScreenState {
    let mut s = CreateScreenState::new();
    s.modlist_name = modlist_name.to_string();
    s.fork_code = fork_code.to_string();
    s.starting_point = StartingPoint::Import;
    s
}

fn render_with_scaffold<F>(w: u16, h: u16, palette: ThemePalette, mut body: F, path: &Path)
where
    F: FnMut(&mut egui::Ui, ThemePalette) + Send + 'static,
{
    let mut frame = 0u64;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(f32::from(w), f32::from(h)))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }
            render_titlebar(ctx);
            render_statusbar(ctx);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                .show(ctx, |ui| {
                    render_rail(ui);
                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                        .show_inside(ui, |ui| {
                            body(ui, palette);
                        });
                });
        });

    settle(&mut harness);

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    img.save(path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    println!("SNAPSHOT  {w}x{h}  -> {}", path.display());
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

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("fork_scaffold_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_statusbar(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("fork_scaffold_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("fork_scaffold_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            render_flat_band(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
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
