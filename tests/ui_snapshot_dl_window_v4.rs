// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Render gate covering four Install Downloading-screen scenes:
//   - fresh_install_first_frame: a re-entered install with the
//     hash + extract snapshots blanked on clear_preview so the new
//     first frame cannot inherit a previous install's `(N, N)`.
//   - mid_hash_classified: rows whose hash decision is in flight
//     render as Hashing; rows already classified render as Queued
//     or Skipped.
//   - mid_download_final_pinned: the final asset's per_byte is
//     `(final_bytes, Some(final_bytes))` so the bar reads 100%
//     even when Content-Length lied.
//   - paste_stage_after_reset: the Paste-stage scaffolding the
//     user lands on after the reset.
//
// All scenes are rendered at three widths (1280 / 1045 / 960) and
// written under `target/ui-snapshots/`. Constructs no
// `OrchestratorApp` / `RegistryStore` / `WorkspaceStore`; touches no
// `%APPDATA%` / real config dir.

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus,
};
use bio::ui::shared::numeric::f32_from_u32;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{REDESIGN_NAV_WIDTH_PX, ThemePalette};

use eframe::egui;
use egui_kittest::Harness;

type SceneFn = fn() -> DownloadProgress;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

struct Cell {
    w: u32,
    h: u32,
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

/// Scene: a freshly-armed install for which `clear_preview` has been
/// called on entry; `hash_progress` and `extract_progress` on the
/// `DownloadProgress` are reset to None so the chrome cannot show the
/// previous install's 51/51. With no rows yet, the screen reads as
/// "queue is being built" rather than "no mods queued while hash
/// reports 100%".
fn scene_fresh_install_first_frame() -> DownloadProgress {
    DownloadProgress::default()
}

/// Scene: classification mid-hash. Indices 1 and 3 already came back
/// (their hash decision is recorded → row status is Queued or
/// Skipped). Indices 0, 2, 4 are still Hashing.
fn scene_mid_hash_classified() -> DownloadProgress {
    let mut p = DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Hashing,
                None,
                Some(100_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Hashing,
                None,
                Some(60_000_000),
            ),
            row(
                "spell_rev",
                "weasel:..",
                ModDownloadStatus::Hashing,
                None,
                Some(40_000_000),
            ),
            // The pass classified indices 1 and 3 so far — they fall
            // through to Queued (download has not run yet) or Skipped
            // (their bytes already on disk).
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Queued,
                None,
                Some(80_000_000),
            ),
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Skipped,
                None,
                Some(20_000_000),
            ),
        ],
        ..Default::default()
    };
    p.hash_progress = Some((2, 5));
    p
}

/// Scene: mid-download with the final asset's bar pinned to 100%
/// before the status flips. The penultimate row is still downloading
/// at 80%; the final row's `AssetDone` has fired so its `per_byte` is
/// `(final, Some(final))` ⇒ bar = 1.0 — but the row's *status* is
/// still `Downloading` because the next `from_wizard_state_full`
/// rebuild has not yet re-classified it to Extracting / Staged.
fn scene_mid_download_final_pinned() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Downloading,
                Some((80_000_000, Some(100_000_000))),
                Some(100_000_000),
            ),
            // Final asset: AssetDone arrived. Bug 4 fix: per_byte is
            // (final_bytes, Some(final_bytes)) so bar_fraction == 1.0
            // even when Content-Length was wrong or absent.
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Downloading,
                Some((42_517_318, Some(42_517_318))),
                Some(42_517_318),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Queued,
                None,
                Some(60_000_000),
            ),
        ],
        ..Default::default()
    }
}

/// Scene: post-reset Paste-stage (the empty grid + arm gate cleared).
/// Rendered via `DownloadProgress::default()` because the reset
/// replaces `download_progress` with the default value.
fn scene_paste_stage_after_reset() -> DownloadProgress {
    DownloadProgress::default()
}

fn paint_scaffold_chrome(ctx: &egui::Context) {
    let bg = egui::Color32::from_rgb(0x0B, 0x11, 0x16);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(bg))
        .show(ctx, |ui| {
            ui.allocate_space(ui.available_size());
        });
}

fn paint_shell_with_progress(
    ctx: &egui::Context,
    palette: ThemePalette,
    progress: &DownloadProgress,
) {
    let chrome = egui::Color32::from_rgb(0x15, 0x22, 0x2B);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            egui::SidePanel::left("scaffold_rail_v4")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    let r = ui.max_rect();
                    ui.painter().rect_filled(r, 0.0, chrome);
                    ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
                    ui.allocate_space(ui.available_size());
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    let _ = stage_downloading::render(
                        ui,
                        palette,
                        DownloadScreenCopy::INSTALL,
                        progress,
                    );
                });
        });
}

fn render_scene_to_png(
    out_dir: &std::path::Path,
    scene_name: &str,
    scene_fn: SceneFn,
    cell: &Cell,
) -> std::path::PathBuf {
    let w = f32_from_u32(cell.w);
    let h = f32_from_u32(cell.h);

    let mut frame: u64 = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(w, h))
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
            let progress = scene_fn();
            paint_scaffold_chrome(ctx);
            paint_shell_with_progress(ctx, ThemePalette::Dark, &progress);
        });

    for _ in 0..8 {
        harness.run();
    }
    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!(
        "dl_window_v4_{scene_name}__{}x{}.png",
        cell.w, cell.h
    ));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}x{}  dl-window-v4/{scene_name}  -> {}",
        cell.w,
        cell.h,
        abs.display()
    );
    path
}

#[test]
fn render_dl_window_v4_scenes() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let cells = [
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 720 },
    ];
    let scenes: [(&str, SceneFn); 4] = [
        ("fresh_install_first_frame", scene_fresh_install_first_frame),
        ("mid_hash_classified", scene_mid_hash_classified),
        ("mid_download_final_pinned", scene_mid_download_final_pinned),
        ("paste_stage_after_reset", scene_paste_stage_after_reset),
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();
    for (scene_name, scene_fn) in scenes {
        for cell in &cells {
            written.push(render_scene_to_png(&out_dir, scene_name, scene_fn, cell));
        }
    }

    for p in &written {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(meta.len() > 0, "rendered PNG empty: {}", p.display());
    }
    assert_eq!(written.len(), 12, "expected 12 PNGs (4 scenes x 3 widths)");
}

fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
