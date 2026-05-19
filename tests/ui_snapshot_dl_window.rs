// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// **DL-Run 2 render gate** — the Wabbajack-grade Install-Modlist
// Downloading window (SPEC §4.3): the TWO distinct phase bars (Download
// byte-aggregate then a separate Extract 0→100), byte-accurate per-mod
// bars at DISTINCT fractions (a row at 37% beside a sibling at 81% — NOT a
// clump), DL-Run-1-skipped mods rendered as instant "✓ already
// downloaded" rows, and a no-Content-Length indeterminate row.
//
// ## Why this exists (the standing UI-render-gate rule)
//
// `cargo test --lib` / diffing are structurally blind to layout — nothing
// renders the screen. Per the standing rule (every redesign-UI change is
// verified by an `egui_kittest` rendered PNG the orchestrator opens
// itself, full-shell, multi-width), this stands up `egui_kittest`'s wgpu
// renderer and paints the *actual* §4.3 chrome with a representative
// mid-pipeline `DownloadProgress` so the orchestrator can SEE: the two
// phase bars + the per-mod byte bars at distinct fills + the skipped rows.
//
// Two scenes per width:
//   • `download` — mid-download: skipped ✓ rows on top, distinct per-mod
//     byte fills (37% / 81% / a no-Content-Length marquee row), Download
//     phase bar climbing, Extract bar muted-empty (0, never inheriting).
//   • `extract` — all archives fetched: Download bar full-muted (handed
//     off), Extract phase bar the live accent 0→100, rows extracting/✓.
//
// ## What it renders — the EXACT live chrome
//
// `render_live` takes `&mut OrchestratorApp`, and the DATA-LOSS-safety
// constraint forbids constructing a real `OrchestratorApp` / any store
// bound to `%APPDATA%\bio\`. So — exactly like the sibling gates render
// the SAME sub-calls (not via an `OrchestratorApp`) — this builds a
// representative `DownloadProgress` directly (the exact value
// `from_wizard_state_full` would produce mid-pipeline) and renders it
// through the public `stage_downloading::render`, which paints the **same
// `render_chrome` body `render_live` paints**. Bit-identical to the live
// path by construction; only the data source differs.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// A synthesized `DownloadProgress` + the public `install_redesign_fonts`.
// Constructs **no** `OrchestratorApp` / `RegistryStore` / `WorkspaceStore`,
// calls **no** `render_shell`, drives **no** pipeline, and touches **no**
// `%APPDATA%` / real config dir.
//
// ## Output
//
// PNGs under the repo `target/ui-snapshots/` (deterministic, git-ignored,
// absolute path) so the orchestrator can open them directly.

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus, SkippedMod,
};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};

use eframe::egui;
use egui_kittest::Harness;

/// The real `CentralPanel` inner margin (`orchestrator_app.rs` — the exact
/// page-content margin inside the shell body, reproduced so the content
/// rect matches the app pixel-for-pixel).
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

fn skip(name: &str, source: &str, size: u64) -> SkippedMod {
    SkippedMod {
        name: name.to_string(),
        source: source.to_string(),
        size: Some(size),
    }
}

/// **Mid-download scene** — DL-Run-1-skipped mods (instant ✓), distinct
/// per-mod byte fills (37% vs 81% — proving the bars advance INDIVIDUALLY,
/// not a clump), one no-Content-Length indeterminate row, some Queued.
/// Download phase bar is the live byte aggregate; Extract bar is 0
/// (muted-empty — it NEVER inherits Download).
fn scene_download() -> DownloadProgress {
    DownloadProgress {
        rows: vec![
            row(
                "stratagems",
                "github:Gibberlings3/stratagems",
                ModDownloadStatus::Downloading,
                Some((81_000_000, Some(100_000_000))),
                Some(100_000_000),
            ),
            row(
                "item_rev",
                "github:Gibberlings3/ItemRevisions",
                ModDownloadStatus::Downloading,
                Some((37_000_000, Some(100_000_000))),
                Some(100_000_000),
            ),
            row(
                "eet_tweaks",
                "weasel:eet_tweaks",
                ModDownloadStatus::Downloading,
                // No Content-Length ⇒ honest indeterminate marquee.
                Some((5_242_880, None)),
                None,
            ),
            row(
                "cdtweaks",
                "github:Gibberlings3/cdtweaks",
                ModDownloadStatus::Extracting,
                None,
                Some(20_000_000),
            ),
            row(
                "spell_rev",
                "weasel:spell_rev",
                ModDownloadStatus::Queued,
                None,
                Some(40_000_000),
            ),
            row(
                "tweaks_anthology",
                "github:.../tweaks",
                ModDownloadStatus::Queued,
                None,
                Some(15_000_000),
            ),
        ],
        // The Wabbajack "already have it": 3 cached, instant ✓ rows.
        skipped: vec![
            skip("EET", "github:K4thos/EET", 500_000_000),
            skip("EET_end", "github:K4thos/EET_end", 50_000_000),
            skip("ascension", "github:Gibberlings3/Ascension", 8_000_000),
        ],
        ..Default::default()
    }
}

/// **Extract scene** — every archive fetched (or skipped); the Download
/// phase bar is full-but-muted (handed off), the Extract phase bar is the
/// live accent 0→100 (its OWN value), rows extracting / ✓ staged.
fn scene_extract() -> DownloadProgress {
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
                Some(100_000_000),
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
                ModDownloadStatus::Staged,
                None,
                Some(40_000_000),
            ),
            row(
                "tweaks_anthology",
                "github:..",
                ModDownloadStatus::Extracting,
                None,
                Some(15_000_000),
            ),
        ],
        skipped: vec![
            skip("EET", "github:K4thos/EET", 500_000_000),
            skip("EET_end", "github:K4thos/EET_end", 50_000_000),
            skip("ascension", "github:Gibberlings3/Ascension", 8_000_000),
        ],
        ..Default::default()
    }
}

/// Render the CURRENT live §4.3 Downloading chrome at each width
/// (1280 / 1045 / 960 per the standing matrix) × the two phase scenes.
#[test]
fn render_dl_window_two_phase_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let cells = [
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 680 },
    ];
    let scenes: [(&str, fn() -> DownloadProgress); 2] =
        [("download", scene_download), ("extract", scene_extract)];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for (scene_name, scene_fn) in scenes {
        for cell in &cells {
            let w = cell.w as f32;
            let h = cell.h as f32;

            // Font binding (verbatim rationale from the sibling gates):
            // frame 0 ONLY installs fonts + a blank panel; from frame 1
            // paint the real shell + chrome.
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

                    // ── Shell-faithful scaffold (structurally replicates
                    //    `shell_chrome::render_shell` + the orchestrator
                    //    body WITHOUT a real OrchestratorApp / store —
                    //    the exact pattern in the sibling gates). ──
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
                                    // The SAME chrome `render_live` paints
                                    // (both route through `render_chrome`).
                                    let _ = stage_downloading::render(
                                        ui,
                                        palette,
                                        DownloadScreenCopy::INSTALL,
                                        &progress,
                                    );
                                });
                        });
                });

            // Settle layout/fonts.
            for _ in 0..8 {
                harness.run();
            }

            let img = harness
                .render()
                .expect("egui_kittest wgpu render() must produce an image");

            let path = out_dir.join(format!("dl_window_{scene_name}__{}x{}.png", cell.w, cell.h));
            img.save(&path)
                .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

            let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
            println!(
                "SNAPSHOT  {}x{}  dl-window/{scene_name}  -> {}",
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
        assert!(
            meta.len() > 0,
            "rendered PNG is empty (renderer produced no pixels): {}",
            p.display()
        );
    }
    assert_eq!(
        written.len(),
        6,
        "expected 6 PNGs (2 phase scenes × the 3 standing widths 1280/1045/960)"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`
/// (identical resolver to the sibling gates).
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
