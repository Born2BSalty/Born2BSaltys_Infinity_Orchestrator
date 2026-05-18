// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Headless rendered-snapshot gate for the redesign **Install-Modlist
// Stage 4 — the §4.4 `InstallProgressScreen`** (Phase 7 P7.T15 / Run 4b).
//
// ## Why this exists
//
// P7.T15 replaces the Phase-5/Run-4a stage-4 stub with the real §4.4
// screen: a simple header (`Installing modlist · <name> · live install
// console` + a back affordance), the C3-gated post-install action row
// ABOVE BIO's embedded panel (per H9 — `Return to Home` + `Open install
// folder`, **NO Share import code**: the user pasted the code), and BIO's
// entire `page_step5::render` panel. It does **NOT** use the workspace
// 4-step progress bar / nav bar / Save-Draft / Share-import header. Code
// review / `cargo test --lib` / diffing are structurally blind to layout —
// nothing renders the screen. Per the standing UI-render-gate rule (every
// redesign-UI change is verified by an `egui_kittest` rendered PNG the
// orchestrator opens itself, full-shell, multi-width), this stands up
// `egui_kittest`'s wgpu renderer and paints the **actual** §4.4 chrome
// inside a **shell-faithful** reproduction of the app shell, with a
// synthesized **install-running** `WizardState` (the canonical
// `InstallProgressScreen` state — the C3 triple FALSE, so the post-install
// row is the empty during-install slot and BIO's panel shows the live
// console / Cancel-Install), and writes a PNG per width.
//
// ## Test-name footgun (HARD this run — the screen is literally
// "installing")
//
// The binary name MUST contain NONE of `install` / `setup` / `update` /
// `patch` — those trigger Windows UAC Installer Detection (`os error
// 740`). Hence **`ui_snapshot_stage4_progress`** (and the test fn
// `render_stage4_progress_screen_matrix`) — `stage4_progress` is safe;
// double-checked the file name AND the test-fn name.
//
// ## What it renders — the EXACT §4.4 chrome calls
//
// `stage_installing::render` takes `&mut OrchestratorApp`, and the
// DATA-LOSS-safety constraint forbids constructing a real
// `OrchestratorApp` / any store bound to `%APPDATA%\bio\`. So — exactly
// like `ui_snapshot_ws_step5_success.rs` paints the SAME sub-calls
// `page_workspace_step5::render` makes (not via an `OrchestratorApp`) —
// this paints the SAME sub-calls `stage_installing::render` makes:
//
//   header:  `Installing modlist · <name> · live install console` (the
//            exact RichText the screen paints) + a `← Back to preview`
//            `redesign_btn(small)` (the screen's back affordance — the
//            workspace progress bar / nav bar / Share header are
//            deliberately ABSENT, which is what this gate proves).
//   1. `post_install_actions::render(ui, palette, &state, &entry)` → the
//      C3-gated post-install row. With the synthesized state INSTALL-
//      RUNNING (C3 FALSE) it renders **nothing** — the empty during-install
//      slot (proving the screen has no Share row + no workspace banner
//      mid-install).
//   2. `bio::ui::step5::page_step5::render(ui, &mut state, &mut
//      console_view, /*terminal*/ None, /*err*/ None, dev_mode, fp)` →
//      BIO's Step-5 panel (its internal Cancel-Install / Actions /
//      Diagnostics / Prompt-Answers / console / prompt-input is
//      BIO-internal; what THIS gate proves is the §4.4 chrome AROUND it +
//      the ABSENCE of the workspace chrome).
//
// (`Step5Action::StartInstall` dispatch + the post-install nav/open-folder
// side-effects need a live `OrchestratorApp`, forbidden here — they are
// `cargo test --lib`-covered + the manual breakpoint; this is a *visual*
// gate, identical in spirit to `ui_snapshot_ws_step5_success.rs`.)
//
// ## Shell-faithful scaffold (load-bearing — mirrors ui_snapshot_*)
//
// The chrome is painted inside a structural replica of `shell_chrome::
// render_shell` + the orchestrator body — titlebar 34 + statusbar 26 +
// SidePanel 200 + page CentralPanel inner_margin 28/28/24/24 — built on a
// real `egui::Context` via `Harness::builder().build(...)`, reusing the
// read-only shell constants.
//
// ## Test hygiene (directive-grade — DATA-LOSS)
//
// Synthesized `WizardState` + a pure `ModlistEntry` stand-in +
// `Step5ConsoleViewState` only. Constructs **no** `RegistryStore` /
// `WorkspaceStore` / `OrchestratorApp`, calls **no** `render_shell` /
// `RegistryStore::save`, drives **no** install, touches **no** `%APPDATA%`
// / real config dir. `modlists.json` is never bound.
//
// ## Output
//
// Renders to PNGs under the repo `target/ui-snapshots/` (deterministic,
// git-ignored, absolute path) so the orchestrator can open them directly.

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::render_screen_title;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_WIDTH_PX,
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
    redesign_border_strong, redesign_shell_bg, redesign_text_primary,
};
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::post_install_actions;

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

/// The synthesized **install-running** `WizardState` — the canonical §4.4
/// `InstallProgressScreen` state. The C3 triple
/// (`success_banner::clean_exit`) is FALSE (install_running == true), so
/// the post-install row renders nothing (the empty during-install slot)
/// and BIO's embedded panel shows the live-install body (Cancel Install /
/// console). This is exactly what the screen looks like while the install
/// runs.
fn install_running_state() -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = true;
    s.step5.last_status_text = "Installing component 12 / 136".to_string();
    // A representative game so BIO's panel has content to lay out (not
    // load-bearing for the §4.4 chrome under test).
    s.step1.game_install = "EET".to_string();
    s
}

/// A pure `ModlistEntry` stand-in (NO registry/store). For the §4.4 screen
/// only its identity is used (`post_install_actions::render` takes it for
/// gating/identity symmetry; the row is hidden mid-install anyway). Built
/// via `ModlistEntry::default()` + `pub` field setters (NOT a struct
/// literal — `forked_from` is `pub(crate)`, holding BIO's carve-out-#5
/// `pub(crate)` `ForkAncestor`; a from-scratch modlist's `forked_from` is
/// the default empty `Vec` anyway).
fn install_modlist_entry() -> ModlistEntry {
    let mut e = ModlistEntry::default();
    e.id = "SNAP00000004".to_string();
    e.name = "Polished EET".to_string();
    e.game = Game::EET;
    e.destination_folder = "D:\\import test".to_string();
    e.state = ModlistState::InProgress;
    e.latest_share_code = Some("BIO-MODLIST-V1:SNAPSHOT".to_string());
    e
}

/// The packed name the §4.4 header shows (the same honest-fallback derive
/// `stage_installing::render` performs from the parsed preview — here
/// hard-set to the representative name; the fallback path is unit-tested in
/// `stage_installing`).
const HEADER_NAME: &str = "Polished EET";

/// Render the CURRENT §4.4 `InstallProgressScreen` chrome at each width
/// (1280 / 1045 per the brief) and write a PNG for each.
#[test]
fn render_stage4_progress_screen_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    // The brief's render matrix: full-shell at 1280 and 1045.
    let cells = [Cell { w: 1280, h: 820 }, Cell { w: 1045, h: 735 }];

    let mut written: Vec<std::path::PathBuf> = Vec::new();

    for cell in &cells {
        let w = cell.w as f32;
        let h = cell.h as f32;

        // Font-binding rationale (verbatim from the sibling gates):
        // `Context::set_fonts` applies at the START of the next
        // `begin_pass`; `Harness::build()` runs an initial frame, so
        // install fonts on frame 0 (font-neutral blank) and paint the real
        // shell + chrome from frame 1 on.
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

                // Synthesized install-running state + entry — NO
                // OrchestratorApp / store / %APPDATA%. Rebuilt per frame
                // (deterministic).
                let mut wizard_state = install_running_state();
                let entry = install_modlist_entry();
                let mut console_view = Step5ConsoleViewState::default();
                let palette = ThemePalette::Dark;
                let dev_mode = false;
                let exe_fingerprint = String::new();

                // ── Shell-faithful scaffold (structurally replicates
                //    `shell_chrome::render_shell` + the orchestrator body —
                //    the exact pattern in `ui_snapshot_*`). ──

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
                                // ── 1. The §4.4 header — the EXACT
                                //    wireframe `InstallProgressScreen`
                                //    chrome `stage_installing::render`
                                //    paints: `render_screen_title("Installing
                                //    modlist", "<name> · live install
                                //    console")` (left, width-reserved) + a
                                //    flush-right glyph-aware `← back to
                                //    import` small button (the `←` in
                                //    `firacode_nerd`, prose in
                                //    `poppins_medium`, side by side —
                                //    replicated here exactly because the
                                //    screen's helper is private, same as
                                //    `ui_snapshot_ws_step5_success.rs`
                                //    replicates the header Share button).
                                //    The workspace 4-step progress bar /
                                //    nav bar / Save-Draft / Share header are
                                //    deliberately ABSENT — that absence is
                                //    what this gate proves. ──
                                let sub = format!("{HEADER_NAME} \u{00B7} live install console");
                                ui.horizontal_top(|ui| {
                                    let back_btn_w = 130.0;
                                    let title_w = (ui.available_width() - back_btn_w).max(160.0);
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(title_w, ui.available_height()),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            render_screen_title(
                                                ui,
                                                palette,
                                                "Installing modlist",
                                                Some(&sub),
                                            );
                                        },
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Min),
                                        |ui| {
                                            ui.add_space(0.0);
                                            paint_back_to_import_btn(ui, palette);
                                        },
                                    );
                                });
                                ui.add_space(10.0);

                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        // 2. C3-gated post-install row.
                                        //    Install-running ⇒ C3 FALSE ⇒
                                        //    renders nothing (the empty
                                        //    during-install slot — NO
                                        //    Share, NO workspace banner).
                                        //    The returned action is ignored
                                        //    (no orchestrator — this is a
                                        //    visual gate).
                                        let _ = post_install_actions::render(
                                            ui,
                                            palette,
                                            &wizard_state,
                                            &entry,
                                        );
                                        // 3. BIO's entire Step-5 panel —
                                        //    `terminal: None`. Called
                                        //    DIRECTLY (read-only; BIO
                                        //    Step-5 source untouched). With
                                        //    install_running == true BIO
                                        //    paints the live-install body
                                        //    (Cancel Install / console /
                                        //    prompt input).
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

        let path = out_dir.join(format!("stage4_progress__{}x{}.png", cell.w, cell.h));
        img.save(&path)
            .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

        let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
        println!(
            "SNAPSHOT  {}x{}  stage-4 InstallProgressScreen (install-running)  -> {}",
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
        "expected 2 matrix-cell PNGs (the Phase-7 Run-4b widths 1280/1045)"
    );
}

/// Replica of `stage_installing`'s private `back_to_import_btn` (the §4.4
/// wireframe back affordance — `← back to import`, `←` in `firacode_nerd`,
/// prose in `poppins_medium`, side by side, sketchy-bordered small button).
/// Replicated here because that helper is private — the SAME visual-gate
/// replication pattern `ui_snapshot_ws_step5_success.rs` uses for the
/// private header Share button. Bit-identical to the screen's helper.
fn paint_back_to_import_btn(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let gap = 5.0;

    let fill = redesign_shell_bg(palette);
    let text_color = redesign_text_primary(palette);
    let border = redesign_border_strong(palette);

    let glyph_font = egui::FontId::new(font_size, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));

    let glyph_galley =
        ui.painter()
            .layout_no_wrap("\u{2190}".to_string(), glyph_font.clone(), text_color);
    let prose_galley =
        ui.painter()
            .layout_no_wrap("back to import".to_string(), prose_font.clone(), text_color);

    let content_w = glyph_galley.size().x + gap + prose_galley.size().x;
    let content_h = glyph_galley.size().y.max(prose_galley.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);

    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8);
        painter.rect_filled(rect, radius, fill);
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, border),
            egui::StrokeKind::Inside,
        );
        let start_x = rect.center().x - content_w / 2.0;
        let cy = rect.center().y;
        painter.text(
            egui::pos2(start_x, cy),
            egui::Align2::LEFT_CENTER,
            "\u{2190}",
            glyph_font,
            text_color,
        );
        painter.text(
            egui::pos2(start_x + glyph_galley.size().x + gap, cy),
            egui::Align2::LEFT_CENTER,
            "back to import",
            prose_font,
            text_color,
        );
    }

    response
}

/// Deterministic absolute output dir: `<repo>/target/ui-snapshots/`.
/// Identical resolver to the sibling `ui_snapshot_*` gates.
fn snapshot_out_dir() -> std::path::PathBuf {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
