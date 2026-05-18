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
// unequal box heights when shrunk). Code review, diffing, and
// `cargo test --lib` are structurally blind to layout — nothing rendered
// the screen before the user did. This test stands up `egui_kittest`'s
// wgpu test renderer, paints the *actual* `stage_choose::render` egui code
// inside a faithful reproduction of the real app shell, and writes a PNG
// per matrix cell so the orchestrator can SEE the screen before the user.
//
// ## Faithful shell (this is load-bearing)
//
// `stage_choose::render` bottom-pins its footer from `ui.available_height()`
// and computes its column widths from `ui.available_width()`. In the real
// app (`orchestrator_app.rs` ~L778-805) the screen is painted inside:
//   - a left `SidePanel` of exactly `REDESIGN_NAV_WIDTH_PX` (200px), then
//   - a `CentralPanel` with `inner_margin { left:28, right:28, top:24,
//     bottom:24 }`.
// So we reproduce *exactly that* panel structure on a real `egui::Context`
// (via `Harness::builder().build(|ctx| ...)`). egui itself then derives the
// content rect: width = window_w − 200 (rail) − 56 (L/R margin); height =
// window_h − 48 (T/B margin) — i.e. the brief's
// `window_width − 200 − 56` content width, computed by egui rather than
// hardcoded, so the snapshot reproduces the SAME margin / footer / clip
// behavior the user sees. Painting `stage_choose` in a bare full-window
// `Ui` would NOT reproduce the margin-collapse / clip bug, which is the
// whole point of the gate.
//
// ## Test hygiene (directive-grade)
//
// Pure `CreateScreenState::new()` + `ThemePalette::Dark` + the public
// `install_redesign_fonts` (the exact font wiring `infinity_orchestrator`'s
// `main` does, so Poppins / FiraCode-Nerd resolve instead of tofu/fallback).
// This test constructs **no** `RegistryStore` / `WorkspaceStore` /
// `OrchestratorApp` and touches **no** `%APPDATA%` / real config dir.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (a deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.
// This is intentionally an *unconditional render-to-PNG* (not a
// baseline-diff `snapshot()` that panics on first run): the spike's job is
// to expose the current bug, not to lock a known-good baseline. Promoting
// this to a strict committed diff-gate (golden baselines under
// `tests/snapshots/`) is a deliberate follow-up the orchestrator decides
// once the layout is fixed — the renderer here is built to be that gate,
// not throwaway.

use bio::ui::create::stage_choose;
use bio::ui::create::state_create::{CreateScreenState, StartingPoint};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::ThemePalette;

use eframe::egui;
use egui_kittest::Harness;

/// The real left-rail width (`REDESIGN_NAV_WIDTH_PX`), reproduced so the
/// content rect width matches the app exactly.
const NAV_WIDTH_PX: f32 = 200.0;

/// The real `CentralPanel` inner margin (`orchestrator_app.rs` ~L796-801).
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
        //   `Start →` clipped — MUST reproduce the bug if the gate is faithful.
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

        // Real `egui::Context` so we can reproduce the real panel shell
        // (SidePanel 200 + CentralPanel inner_margin 28/28/24/24). egui then
        // derives the content rect, so the snapshot shows the SAME
        // margin/footer behavior the user sees.
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
        // first `update`): on frame 0 ONLY install the fonts + paint blank
        // panels that use NO `poppins_*`/`firacode_nerd` family; from frame 1
        // on, the queued fonts are bound and we paint the real screen. The
        // settle loop below then runs enough frames for the font atlas + the
        // equalized-box galley wrapping to stabilize before the capture.
        let mut frame: u64 = 0;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(w, h))
            .with_pixels_per_point(1.0)
            .build(move |ctx| {
                if frame == 0 {
                    // Queue the redesign fonts (applied at the next pass)
                    // and paint a font-neutral blank shell this frame so
                    // nothing references an as-yet-unbound family.
                    install_redesign_fonts(ctx);
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.allocate_space(ui.available_size());
                    });
                    frame += 1;
                    return;
                }
                frame += 1;

                egui::SidePanel::left("rail_repro")
                    .exact_width(NAV_WIDTH_PX)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        // The rail content is irrelevant to the
                        // stage_choose layout bug; reserve the exact width
                        // (so the CentralPanel's available width matches the
                        // app) and leave it blank.
                        ui.set_min_width(NAV_WIDTH_PX);
                        ui.allocate_space(ui.available_size());
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                    .show(ctx, |ui| {
                        // The screen under test, painted with the SAME call
                        // shape `page_create` uses: render(ui, palette,
                        // &mut CreateScreenState).
                        let _ = stage_choose::render(ui, palette, &mut state);
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
        5,
        "expected 5 matrix-cell PNGs (the brief's render matrix)"
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
