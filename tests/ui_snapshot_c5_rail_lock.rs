// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the **install-running shell state**
// (Phase 7 Run 2 — P7.T9b C5 rail-nav lock + P7.T14 statusbar readout).
//
// ## Why this exists
//
// Run 2 ships the C5 rail-nav lock (`left_rail::render` with
// `rail_locked: Some(RailLockReason::InstallRunning { … })` ⇒ all four nav
// items disabled + the verbatim SPEC §13.15 tooltip) and the statusbar
// running-install readout (`shell_statusbar::render` with
// `Some(RunningInstallStatus { … })` ⇒ `… · 1 job running · <modlist> ·
// <elapsed>`). Code review / `cargo test --lib` are blind to layout —
// nothing renders the screen. Per the standing UI-render-gate rule (any
// redesign-UI change is verified by an `egui_kittest` rendered PNG the
// orchestrator opens itself, full-shell, multi-width), this test stands up
// `egui_kittest`'s wgpu renderer, paints the **real** locked rail + the
// **real** running statusbar inside a **shell-faithful** reproduction of
// the app shell, and writes a PNG per width so the orchestrator can SEE
// the install-running state.
//
// ## What it renders — the EXACT install-running widgets
//
// The DATA-LOSS-safety constraint forbids constructing a real
// `OrchestratorApp` / any store bound to `%APPDATA%\bio\`. So — exactly
// like `ui_snapshot_create.rs` / `ui_snapshot_workspace_step5.rs` render
// their target widgets directly (not via an `OrchestratorApp`) — this test
// invokes the **same two real widgets** the orchestrator's `update` loop
// calls when an install is running, with the C5 lock + the running-status
// engaged:
//
//   - `left_rail::render(ui, palette, &mut nav, dev_mode,
//        &PathValidationSummary::default(),
//        Some(&RailLockReason::InstallRunning { modlist_id, modlist_label,
//        started_at }))`  → all four nav items render disabled (idle
//        visual, dimmed text); each carries the verbatim SPEC §13.15
//        tooltip naming the running modlist (hover-only — not captured in
//        a static PNG, but the disabled visual + the no-click `Sense` are).
//   - `shell_statusbar::render(ui, palette, modlist_count,
//        Some(&RunningInstallStatus { modlist_name, elapsed }))` →
//        `● connected · <N> modlists · 1 job running · <modlist> ·
//        <elapsed>`.
//
// Pure default `NavDestination` + `PathValidationSummary::default()` + a
// hand-built `RailLockReason` / `RunningInstallStatus` — no `WizardState`,
// no `OrchestratorApp`, no store.
//
// ## Shell-faithful scaffold (load-bearing — mirrors ui_snapshot_*)
//
// The widgets are painted inside a structural replica of
// `shell_chrome::render_shell` + the orchestrator body — titlebar 34 +
// statusbar 26 (the REAL `shell_statusbar::render`) + SidePanel 200 (the
// REAL `left_rail::render`) + page CentralPanel inner_margin 28/28/24/24 —
// built on a real `egui::Context` via `Harness::builder().build(...)`,
// reusing the read-only shell constants. egui derives the rail/statusbar
// rects exactly as the live app does, so the snapshot reproduces the SAME
// disabled-rail + running-statusbar the user sees during an install.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// Constructs **no** `RegistryStore` / `WorkspaceStore` / `OrchestratorApp`,
// calls **no** `render_shell` (replicates the panel scaffold structurally),
// touches **no** `%APPDATA%` / real config dir. The seed at
// `%APPDATA%\bio\modlists.json` is never bound. Fonts via the public
// `install_redesign_fonts` (the exact wiring `infinity_orchestrator`'s
// `main` does — the locked rail paints its label/icon in `poppins_medium`
// / vectors, so the fonts must bind before capture).
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.
//
// ## File name (a recorded Windows footgun)
//
// Deliberately `ui_snapshot_c5_rail_lock` (NOT `…_install_running`):
// Windows' UAC **Installer Detection** heuristic auto-flags any freshly-
// built `.exe` whose name contains `install` / `setup` / `update` /
// `patch` and refuses to launch it without elevation (`os error 740` —
// "The requested operation requires elevation"). The compiled
// integration-test binary is named after this file; the "install"
// substring is avoided so the gate runs unelevated exactly like its
// `ui_snapshot_create` / `ui_snapshot_workspace_step5` siblings.

use std::time::{Duration, Instant};

use bio::install_runtime::rail_lock_reason::RailLockReason;
use bio::ui::orchestrator::left_rail;
use bio::ui::orchestrator::nav_destination::NavDestination;
use bio::ui::orchestrator::nav_status::PathValidationSummary;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::shell::shell_statusbar::{self, RunningInstallStatus};

use eframe::egui;
use egui_kittest::Harness;

/// The real `CentralPanel` inner margin (`orchestrator_app.rs` — the exact
/// page-content margin inside the shell body).
const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

/// One matrix cell: a full window size (the shell scaffold derives the
/// rail/statusbar rects exactly as the live app does).
struct Cell {
    w: u32,
    h: u32,
}

/// Render the CURRENT install-running shell state (the C5-locked rail +
/// the running statusbar) at each width (1280 / 1045 per the brief) and
/// write a PNG for each.
#[test]
fn render_c5_rail_lock_shell_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // The brief's render matrix: full-shell at the two widths 1280, 1045.
    let cells = [Cell { w: 1280, h: 820 }, Cell { w: 1045, h: 735 }];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        let w = cell.w as f32;
        let h = cell.h as f32;

        // Font-binding note (verbatim rationale from the sibling gates):
        // `Context::set_fonts` queues `FontDefinitions` egui applies at the
        // START of the next `begin_pass`; `Harness::builder().build()` runs
        // an initial frame immediately, so a `set_fonts` from inside the
        // closure is one frame too late for that frame. The faithful fix
        // (matching `infinity_orchestrator`'s `main`): frame 0 ONLY installs
        // the fonts + paints a font-neutral blank panel; from frame 1 on,
        // paint the real shell + the real locked rail + running statusbar.
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

                let palette = ThemePalette::Dark;

                // The install-running state, hand-built (NO OrchestratorApp
                // / store / %APPDATA%). A monotonic `Instant` 3m07s in the
                // past so the statusbar `<elapsed>` reads a realistic
                // `03:07` (the exact `format_elapsed` the live statusbar
                // uses). The rail-lock reason carries the registry-resolved
                // display name the orchestrator would pass.
                let started_at = Instant::now()
                    .checked_sub(Duration::from_secs(3 * 60 + 7))
                    .unwrap_or_else(Instant::now);
                let modlist_label = "Polished BG2EE".to_string();
                let rail_lock = RailLockReason::InstallRunning {
                    modlist_id: "RUN1BBBBBBBB".to_string(),
                    modlist_label: modlist_label.clone(),
                    started_at,
                };
                let running_status = RunningInstallStatus {
                    modlist_name: modlist_label.clone(),
                    elapsed: started_at.elapsed(),
                };
                let mut nav = NavDestination::Workspace {
                    modlist_id: Some("RUN1BBBBBBBB".to_string()),
                };
                let validation = PathValidationSummary::default();

                // ── Shell-faithful scaffold (structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator body
                //    WITHOUT a real OrchestratorApp / store / render_shell
                //    — the exact pattern in the sibling gates). ──

                // Titlebar (34px exact) — flat fill (font-neutral band).
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

                // Statusbar (26px exact) — the REAL `shell_statusbar
                // ::render` with the running-install readout (P7.T14).
                egui::TopBottomPanel::bottom("scaffold_statusbar")
                    .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        shell_statusbar::render(ui, palette, 2, Some(&running_status));
                    });

                // Shell body CentralPanel; inside it the REAL locked rail
                // + an empty page (the page content is not the subject of
                // this gate — the C5 rail lock + the statusbar are).
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                    .show(ctx, |ui| {
                        // Left nav rail (200px exact) — the REAL
                        // `left_rail::render` with the C5 lock engaged
                        // (P7.T9b): all four items disabled, dimmed,
                        // no-click, verbatim SPEC §13.15 tooltip.
                        egui::SidePanel::left("scaffold_rail")
                            .exact_width(REDESIGN_NAV_WIDTH_PX)
                            .resizable(false)
                            .show_separator_line(false)
                            .frame(egui::Frame::NONE)
                            .show_inside(ui, |ui| {
                                left_rail::render(
                                    ui,
                                    palette,
                                    &mut nav,
                                    /* dev_mode */ false,
                                    &validation,
                                    Some(&rail_lock),
                                );
                            });

                        // Page CentralPanel with the EXACT app inner margin
                        // (empty — the running install's workspace body is
                        // not what this gate verifies).
                        egui::CentralPanel::default()
                            .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                            .show_inside(ui, |ui| {
                                ui.allocate_space(ui.available_size());
                            });
                    });
            });

        // Settle layout/fonts: frame 0 only queued fonts; run more frames
        // so the fonts bind, the atlas builds, and the rail/statusbar
        // galleys stabilize before capture.
        for _ in 0..8 {
            harness.run();
        }

        let img = harness
            .render()
            .expect("egui_kittest wgpu render() must produce an image");

        let path = out_dir.join(format!("c5_rail_lock_shell__{}x{}.png", cell.w, cell.h));
        img.save(&path)
            .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

        let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
        println!(
            "SNAPSHOT  {}x{}  install-running (C5 rail lock + statusbar)  -> {}",
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
        2,
        "expected 2 matrix-cell PNGs (the Phase-7 Run-2 widths 1280/1045)"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// Identical resolver to the sibling gates.
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
