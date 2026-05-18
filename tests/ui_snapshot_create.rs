// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the redesign Create `stage_choose`
// screen (SPIKE — verification-process answer).
//
// ## Why this exists
//
// `stage_choose::render` has user-confirmed *layout* bugs (margin collapse
// at narrow widths, footer not bottom-pinned, unequal input widths/heights,
// unequal box heights when shrunk, selected-box text washed out). Code
// review, diffing, and `cargo test --lib` are structurally blind to layout —
// nothing rendered the screen before the user did. This test stands up
// `egui_kittest`'s wgpu test renderer, paints the *actual*
// `stage_choose::render` egui code inside a **shell-faithful** reproduction
// of the real app shell, and writes a PNG per matrix cell so the
// orchestrator can SEE the screen before the user.
//
// ## Shell-faithful scaffold (this is load-bearing — Fix-Run 6 Task 1)
//
// `stage_choose::render` bottom-pins its footer from `ui.available_height()`
// and computes its column widths from `ui.available_width()`. In the real
// app the screen is painted inside `shell_chrome::render_shell`
// (`shell/shell_chrome.rs` L35-55) → `orchestrator_app.rs` L778-805:
//
//   - `egui::TopBottomPanel::top("titlebar").exact_height(34)`   (titlebar)
//   - `egui::TopBottomPanel::bottom("statusbar").exact_height(26)` (statusbar)
//   - `egui::CentralPanel::default()` (the shell body), and **inside** it:
//       - `egui::SidePanel::left("rail").exact_width(200)`  (left nav rail)
//       - `egui::CentralPanel::default()
//             .frame(Frame::NONE.inner_margin({28,28,24,24}))` (the page)
//
// Earlier this gate only painted the SidePanel + page CentralPanel and
// **omitted the titlebar / statusbar bands**, so the content rect was the
// wrong *height* and its right/bottom edges had no visible relationship to
// the window frame — the user's "right margin collapses / `Start →` clipped
// at narrow width" bug was out of frame and unverifiable. This scaffold
// reproduces *exactly* that panel structure on a real `egui::Context` (via
// `Harness::builder().build(|ctx| ...)`), reusing the read-only shell
// constants (`REDESIGN_TITLEBAR_HEIGHT_PX` 34, `REDESIGN_STATUSBAR_HEIGHT_PX`
// 26, `REDESIGN_NAV_WIDTH_PX` 200) and the exact CentralPanel inner margin.
// egui itself then derives the content rect (width = window_w − 200 (rail) −
// 56 (L/R margin); height = window_h − 34 (titlebar) − 26 (statusbar) − 48
// (T/B margin)) — so the snapshot reproduces the SAME margin / footer / clip
// behavior the user sees. **The right edge of every PNG is the window
// edge**, so a collapsed right gutter shows as content touching the frame.
//
// The titlebar / statusbar / rail bands are painted with a flat fill (NO
// `poppins_*` / `firacode_nerd` text — see the font note below) purely so
// the window-edge ↔ 28px page-gutter relationship is unmistakable in the
// PNG; their content is irrelevant to the `stage_choose` layout bug.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// Pure `CreateScreenState::new()` + `ThemePalette::Dark` + the public
// `install_redesign_fonts` (the exact font wiring `infinity_orchestrator`'s
// `main` does, so Poppins / FiraCode-Nerd resolve instead of tofu/fallback).
// This test constructs **no** `RegistryStore` / `WorkspaceStore` /
// `OrchestratorApp` and calls **no** `render_shell` (it replicates the panel
// scaffold structurally) and touches **no** `%APPDATA%` / real config dir.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (a deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.
// This is intentionally an *unconditional render-to-PNG* (not a
// baseline-diff `snapshot()` that panics on first run): the spike's job is
// to expose / verify the layout, not to lock a known-good baseline.
// Promoting this to a strict committed diff-gate (golden baselines under
// `tests/snapshots/`) is a deliberate follow-up the orchestrator decides
// once the layout is fixed — the renderer here is built to be that gate.

use bio::ui::create::stage_choose;
use bio::ui::create::state_create::{CreateScreenState, StartingPoint};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};

use eframe::egui;
use egui_kittest::Harness;

/// The real `CentralPanel` inner margin (`orchestrator_app.rs` L796-801 —
/// the exact page-content margin inside the shell body, reproduced so the
/// content rect matches the app pixel-for-pixel).
const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

/// One matrix cell: a window size + the selected starting point.
struct Cell {
    w: u32,
    h: u32,
    starting_point: StartingPoint,
    /// Short tag for the filename (`scratch` / `import`).
    sp_tag: &'static str,
}

/// Render the CURRENT `stage_choose::render` at each matrix cell and write a
/// PNG for each. Each absolute path is printed (captured stdout — run with
/// `cargo test --test ui_snapshot_create -- --nocapture` to see them live;
/// they are also asserted to exist on disk).
#[test]
fn render_create_choose_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // Fix-Run 6 render matrix (full window dimensions; the shell scaffold
    // derives the page content rect exactly as the live app does). scratch
    // AND import at the wide / mid / narrow widths the user reported;
    // scratch-only at the two intermediate widths. 960×680 adds the
    // narrower cell to check the box-shrink case (P5).
    let cells = [
        // 1280x820 default window — both starting points.
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
        // ≈ the user's screenshot width where the margin was OK.
        Cell {
            w: 1045,
            h: 735,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        // ≈ the narrower screenshot where the right margin collapsed +
        //   `Start →` clipped — MUST stay within the 28px page gutter now.
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
        // A 1px-wider sibling of 1021 — checks the calc is stable across the
        // boundary the user reported (no off-by-one collapse).
        Cell {
            w: 1024,
            h: 680,
            starting_point: StartingPoint::Scratch,
            sp_tag: "scratch",
        },
        // Narrowest cell — the box-shrink case (P5: the two selectable
        // boxes must stay equal height when the column is narrow).
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
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        // Pure, isolated screen state — the SPEC §5.1 EET default. No
        // registry / workspace / OrchestratorApp / %APPDATA%.
        let mut state = CreateScreenState::new();
        state.starting_point = cell.starting_point;

        let palette = ThemePalette::Dark;
        let w = cell.w as f32;
        let h = cell.h as f32;

        // Real `egui::Context` so we can reproduce the real shell scaffold
        // (titlebar 34 + statusbar 26 + SidePanel 200 + CentralPanel
        // inner_margin 28/28/24/24). egui then derives the page content
        // rect, so the snapshot shows the SAME margin/footer/clip behavior
        // the user sees.
        //
        // NOTE on font binding: `Context::set_fonts` queues new
        // `FontDefinitions` that egui only applies at the START of the
        // *next* `begin_pass` — never mid-frame. `Harness::builder().build()`
        // runs an initial frame immediately and invokes the closure inside
        // that frame's pass, so a `set_fonts` from inside the closure (or a
        // post-`build` call) is always one frame too late for the frame that
        // tries to lay out `poppins_medium` → "not bound to any fonts" panic.
        // The faithful fix (matching `infinity_orchestrator`'s `main`, where
        // `install_redesign_fonts` runs in the creation callback *before* the
        // first `update`): on frame 0 ONLY install the fonts + paint a blank
        // panel that uses NO `poppins_*`/`firacode_nerd` family; from frame 1
        // on, the queued fonts are bound and we paint the real shell + the
        // real screen. The settle loop below then runs enough frames for the
        // font atlas + the equalized-box galley wrapping to stabilize before
        // the capture.
        let mut frame: u64 = 0;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(w, h))
            .with_pixels_per_point(1.0)
            .build(move |ctx| {
                if frame == 0 {
                    // Queue the redesign fonts (applied at the next pass)
                    // and paint a font-neutral blank panel this frame so
                    // nothing references an as-yet-unbound family.
                    install_redesign_fonts(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.allocate_space(ui.available_size());
                    });
                    frame += 1;
                    return;
                }
                frame += 1;

                // ── Shell-faithful scaffold — structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator_app
                //    body (SidePanel 200 + page CentralPanel margin
                //    28/28/24/24) WITHOUT constructing a real
                //    OrchestratorApp / store / calling render_shell. ──

                // Titlebar (34px exact) — `render_shell` L35-41.
                egui::TopBottomPanel::top("scaffold_titlebar")
                    .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        // Flat band (no Poppins/FiraCode text) so the
                        // titlebar's vertical footprint is visible in-frame.
                        let r = ui.max_rect();
                        ui.painter()
                            .rect_filled(r, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
                        ui.allocate_space(ui.available_size());
                    });

                // Statusbar (26px exact) — `render_shell` L43-49.
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

                // Shell body CentralPanel — `render_shell` L51-55. Inside it
                // the orchestrator_app paints the rail + the page.
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                    .show(ctx, |ui| {
                        // Left nav rail (200px exact) —
                        // `orchestrator_app.rs` L779-793.
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

                        // Page CentralPanel with the EXACT app inner margin
                        // — `orchestrator_app.rs` L795-804. The screen under
                        // test is painted with the SAME call shape
                        // `page_create` uses: render(ui, palette,
                        // &mut CreateScreenState).
                        egui::CentralPanel::default()
                            .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                            .show_inside(ui, |ui| {
                                let _ = stage_choose::render(ui, palette, &mut state);
                            });
                    });
            });

        // Settle layout/fonts: frame 0 (during `build`) only queued fonts;
        // run several more frames so the fonts bind, the atlas builds, and
        // the equalized-box galley wrapping stabilizes before capture.
        for _ in 0..8 {
            harness.run();
        }

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
        written.push(path);
    }

    // Every matrix cell produced a non-empty PNG on disk (the gate's
    // success contract).
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
        8,
        "expected 8 matrix-cell PNGs (the Fix-Run 6 render matrix)"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// `CARGO_TARGET_TMPDIR` is `<target>/tmp/...`; its grandparent is
/// `<target>`, keeping the path inside the git-ignored build dir regardless
/// of where `cargo test` is invoked from.
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    // <target>/tmp  ->  <target>
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
