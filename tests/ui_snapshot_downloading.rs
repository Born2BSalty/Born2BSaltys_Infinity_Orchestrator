// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the redesign **Install-Modlist
// Downloading screen with LIVE per-mod data** (Phase 7 P7.T17 / Run 4a,
// SPEC §4.3 / §13.12a).
//
// ## Why this exists
//
// P7.T17 wires the Phase-5 §4.3 chassis live: `stage_downloading::
// render_live` arms BIO's import → auto-build pipeline + interposes the
// content-addressed staging layer + feeds the §4.3 4-column grid from the
// live BIO auto-build state (`DownloadProgress::from_wizard_state`). Code
// review / `cargo test --lib` / diffing are structurally blind to layout —
// nothing renders the screen. Per the standing UI-render-gate rule (every
// redesign-UI change is verified by an `egui_kittest` rendered PNG the
// orchestrator opens itself, full-shell, multi-width), this stands up
// `egui_kittest`'s wgpu renderer and paints the *actual* §4.3 chrome with
// a representative mid-pipeline `DownloadProgress` so the orchestrator can
// SEE the live grid (mixed `queued` / `downloading N%` / `extracting...` /
// `✓ staged` rows + the moving overall bar).
//
// ## What it renders — the EXACT live chrome
//
// `render_live` takes `&mut OrchestratorApp`, and the DATA-LOSS-safety
// constraint forbids constructing a real `OrchestratorApp` / any store
// bound to `%APPDATA%\bio\`. So — exactly like `ui_snapshot_workspace_
// step5.rs` renders the SAME sub-calls `page_workspace_step5::render`
// makes (not via an `OrchestratorApp`) — this builds a representative
// `DownloadProgress` directly (the exact value
// `DownloadProgress::from_wizard_state` would produce mid-pipeline) and
// renders it through the public `stage_downloading::render`, which paints
// the **same `render_chrome` body `render_live` paints** (title +
// overall-progress Box + 4-col grid + footer). The visual is bit-identical
// to the live path by construction (both go through `render_chrome`); only
// the data source differs, and a synthesized model exercises every row
// status the live feed can produce.
//
// ## Shell-faithful scaffold (load-bearing — mirrors the sibling gates)
//
// The screen is painted inside a structural replica of `shell_chrome::
// render_shell` + the orchestrator body — titlebar 34 + statusbar 26 +
// SidePanel 200 + page CentralPanel inner_margin 28/28/24/24 — built on a
// real `egui::Context` via `Harness::builder().build(...)`, reusing the
// read-only shell constants. egui derives the page content rect exactly
// as the live app does, so the snapshot reproduces the SAME margin / clip
// the user sees.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// A synthesized `DownloadProgress` + the public `install_redesign_fonts`
// (the exact font wiring `infinity_orchestrator`'s `main` does).
// Constructs **no** `OrchestratorApp` / `RegistryStore` / `WorkspaceStore`,
// calls **no** `render_shell`, drives **no** pipeline, and touches **no**
// `%APPDATA%` / real config dir. The seed at `%APPDATA%\bio\modlists.json`
// is never bound.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.
// An unconditional render-to-PNG (not a baseline-diff that panics on first
// run) — the gate's job is to expose/verify the layout.

use bio::ui::install::stage_downloading::{
    self, DownloadProgress, DownloadScreenCopy, ModDownloadRow, ModDownloadStatus,
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

/// A representative mid-pipeline `DownloadProgress` — the exact shape
/// `DownloadProgress::from_wizard_state` produces while BIO's auto-build
/// is downloading/extracting: one row per resolved asset, every status
/// the live feed can emit (`Staged` ✓ / `Extracting` / `Downloading N%` /
/// `Queued`) so the snapshot exercises all four row tones + the moving
/// overall bar.
fn representative_live_progress() -> DownloadProgress {
    // DL-Run 2 — a row with a live byte fraction (the whole 0→1 bar) is
    // `Downloading` + `per_byte = Some((bytes, Some(total)))`; the §4.3
    // grid renders THIS mod's `bytes / Content-Length`. Distinct fractions
    // per row (37% vs 81%) prove the per-mod bars advance individually.
    let row = |name: &str,
               source: &str,
               status: ModDownloadStatus,
               per_byte: Option<(u64, Option<u64>)>| ModDownloadRow {
        name: name.to_string(),
        source: source.to_string(),
        status,
        per_byte,
        expected_size: per_byte.and_then(|(_, t)| t),
    };
    DownloadProgress {
        rows: vec![
            row("EET", "github:K4thos/EET", ModDownloadStatus::Staged, None),
            row(
                "EET_end",
                "github:K4thos/EET_end",
                ModDownloadStatus::Staged,
                None,
            ),
            row(
                "cdtweaks",
                "github:Gibberlings3/cdtweaks",
                ModDownloadStatus::Extracting,
                None,
            ),
            row(
                "stratagems",
                "github:Gibberlings3/stratagems",
                ModDownloadStatus::Downloading,
                Some((81, Some(100))),
            ),
            row(
                "item_rev",
                "github:Gibberlings3/ItemRevisions",
                ModDownloadStatus::Downloading,
                Some((37, Some(100))),
            ),
            row(
                "spell_rev",
                "weasel:spell_rev",
                ModDownloadStatus::Queued,
                None,
            ),
            row(
                "tweaks_anthology",
                "github:.../tweaks",
                ModDownloadStatus::Queued,
                None,
            ),
        ],
        // DL-Run 1 already-present-by-hash mods render as instant ✓ rows.
        skipped: vec![bio::ui::install::stage_downloading::SkippedMod {
            name: "ascension".to_string(),
            source: "github:Gibberlings3/Ascension".to_string(),
            size: Some(4096),
        }],
        ..Default::default()
    }
}

/// Render the CURRENT live §4.3 Downloading chrome at each width
/// (1280 / 1045 / 960 per the standing matrix) with a representative
/// mid-pipeline model and write a PNG for each.
#[test]
fn render_downloading_live_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let cells = [
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 680 },
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        let w = cell.w as f32;
        let h = cell.h as f32;

        // Font binding (verbatim rationale from the sibling gates):
        // `Context::set_fonts` queues `FontDefinitions` egui only applies
        // at the START of the next `begin_pass`; `Harness::builder().
        // build()` runs an initial frame immediately, so on frame 0 ONLY
        // install fonts + paint a font-neutral blank panel; from frame 1
        // paint the real shell + the real chrome.
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

                // Pure, isolated state — NO OrchestratorApp / store /
                // %APPDATA% / pipeline. The synthesized model is the exact
                // shape `from_wizard_state` produces; `render` paints the
                // SAME `render_chrome` body `render_live` paints (only the
                // data source differs), so this is the live screen's
                // pixels by construction.
                let progress = representative_live_progress();
                let palette = ThemePalette::Dark;

                // ── Shell-faithful scaffold (structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator body
                //    WITHOUT a real OrchestratorApp / store / render_shell
                //    — the exact pattern in the sibling gates). ──

                egui::TopBottomPanel::top("scaffold_titlebar")
                    .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let r = ui.max_rect();
                        ui.painter()
                            .rect_filled(r, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
                        ui.allocate_space(ui.available_size());
                    });

                egui::TopBottomPanel::bottom("scaffold_statusbar")
                    .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let r = ui.max_rect();
                        ui.painter()
                            .rect_filled(r, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
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

        let path = out_dir.join(format!("downloading_live__{}x{}.png", cell.w, cell.h));
        img.save(&path)
            .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

        let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
        println!(
            "SNAPSHOT  {}x{}  downloading-live  -> {}",
            cell.w,
            cell.h,
            abs.display()
        );
        written.push(path);
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
        3,
        "expected 3 matrix-cell PNGs (the standing widths 1280/1045/960)"
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
