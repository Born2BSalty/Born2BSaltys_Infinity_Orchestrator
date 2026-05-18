// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the redesign **workspace Step-5
// chrome, pre-install** (Phase 7 P7.T2 / Run 1).
//
// ## Why this exists
//
// Phase 7 Run 1 ships net-new Step-5 chrome (`page_workspace_step5`): an
// empty pre-install success-banner slot + an empty pre-install
// post-install action slot ABOVE BIO's embedded `page_step5::render`
// panel (per H9). Code review, diffing, and `cargo test --lib` are
// structurally blind to layout — nothing renders the screen. Per the
// standing UI-render-gate rule (any redesign-UI change is verified by an
// `egui_kittest` rendered PNG the orchestrator opens itself, full-shell,
// multi-width), this test stands up `egui_kittest`'s wgpu renderer, paints
// the *actual* pre-install chrome inside a **shell-faithful** reproduction
// of the real app shell, and writes a PNG per width so the orchestrator
// can SEE the screen.
//
// ## What it renders — the EXACT pre-install chrome calls
//
// `page_workspace_step5::render(ui, orchestrator, modlist_id)` takes
// `&mut OrchestratorApp`, and the brief's DATA-LOSS-safety constraint
// forbids constructing a real `OrchestratorApp` / any store bound to
// `%APPDATA%\bio\`. So — exactly like `ui_snapshot_create.rs` renders
// `stage_choose::render` directly (not via `page_create` / an
// `OrchestratorApp`) — this test invokes the **same three sub-render
// calls `page_workspace_step5::render` makes, in the same order**, with a
// pure default `WizardState` + `Step5ConsoleViewState` and
// `terminal: None`:
//
//   1. `success_banner::render(ui, &wizard_state)`      (empty: C3 false)
//   2. `post_install_actions::render(ui, &wizard_state)`(empty: C3 false)
//   3. `bio::ui::step5::page_step5::render(ui, &mut wizard_state,
//        &mut console_view, /*terminal*/ None, /*err*/ None, dev_mode,
//        exe_fingerprint)`  → BIO's pre-install panel (Command card,
//        Summary card, console box, prompt input — no live child)
//
// `terminal: None` is the pre-install path: the C3 clean-exit triple is
// structurally false on a default `WizardState` (`last_exit_code ==
// None`), so the banner / post-install rows are empty and BIO paints its
// pre-install panel — exactly the Run-1 breakpoint state.
//
// ## Shell-faithful scaffold (load-bearing — mirrors ui_snapshot_create)
//
// The chrome is painted inside a structural replica of
// `shell_chrome::render_shell` + the orchestrator body — titlebar 34 +
// statusbar 26 + SidePanel 200 + page CentralPanel inner_margin
// 28/28/24/24 — built on a real `egui::Context` via
// `Harness::builder().build(...)`, reusing the read-only shell constants.
// egui derives the page content rect exactly as the live app does, so the
// snapshot reproduces the SAME margin / clip the user sees; the
// titlebar/statusbar/rail bands are flat fills (no `poppins_*` /
// `firacode_nerd` text — the font-binding note below) purely to make the
// window-edge ↔ 28px gutter relationship unmistakable.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// Pure `WizardState::default()` + `Step5ConsoleViewState::default()` +
// the public `install_redesign_fonts` (the exact font wiring
// `infinity_orchestrator`'s `main` does — BIO's `page_step5::render`
// handles its own theming; the empty pre-install redesign chrome rows
// take no palette). Constructs **no**
// `RegistryStore` / `WorkspaceStore` / `OrchestratorApp`, calls **no**
// `render_shell` (it replicates the panel scaffold structurally), and
// touches **no** `%APPDATA%` / real config dir. The seed at
// `%APPDATA%\bio\modlists.json` is never bound.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.
// An unconditional render-to-PNG (not a baseline-diff that panics on first
// run) — the gate's job is to expose/verify the layout.

use bio::app::state::WizardState;
use bio::registry::model::ModlistEntry;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::{post_install_actions, success_banner};

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

/// One matrix cell: a full window size (the shell scaffold derives the
/// page content rect exactly as the live app does).
struct Cell {
    w: u32,
    h: u32,
}

/// Render the CURRENT pre-install workspace Step-5 chrome at each width
/// (1280 / 1045 / 960 per the brief) and write a PNG for each. Each
/// absolute path is printed (run with `-- --nocapture` to see them live;
/// they are also asserted to exist + be non-empty on disk).
#[test]
fn render_workspace_step5_pre_install_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // The brief's render matrix: full-shell at the three widths 1280,
    // 1045, 960. Heights mirror `ui_snapshot_create`'s cells so the
    // page-content rect derivation is comparable across gates.
    let cells = [
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 680 },
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        let w = cell.w as f32;
        let h = cell.h as f32;

        // NOTE on font binding (verbatim rationale from
        // `ui_snapshot_create.rs`): `Context::set_fonts` queues new
        // `FontDefinitions` egui only applies at the START of the *next*
        // `begin_pass`. `Harness::builder().build()` runs an initial frame
        // immediately, so a `set_fonts` from inside the closure is one
        // frame too late for that frame. The faithful fix (matching
        // `infinity_orchestrator`'s `main`, where `install_redesign_fonts`
        // runs in the creation callback before the first `update`): on
        // frame 0 ONLY install the fonts + paint a font-neutral blank
        // panel; from frame 1 on, paint the real shell + the real chrome.
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
                // %APPDATA%. A default `WizardState` has
                // `step5.last_exit_code == None`, so the C3 clean-exit
                // triple is false ⇒ the banner / post-install rows render
                // empty and BIO paints its pre-install panel (the Run-1
                // breakpoint state). Rebuilt per frame (the chrome takes
                // `&mut WizardState`); a fresh default is deterministic
                // and identical every frame.
                let mut wizard_state = WizardState::default();
                let mut console_view = Step5ConsoleViewState::default();
                let dev_mode = false;
                let exe_fingerprint = String::new();
                // Run-3 signature update: the chrome rows now take
                // `(palette, state, entry)`. A default `WizardState` has
                // `last_exit_code == None` ⇒ the C3 triple is false ⇒ both
                // rows early-return (the empty pre-install slot — the Run-1
                // breakpoint property is unchanged). The entry is a pure
                // default stand-in (NO registry/store — DATA-LOSS-safe).
                let entry = ModlistEntry::default();
                let palette = ThemePalette::Dark;

                // ── Shell-faithful scaffold (structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator body
                //    WITHOUT a real OrchestratorApp / store / render_shell
                //    — the exact pattern in `ui_snapshot_create.rs`). ──

                // Titlebar (34px exact).
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

                // Statusbar (26px exact).
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

                // Shell body CentralPanel; inside it the rail + the page.
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                    .show(ctx, |ui| {
                        // Left nav rail (200px exact).
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

                        // Page CentralPanel with the EXACT app inner
                        // margin. Inside it: the SAME three sub-render
                        // calls `page_workspace_step5::render` makes, in
                        // the same order, with `terminal: None` (the
                        // pre-install path). The body takes the remaining
                        // vertical space + scrolls, mirroring
                        // `workspace_view`'s content wrap (so the embedded
                        // panel's `flex:1 minHeight:0` lays out the same).
                        egui::CentralPanel::default()
                            .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                            .show_inside(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        // 1. Empty pre-install success
                                        //    banner slot (C3 false).
                                        success_banner::render(ui, palette, &wizard_state, &entry);
                                        // 2. Empty pre-install post-
                                        //    install action slot (C3
                                        //    false), per H9 above the
                                        //    panel. The returned
                                        //    `Option<PostInstallAction>`
                                        //    is `None` (C3 false) — the
                                        //    gate ignores it.
                                        let _ = post_install_actions::render(
                                            ui,
                                            palette,
                                            &wizard_state,
                                            &entry,
                                        );
                                        // 3. BIO's entire pre-install
                                        //    Step-5 panel — `terminal:
                                        //    None` ⇒ Command card, Summary
                                        //    card, console box, prompt
                                        //    input, no live child. Called
                                        //    DIRECTLY (read-only; BIO
                                        //    Step-5 source untouched).
                                        let _ = bio::ui::step5::page_step5::render(
                                            ui,
                                            &mut wizard_state,
                                            &mut console_view,
                                            None, // terminal — pre-install
                                            None, // terminal_error
                                            dev_mode,
                                            &exe_fingerprint,
                                        );
                                    });
                            });
                    });
            });

        // Settle layout/fonts: frame 0 only queued fonts; run more frames
        // so the fonts bind, the atlas builds, and the panel galley
        // wrapping stabilizes before capture.
        for _ in 0..8 {
            harness.run();
        }

        let img = harness
            .render()
            .expect("egui_kittest wgpu render() must produce an image");

        let path = out_dir.join(format!(
            "workspace_step5_preinstall__{}x{}.png",
            cell.w, cell.h
        ));
        img.save(&path)
            .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

        let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
        println!(
            "SNAPSHOT  {}x{}  pre-install  -> {}",
            cell.w,
            cell.h,
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
        3,
        "expected 3 matrix-cell PNGs (the Phase-7 Run-1 widths 1280/1045/960)"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// `CARGO_TARGET_TMPDIR` is `<target>/tmp/...`; its grandparent is
/// `<target>`, keeping the path inside the git-ignored build dir
/// regardless of where `cargo test` is invoked from. (Identical resolver
/// to `ui_snapshot_create.rs`.)
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
