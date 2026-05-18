// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the redesign **workspace Step-5
// chrome, POST-install (the C3 clean-exit success state)** — Phase 7
// P7.T4 / P7.T5 / P7.T7 / Run 3.
//
// ## Why this exists
//
// Run 3 fills the Run-1 chrome stubs: the success banner (green
// `Installed` pill + `<N> mods · <C> components · no errors` +
// `ran <MM:SS> · finished <relative>`), the post-install action row
// (`Return to Home` + `Open install folder`, per H9 ABOVE BIO's embedded
// panel — visually adjacent to BIO's now-disabled `✓ Installed` button),
// and the workspace header's `Share import code` button flipping to a
// primary-teal CTA. Code review / `cargo test --lib` are blind to layout —
// nothing renders the screen. Per the standing UI-render-gate rule (any
// redesign-UI change is verified by an `egui_kittest` rendered PNG the
// orchestrator opens itself, full-shell, multi-width), this test stands up
// `egui_kittest`'s wgpu renderer, paints the **real** post-install chrome
// inside a **shell-faithful** reproduction of the app shell with the C3
// triple **TRUE**, and writes a PNG per width so the orchestrator can SEE
// the success screen.
//
// ## Test-name footgun (Run-2 catch)
//
// The binary name MUST NOT contain `install` / `setup` / `update` /
// `patch` — those trigger Windows UAC Installer Detection (`os error
// 740`). Hence `ui_snapshot_ws_step5_success` (no banned token —
// "success", not "postinstall").
//
// ## What it renders — the EXACT post-install chrome calls
//
// The DATA-LOSS-safety constraint forbids constructing a real
// `OrchestratorApp` / any store bound to `%APPDATA%\bio\`. So — exactly
// like `ui_snapshot_create.rs` / `ui_snapshot_workspace_step5.rs` —
// this test paints the **same chrome calls** the post-install workspace
// makes, with a **synthesized** C3-TRUE `WizardState` + a pure
// `ModlistEntry` stand-in (NO registry/store):
//
//   header row:  the `Share import code` button in its **C3-true state**
//                (the exact `redesign_btn(small, primary)` the Phase-6
//                `workspace_header::render_save_or_share_button` paints
//                when `success_banner::clean_exit` holds — rendered
//                directly here because that header fn takes `&mut
//                OrchestratorApp`, forbidden by DATA-LOSS; the visual is
//                identical).
//   1. `success_banner::render(ui, palette, &state, &entry)`  → the
//      green `Installed` pill + counts + `ran · finished` (C3 TRUE).
//   2. `post_install_actions::render(ui, palette, &state, &entry)` →
//      `Return to Home` + `Open install folder`, ABOVE the panel (H9).
//   3. `bio::ui::step5::page_step5::render(ui, &mut state, &mut
//      console_view, /*terminal*/ None, /*err*/ None, dev_mode, fp)` →
//      BIO's Step-5 panel (the install row at its top — its `✓ Installed`
//      disabled state is BIO-internal; what THIS gate proves is the
//      chrome ABOVE it).
//
// The C3 triple is forced TRUE on the synthesized state
// (`install_running = false`, `last_exit_code = Some(0)`,
// `last_install_failed = false`), so the banner + post-install row paint
// their real bodies and the header button is primary — the post-install
// breakpoint state.
//
// ## Shell-faithful scaffold (load-bearing — mirrors ui_snapshot_*)
//
// The chrome is painted inside a structural replica of
// `shell_chrome::render_shell` + the orchestrator body — titlebar 34 +
// statusbar 26 + SidePanel 200 + page CentralPanel inner_margin
// 28/28/24/24 — built on a real `egui::Context` via
// `Harness::builder().build(...)`, reusing the read-only shell constants.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// Synthesized `WizardState` + `ModlistEntry` + `Step5ConsoleViewState`
// only. Constructs **no** `RegistryStore` / `WorkspaceStore` /
// `OrchestratorApp`, calls **no** `render_shell` / `RegistryStore::save`,
// touches **no** `%APPDATA%` / real config dir. `modlists.json` is never
// bound.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::{post_install_actions, success_banner};

use chrono::{Duration as ChronoDuration, Utc};
use eframe::egui;
use egui_kittest::Harness;

/// The real `CentralPanel` inner margin (`orchestrator_app.rs`).
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

/// Build the synthesized C3-TRUE `WizardState` (the success state). The
/// banner / post-install row / header-button flip ALL gate on
/// `success_banner::clean_exit` — force its triple true.
fn c3_clean_state() -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = false;
    s.step5.last_exit_code = Some(0);
    s.step5.last_install_failed = false;
    // A representative Step-3 selection so BIO's pre-install panel below
    // has content to lay out (not load-bearing for the chrome under test).
    s.step1.game_install = "EET".to_string();
    s
}

/// A pure post-install `ModlistEntry` stand-in (NO registry/store). Counts
/// + timestamps drive the banner's `<N> mods · <C> components` and
/// `ran <MM:SS> · finished <relative>`.
///
/// Built via `ModlistEntry::default()` + the `pub` field setters (NOT a
/// struct literal): `forked_from` is `pub(crate)` (it holds BIO's
/// carve-out-#5 `pub(crate)` `ForkAncestor`), so an external struct literal
/// is forbidden by Rust's field-visibility rule — and a from-scratch
/// (non-forked) modlist's `forked_from` is correctly the default empty
/// `Vec` anyway, so nothing needs to set it.
fn installed_entry() -> ModlistEntry {
    let started = Utc::now() - ChronoDuration::seconds(4 * 60 + 12);
    let mut e = ModlistEntry::default();
    e.id = "SNAP00000001".to_string();
    e.name = "Polished EET".to_string();
    e.game = Game::EET;
    e.destination_folder = "D:\\import test".to_string();
    e.state = ModlistState::Installed;
    e.install_started_at = Some(started);
    e.install_date = Some(started + ChronoDuration::seconds(4 * 60 + 12));
    e.mod_count = 9;
    e.component_count = 136;
    e.latest_share_code = Some("BIO-MODLIST-V1:SNAPSHOT".to_string());
    e
}

/// Render the CURRENT post-install workspace Step-5 chrome at each width
/// (1280 / 1045 per the brief) and write a PNG for each.
#[test]
fn render_workspace_step5_post_install_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // The brief's render matrix: full-shell at 1280 and 1045.
    let cells = [Cell { w: 1280, h: 820 }, Cell { w: 1045, h: 735 }];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        let w = cell.w as f32;
        let h = cell.h as f32;

        // Font-binding rationale (verbatim from
        // `ui_snapshot_workspace_step5.rs`): `Context::set_fonts` applies
        // at the START of the next `begin_pass`; `Harness::build()` runs an
        // initial frame, so install fonts on frame 0 (font-neutral blank)
        // and paint the real shell + chrome from frame 1 on.
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

                // Synthesized C3-TRUE state + entry — NO OrchestratorApp /
                // store / %APPDATA%. Rebuilt per frame (deterministic).
                let mut wizard_state = c3_clean_state();
                let entry = installed_entry();
                let mut console_view = Step5ConsoleViewState::default();
                let palette = ThemePalette::Dark;
                let dev_mode = false;
                let exe_fingerprint = String::new();

                // ── Shell-faithful scaffold (structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator body
                //    — the exact pattern in `ui_snapshot_*`). ──

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
                                // ── Header right-cluster: the `Share
                                //    import code` button in its C3-TRUE
                                //    state — the EXACT
                                //    `redesign_btn(small, primary)` the
                                //    Phase-6 `workspace_header` paints when
                                //    `clean_exit` holds. Rendered directly
                                //    (that header fn takes `&mut
                                //    OrchestratorApp`, forbidden by
                                //    DATA-LOSS); the visual is identical.
                                //    Right-aligned in a FIXED-height band
                                //    (the `confirm_dialog.rs` rationale —
                                //    an unbounded `right_to_left`
                                //    `with_layout` claims the full
                                //    remaining column height and would push
                                //    the banner/panel off-screen). ──
                                let header_h = 28.0;
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), header_h),
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let _ = redesign_btn(
                                            ui,
                                            palette,
                                            "Share import code",
                                            BtnOpts {
                                                small: true,
                                                primary: true,
                                                ..Default::default()
                                            },
                                        );
                                    },
                                );
                                ui.add_space(10.0);

                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        // 1. Success banner (C3 TRUE ⇒
                                        //    green `Installed` pill +
                                        //    counts + ran/finished).
                                        success_banner::render(ui, palette, &wizard_state, &entry);
                                        // 2. Post-install action row
                                        //    (C3 TRUE ⇒ `Return to Home` +
                                        //    `Open install folder`), per
                                        //    H9 ABOVE the panel. The
                                        //    returned action is ignored
                                        //    (no orchestrator to apply it
                                        //    against — this is a visual
                                        //    gate).
                                        let _ = post_install_actions::render(
                                            ui,
                                            palette,
                                            &wizard_state,
                                            &entry,
                                        );
                                        // 3. BIO's entire Step-5 panel —
                                        //    `terminal: None`. Called
                                        //    DIRECTLY (read-only; BIO
                                        //    Step-5 source untouched).
                                        let _ = bio::ui::step5::page_step5::render(
                                            ui,
                                            &mut wizard_state,
                                            &mut console_view,
                                            None, // terminal
                                            None, // terminal_error
                                            dev_mode,
                                            &exe_fingerprint,
                                        );
                                    });
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
            "workspace_step5_success__{}x{}.png",
            cell.w, cell.h
        ));
        img.save(&path)
            .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

        let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
        println!(
            "SNAPSHOT  {}x{}  post-install (C3 TRUE)  -> {}",
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
        "expected 2 matrix-cell PNGs (the Phase-7 Run-3 widths 1280/1045)"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// Identical resolver to `ui_snapshot_workspace_step5.rs`.
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
