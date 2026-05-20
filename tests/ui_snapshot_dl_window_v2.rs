// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// **DL Fix-Set v2 render gate** — renders the §4.3 Downloading window in
// four scenes targeted at the post-DL-Run-2 defect fixes:
//   • `oldcode_mid` — mid-download with an OLD share code (no baked
//     `expected_size` → mix of known + unknown sizes). Fix 1a: pure-count
//     fallback ⇒ overall bar climbs honestly (not "% of currently-known
//     bytes"). Fix 1b: per-asset push ⇒ "N / T mods" tracks landings
//     (here, distinct row statuses prove that fan).
//   • `extract_mid` — every archive fetched; live extract snapshot says
//     3 / 10 completed. Fix 1c: the snapshot drives the Extract phase
//     bar (30%) instead of the count fallback (0%).
//   • `all_cached` — DL-Run-1 cached every archive (3 skipped, 0 to-fetch
//     rows). Fix 1e: "✓ already downloaded" rows + Download bar 100% +
//     extract phase auto-complete.
//   • `all_staged` — every to-fetch row Staged + skipped rows. Confirms
//     the chrome looks correct just before the advance gate.
//
// Per the standing render-gate rule, the orchestrator opens these PNGs
// itself and judges. Output: `target/ui-snapshots/dl_window_v2_*.png`.
//
// Test hygiene (directive-grade — DATA-LOSS): synthesised
// `DownloadProgress` + the public `install_redesign_fonts`. Constructs no
// `OrchestratorApp` / `RegistryStore` / `WorkspaceStore`, touches no
// `%APPDATA%` / real config dir.

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus, SkippedMod,
};
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

fn skip(name: &str, source: &str, size: Option<u64>) -> SkippedMod {
    SkippedMod {
        name: name.to_string(),
        source: source.to_string(),
        size,
    }
}

/// **Fix 1a / Fix 1b scene** — OLD share code (no baked sizes); mid-
/// download with assets at different lifecycle stages. The "any row
/// lacks known size" trigger fires ⇒ pure-count overall (the smooth bar
/// for old codes — instead of the "active-pool sizes dominate" defect).
fn scene_oldcode_mid() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            // Staged + Extracting rows act as "complete" in the count.
            row(
                "stratagems",
                "github:Gibberlings3/stratagems",
                ModDownloadStatus::Staged,
                None,
                None, // OLD code — no baked size
            ),
            row(
                "item_rev",
                "github:Gibberlings3/ItemRevisions",
                ModDownloadStatus::Extracting,
                None,
                None,
            ),
            // An in-flight no-Content-Length row (indeterminate fill).
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

/// **Fix 1c scene** — extract in flight; live snapshot says 3 / 10
/// completed. Extract phase bar reads 30%, not the count-fallback 0%.
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
        // **Fix 1c**: the snapshot drives the Extract bar.
        extract_progress: Some((3, 10)),
        ..Default::default()
    }
}

/// **Fix 1e scene** — DL-Run-1 cached every archive; 0 to-fetch rows + 3
/// skipped rows. Renders the "✓ already downloaded" rows + Download 100%
/// + Extract trivially 100% (no extract work). The auto-advance gate
/// fires honestly.
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

/// **Done scene** — every row Staged + skipped rows. Confirms the
/// post-advance look (Download full muted; Extract full live).
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

    let cells = [Cell { w: 1280, h: 820 }, Cell { w: 1045, h: 735 }];
    let scenes: [(&str, fn() -> DownloadProgress); 4] = [
        ("oldcode_mid", scene_oldcode_mid),
        ("extract_mid", scene_extract_mid),
        ("all_cached", scene_all_cached),
        ("all_staged", scene_all_staged),
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for (scene_name, scene_fn) in scenes {
        for cell in &cells {
            let w = cell.w as f32;
            let h = cell.h as f32;

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
                    let palette = ThemePalette::Dark;

                    egui::TopBottomPanel::top("scaffold_titlebar")
                        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
                        .resizable(false)
                        .show_separator_line(false)
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            let r = ui.max_rect();
                            ui.painter().rect_filled(
                                r,
                                0.0,
                                egui::Color32::from_rgb(0x15, 0x22, 0x2B),
                            );
                            ui.allocate_space(ui.available_size());
                        });

                    egui::TopBottomPanel::bottom("scaffold_statusbar")
                        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
                        .resizable(false)
                        .show_separator_line(false)
                        .frame(egui::Frame::NONE)
                        .show(ctx, |ui| {
                            let r = ui.max_rect();
                            ui.painter().rect_filled(
                                r,
                                0.0,
                                egui::Color32::from_rgb(0x15, 0x22, 0x2B),
                            );
                            ui.allocate_space(ui.available_size());
                        });

                    egui::CentralPanel::default()
                        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                        .show(ctx, |ui| {
                            egui::SidePanel::left("scaffold_rail")
                                .exact_width(REDESIGN_NAV_WIDTH_PX)
                                .resizable(false)
                                .show_separator_line(false)
                                .frame(egui::Frame::NONE)
                                .show_inside(ui, |ui| {
                                    let r = ui.max_rect();
                                    ui.painter().rect_filled(
                                        r,
                                        0.0,
                                        egui::Color32::from_rgb(0x15, 0x22, 0x2B),
                                    );
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
                                        &progress,
                                    );
                                });
                        });
                });

            for _ in 0..8 {
                harness.run();
            }
            let img = harness
                .render()
                .expect("egui_kittest wgpu render() must produce an image");
            let path = out_dir.join(format!(
                "dl_window_v2_{scene_name}__{}x{}.png",
                cell.w, cell.h
            ));
            img.save(&path)
                .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
            let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
            println!(
                "SNAPSHOT  {}x{}  dl-window-v2/{scene_name}  -> {}",
                cell.w,
                cell.h,
                abs.display()
            );
            written.push(path);
        }
    }

    for p in &written {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(meta.len() > 0, "rendered PNG empty: {}", p.display());
    }
    assert_eq!(written.len(), 8, "expected 8 PNGs (4 scenes × 2 widths)");
}

fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
