// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// **DL-Run 3 render gate** — the §4.4 Install-Modlist **completion** screen
// (`ui::install::stage_installing`) restoring the SPEC §9.2 success banner.
//
// ## The verified defect this gate proves fixed (premise SETTLED)
//
// SPEC §4.4 (line ~343) routes the post-success state to **§9.2** (Appendix
// B.2): `Return to Home` + `Open install folder`. SPEC §9.2's FIRST element
// IS the success banner — a success-green `Installed` pill +
// `<N> mods · <C> components · no errors` + a right-aligned
// `ran <MM:SS> · finished <relative>`. The §9.2-vs-§9.3 split is by install
// **state** (finished vs. during-install), NOT by entry point — the ONLY
// §4.4-specific exclusion is the Share-import-code button ("the user is
// **not** offered a Share import code button from this entry point"; that
// button lives in the workspace header, never in this screen's chrome). The
// Phase-7-arc A-1 run wrongly stripped this banner on the false premise
// that "§4.4 has no banner — it's workspace-only chrome". DL-Run 3 restores
// the C3-gated `success_banner` on the §4.4 completion screen, exactly as
// `page_workspace_step5::render` does (banner-then-post-install-row), reusing
// the existing component AS-IS.
//
// ## Why this exists (the standing UI-render-gate rule)
//
// `cargo test --lib` / diffing are structurally blind to layout — nothing
// renders the screen. Per the standing rule (every redesign-UI change is
// verified by an `egui_kittest` rendered PNG the orchestrator opens itself,
// full-shell, multi-width), this stands up `egui_kittest`'s wgpu renderer
// and paints the *actual* §4.4 completion chrome so the orchestrator can
// SEE: on a C3-clean exit the green `Installed` banner + counts +
// `ran/finished` ABOVE the `Return to Home` / `Open install folder` row,
// with the (correct) `Shared modlist` fallback title and NO
// Share-import-code button; and on a NON-clean exit the banner ABSENT
// (C3-gated — the post-install chrome does not appear).
//
// Two scenes per width:
//   • `clean`   — C3 TRUE  (`!install_running && last_exit_code==Some(0)
//                 && !last_install_failed`): banner + post-install row paint
//                 their real bodies; the `Shared modlist` fallback title.
//   • `noclean` — C3 FALSE (a cancel / nonzero exit — `last_exit_code =
//                 Some(1)`): banner + post-install row render NOTHING (the
//                 embedded panel shows BIO's pre-install/console body) —
//                 proves the banner is C3-gated, not unconditional.
//
// ## What it renders — the EXACT §4.4 completion chrome calls
//
// `stage_installing::render` takes `&mut OrchestratorApp`, and the
// DATA-LOSS-safety constraint forbids constructing a real `OrchestratorApp`
// / any store bound to `%APPDATA%\bio\`. So — exactly like the sibling
// `ui_snapshot_ws_step5_success.rs` gate renders the SAME sub-calls (not via
// an `OrchestratorApp`) — this paints the SAME chrome calls
// `stage_installing::render` makes, with a synthesized `WizardState` + a
// pure `ModlistEntry` stand-in (NO registry/store):
//
//   header:  `render_screen_title("Installing modlist",
//            "Shared modlist · live install console")` — the §4.4 header
//            with the SPEC-§4.2 honest `Shared modlist` fallback (a
//            nameless pasted code; correct per A-vi, NOT in scope to
//            change). (The screen's small `← back to import` glyph button
//            is BIO-font-dependent decorative chrome — the load-bearing
//            assertion is the title + the banner/row order; the screen-title
//            is rendered, the glyph button is intentionally omitted to keep
//            the gate to the under-test surface, matching the sibling gate's
//            "header right-cluster only" approach.)
//   1. `success_banner::render(ui, palette, &state, &entry)`  → the green
//      `Installed` pill + `<N> mods · <C> components · no errors` +
//      `ran <MM:SS> · finished <relative>` (C3 TRUE) / nothing (C3 FALSE).
//      THE SURFACE UNDER TEST — the banner DL-Run 3 restores.
//   2. `post_install_actions::render(ui, palette, &state, &entry)` →
//      `Return to Home` + `Open install folder`, immediately BELOW the
//      banner (proves the banner is ABOVE the action row, per H9 /
//      `page_workspace_step5`'s order) — and NO Share button anywhere (the
//      one §4.4-specific exclusion; `post_install_actions` never paints it).
//   3. `bio::ui::step5::page_step5::render(ui, &mut state, &mut
//      console_view, None, None, dev_mode, fp)` → BIO's Step-5 panel
//      (read-only; BIO Step-5 source untouched). Confirms there is NO
//      Share-import-code button in this screen's chrome (it would only
//      exist in the *workspace header*, which this §4.4 screen has no
//      equivalent of).
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
// PNGs under the repo `target/ui-snapshots/` (deterministic, git-ignored,
// absolute path) so the orchestrator can open them directly.

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::render_screen_title;
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

/// The exact SPEC-§4.2 honest fallback name `stage_installing::FALLBACK_NAME`
/// uses for a nameless pasted code (a `Shared modlist` — correct per A-vi,
/// NOT in scope to change). Asserted bit-identical so a future drift of the
/// fallback string is caught here too.
const FALLBACK_NAME: &str = "Shared modlist";

struct Cell {
    w: u32,
    h: u32,
}

/// Build the synthesized C3-state `WizardState`.
///
/// `clean = true`  ⇒ the C3 triple holds (`!install_running &&
/// last_exit_code == Some(0) && !last_install_failed`) ⇒ banner +
/// post-install row paint.
/// `clean = false` ⇒ a non-clean exit (`last_exit_code = Some(1)`) ⇒ the C3
/// gate is false ⇒ banner + post-install row render NOTHING (proves the
/// banner is C3-gated, not unconditional).
fn c3_state(clean: bool) -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = false;
    s.step5.last_exit_code = Some(if clean { 0 } else { 1 });
    s.step5.last_install_failed = false;
    // A representative game so BIO's pre-install panel below has content
    // to lay out (not load-bearing for the chrome under test).
    s.step1.game_install = "EET".to_string();
    s
}

/// A pure `ModlistEntry` stand-in (NO registry/store) for a **nameless
/// pasted code** — i.e. an Install-Modlist paste whose share code carried
/// no packed `name`, so the registry/screen falls back to `Shared modlist`
/// (correct per A-vi). The banner reads `mod_count` / `component_count` /
/// `install_started_at` / `install_date` — **never the name** — so it
/// renders fully even with the `Shared modlist` fallback title (the exact
/// point of DL-Run 3's "counts-based, name-independent" banner).
///
/// Built via `ModlistEntry::default()` + `pub` field setters (NOT a struct
/// literal): `forked_from` is `pub(crate)` (BIO carve-out-#5
/// `pub(crate)` `ForkAncestor`), so an external struct literal is forbidden
/// by Rust's field-visibility rule; a nameless paste's `forked_from` is the
/// default empty `Vec` anyway.
fn nameless_pasted_entry() -> ModlistEntry {
    let started = Utc::now() - ChronoDuration::seconds(4 * 60 + 12);
    let mut e = ModlistEntry::default();
    e.id = "DLBNR0000001".to_string();
    // The registry name for a nameless pasted code IS the fallback (the
    // same string the §4.4 screen renders in its header). Counts/timestamps
    // drive the banner regardless of this.
    e.name = FALLBACK_NAME.to_string();
    e.game = Game::EET;
    e.destination_folder = "D:\\import test".to_string();
    e.state = ModlistState::Installed;
    e.install_started_at = Some(started);
    e.install_date = Some(started + ChronoDuration::seconds(4 * 60 + 12));
    e.mod_count = 9;
    e.component_count = 136;
    e.latest_share_code = Some("BIO-MODLIST-V1:DLBNR".to_string());
    e
}

/// Render the CURRENT §4.4 Install-Modlist completion chrome — `clean`
/// (C3 TRUE → banner shown) and `noclean` (C3 FALSE → banner absent) — at
/// each width (1280 / 1045 / 960 per the brief) and write a PNG per
/// (scene × width).
#[test]
fn render_stage4_completion_banner_matrix() {
    // The fallback the §4.4 header renders for a nameless pasted code is
    // bit-identical to `stage_installing`'s (and `stage_preview`'s)
    // authoritative SPEC-§4.2 fallback — a future drift is caught here.
    assert_eq!(
        FALLBACK_NAME, "Shared modlist",
        "the §4.4 nameless-code fallback must match the SPEC-§4.2 string"
    );

    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // The brief's render matrix: full-shell at 1280 / 1045 / 960.
    let cells = [
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 700 },
    ];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for &clean in &[true, false] {
        let scene = if clean { "clean" } else { "noclean" };

        for cell in &cells {
            let w = cell.w as f32;
            let h = cell.h as f32;

            // Font-binding rationale (verbatim from
            // `ui_snapshot_ws_step5_success.rs`): `Context::set_fonts`
            // applies at the START of the next `begin_pass`;
            // `Harness::build()` runs an initial frame, so install fonts on
            // frame 0 (font-neutral blank) and paint the real shell + chrome
            // from frame 1 on.
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

                    // Synthesized C3 state + a nameless-pasted entry — NO
                    // OrchestratorApp / store / %APPDATA%. Rebuilt per frame
                    // (deterministic).
                    let mut wizard_state = c3_state(clean);
                    let entry = nameless_pasted_entry();
                    let mut console_view = Step5ConsoleViewState::default();
                    let palette = ThemePalette::Dark;
                    let dev_mode = false;
                    let exe_fingerprint = String::new();

                    // ── Shell-faithful scaffold (structurally replicates
                    //    `shell_chrome::render_shell` + the orchestrator
                    //    body — the exact pattern in `ui_snapshot_*`). ──

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
                                    // ── §4.4 header — `render_screen_title`
                                    //    with the SPEC-§4.2 honest
                                    //    `Shared modlist` fallback (a
                                    //    nameless pasted code; correct per
                                    //    A-vi). This is the EXACT title
                                    //    `stage_installing::render` builds:
                                    //    `Installing modlist` /
                                    //    `<name> · live install console`. ──
                                    let sub =
                                        format!("{FALLBACK_NAME} \u{00B7} live install console");
                                    render_screen_title(
                                        ui,
                                        palette,
                                        "Installing modlist",
                                        Some(&sub),
                                    );
                                    ui.add_space(10.0);

                                    egui::ScrollArea::vertical()
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            // 1. THE SURFACE UNDER TEST —
                                            //    the success banner. C3
                                            //    TRUE ⇒ green `Installed`
                                            //    pill + counts +
                                            //    ran/finished; C3 FALSE ⇒
                                            //    renders NOTHING.
                                            success_banner::render(
                                                ui,
                                                palette,
                                                &wizard_state,
                                                &entry,
                                            );
                                            // 2. Post-install action row,
                                            //    immediately BELOW the
                                            //    banner (proves banner is
                                            //    ABOVE it). C3 TRUE ⇒
                                            //    `Return to Home` + `Open
                                            //    install folder`; C3 FALSE
                                            //    ⇒ nothing. NO Share button
                                            //    is ever painted here (the
                                            //    one §4.4-specific
                                            //    exclusion). Returned action
                                            //    ignored (visual gate).
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
                                            //    Confirms no Share button
                                            //    exists in this screen's
                                            //    chrome.
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
                "stage4_completion_banner__{scene}__{}x{}.png",
                cell.w, cell.h
            ));
            img.save(&path)
                .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

            let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
            println!(
                "SNAPSHOT  {}x{}  §4.4 completion [{scene}: C3 {}]  -> {}",
                cell.w,
                cell.h,
                if clean { "TRUE" } else { "FALSE" },
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
        "expected 6 PNGs (2 scenes [clean / noclean] × 3 widths [1280/1045/960])"
    );
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// Identical resolver to `ui_snapshot_ws_step5_success.rs`.
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
