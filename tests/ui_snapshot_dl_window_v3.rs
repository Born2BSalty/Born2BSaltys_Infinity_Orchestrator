// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// **DL Fix-Set v3 render gate** — renders the §4.3 Downloading window
// in SIX scenes targeted at the v3 design's defining moments. The
// orchestrator opens these PNGs itself and judges. Output:
// `target/ui-snapshots/dl_window_v3_*.png`.
//
//   • `hashing` — every row Hashing, "Checking cache" verb line (the
//     new Hashing phase the v3 async skip pass introduces).
//   • `mid_hashing` — some rows transitioned to Skipped/Queued, some
//     still Hashing; the rows are sorted by status priority
//     (Hashing top, Queued mid, Skipped bottom).
//   • `mid_download` — mix of Downloading / Queued / downloaded
//     (cached + extracting); rows sorted (active top, pending
//     middle, downloaded bottom). All downloaded-terminal rows
//     render as "✓ downloaded".
//   • `mid_extract` — every row "✓ downloaded"; Extract bar climbing
//     (live `extract_progress` snapshot says 7/15 — the parallel
//     extract coordinator's mid-extract state).
//   • `preparing` — extract done; "Preparing to install" verb line;
//     both phase bars at 100% (the v3 UX hand-off beat at the tail
//     of Extract).
//   • `all_cached` — every row Skipped (DL-Run-1 all-cached);
//     "✓ downloaded" uniform caption, full Download bar, no extract
//     work.
//
// Test hygiene (directive-grade — DATA-LOSS): synthesised
// `DownloadProgress` + the public `install_redesign_fonts`. Constructs
// no `OrchestratorApp` / `RegistryStore` / `WorkspaceStore`, touches
// no `%APPDATA%` / real config dir.

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus,
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

/// Scene: every row Hashing (the new Hashing phase entry). Phase
/// indicator shows "Checking cache … 0 / 5 mods · 0%"; rows render
/// the new "checking cache..." caption.
fn scene_hashing() -> DownloadProgress {
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
                "item_rev",
                "github:..",
                ModDownloadStatus::Hashing,
                None,
                Some(80_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Hashing,
                None,
                Some(60_000_000),
            ),
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Hashing,
                None,
                Some(20_000_000),
            ),
            row(
                "spell_rev",
                "weasel:..",
                ModDownloadStatus::Hashing,
                None,
                Some(40_000_000),
            ),
        ],
        ..Default::default()
    };
    p.hash_progress = Some((0, 5));
    p
}

/// Scene: mid-hashing. The pool has classified 2 of 5 (one skipped,
/// one queued); 3 are still Hashing. The sort puts Hashing first,
/// Queued middle, Skipped last.
fn scene_mid_hashing() -> DownloadProgress {
    let mut p = DownloadProgress {
        // Already sorted per the v3 priority for the chassis render path
        // (the chassis path takes a literal struct; the live path sorts
        // in `from_wizard_state_full`).
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Hashing,
                None,
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Hashing,
                None,
                Some(80_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Hashing,
                None,
                Some(60_000_000),
            ),
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Queued,
                None,
                Some(20_000_000),
            ),
            row(
                "spell_rev",
                "weasel:..",
                ModDownloadStatus::Skipped,
                None,
                Some(40_000_000),
            ),
        ],
        ..Default::default()
    };
    p.hash_progress = Some((2, 5));
    p
}

/// Scene: mid-download. Skip pass done; some assets downloading,
/// some queued, some already-downloaded (cached + extracting). The
/// row sort: Downloading top, Queued middle, downloaded-terminal
/// bottom. The downloaded rows all render uniformly as "✓ downloaded"
/// (Imp-2 collapse).
fn scene_mid_download() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            // Downloading rows — top tier, real per-mod byte fractions.
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Downloading,
                Some((45_000_000, Some(100_000_000))),
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Downloading,
                Some((20_000_000, Some(80_000_000))),
                Some(80_000_000),
            ),
            // Queued — middle tier.
            row(
                "cdtweaks",
                "github:..",
                ModDownloadStatus::Queued,
                None,
                Some(20_000_000),
            ),
            row(
                "spell_rev",
                "weasel:..",
                ModDownloadStatus::Queued,
                None,
                Some(40_000_000),
            ),
            // Downloaded-terminal — bottom tier. All render as
            // "✓ downloaded" (uniform Imp-2 caption).
            row(
                "EET",
                "github:K4thos/EET",
                ModDownloadStatus::Skipped,
                None,
                Some(500_000_000),
            ),
            row(
                "EET_end",
                "github:K4thos/EET_end",
                ModDownloadStatus::Extracting,
                None,
                Some(50_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Staged,
                None,
                Some(60_000_000),
            ),
        ],
        ..Default::default()
    }
}

/// Scene: mid-extract. Every row downloaded-terminal (Skipped /
/// Extracting / Staged — all rendered "✓ downloaded"). Extract bar
/// reads its live snapshot (7 / 15 = 47%) via `extract_progress`.
fn scene_mid_extract() -> DownloadProgress {
    let mut p = DownloadProgress {
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
            row(
                "EET",
                "github:K4thos/EET",
                ModDownloadStatus::Skipped,
                None,
                Some(500_000_000),
            ),
            row(
                "EET_end",
                "github:K4thos/EET_end",
                ModDownloadStatus::Skipped,
                None,
                Some(50_000_000),
            ),
        ],
        ..Default::default()
    };
    // The parallel extract coordinator's live snapshot: 7 of 15
    // archives extracted (15 because the synthetic scene's extract_
    // total — `extract_total() = rows.len()` — is 7; we set the
    // snapshot to (7, 15) anyway to exercise the snapshot-driven
    // path. The chrome reads the snapshot directly.)
    p.extract_progress = Some((7, 15));
    p
}

/// Scene: "Preparing to install" — the v3 UX hand-off beat. Extract
/// done (snapshot (N, N)), every row downloaded-terminal. The phase
/// verb line swaps from "Extracting … N / T mods · X%" to
/// "Preparing to install …"; both phase bars at 100%.
fn scene_preparing() -> DownloadProgress {
    let mut p = DownloadProgress {
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
            row(
                "EET",
                "github:K4thos/EET",
                ModDownloadStatus::Skipped,
                None,
                Some(500_000_000),
            ),
            row(
                "EET_end",
                "github:K4thos/EET_end",
                ModDownloadStatus::Skipped,
                None,
                Some(50_000_000),
            ),
        ],
        ..Default::default()
    };
    // Extract complete: (3, 3) — every to-fetch row extracted.
    p.extract_progress = Some((3, 3));
    p
}

/// Scene: all-cached. Every row Skipped (DL-Run-1). Uniform "✓
/// downloaded" caption; Download bar 100%; no extract work.
fn scene_all_cached() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:..",
                ModDownloadStatus::Skipped,
                None,
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:..",
                ModDownloadStatus::Skipped,
                None,
                Some(80_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:..",
                ModDownloadStatus::Skipped,
                None,
                Some(60_000_000),
            ),
        ],
        ..Default::default()
    }
}

#[test]
fn render_dl_window_v3_scenes() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let cells = [Cell { w: 1280, h: 820 }, Cell { w: 1045, h: 735 }];
    let scenes: [(&str, fn() -> DownloadProgress); 6] = [
        ("hashing", scene_hashing),
        ("mid_hashing", scene_mid_hashing),
        ("mid_download", scene_mid_download),
        ("mid_extract", scene_mid_extract),
        ("preparing", scene_preparing),
        ("all_cached", scene_all_cached),
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
                "dl_window_v3_{scene_name}__{}x{}.png",
                cell.w, cell.h
            ));
            img.save(&path)
                .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
            let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
            println!(
                "SNAPSHOT  {}x{}  dl-window-v3/{scene_name}  -> {}",
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
    assert_eq!(written.len(), 12, "expected 12 PNGs (6 scenes × 2 widths)");
}

fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
