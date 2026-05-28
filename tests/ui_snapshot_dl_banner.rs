// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::render_screen_title;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::step5::content_step5::Step5RenderCtx;
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::{post_install_actions, success_banner};

use chrono::{Duration as ChronoDuration, Utc};
use eframe::egui;
use egui_kittest::Harness;
use std::path::{Path, PathBuf};

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const FALLBACK_NAME: &str = "Shared modlist";

const CELLS: [Cell; 3] = [
    Cell {
        w: 1280.0,
        h: 820.0,
        suffix: "1280x820",
    },
    Cell {
        w: 1045.0,
        h: 735.0,
        suffix: "1045x735",
    },
    Cell {
        w: 960.0,
        h: 700.0,
        suffix: "960x700",
    },
];

const SCENES: [Scene; 2] = [
    Scene {
        clean: true,
        name: "clean",
        label: "TRUE",
    },
    Scene {
        clean: false,
        name: "noclean",
        label: "FALSE",
    },
];

#[derive(Clone, Copy)]
struct Cell {
    w: f32,
    h: f32,
    suffix: &'static str,
}

impl Cell {
    const fn size(self) -> egui::Vec2 {
        egui::Vec2 {
            x: self.w,
            y: self.h,
        }
    }
}

#[derive(Clone, Copy)]
struct Scene {
    clean: bool,
    name: &'static str,
    label: &'static str,
}

fn c3_state(clean: bool) -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = false;
    s.step5.last_exit_code = Some(i32::from(!clean));
    s.step5.last_install_failed = false;
    s.step1.game_install = "EET".to_string();
    s
}

fn nameless_pasted_entry() -> ModlistEntry {
    let started = Utc::now() - ChronoDuration::seconds(4 * 60 + 12);
    let mut e = ModlistEntry::default();
    e.id = "DLBNR0000001".to_string();
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

#[test]
fn render_stage4_completion_banner_matrix() {
    assert_eq!(
        FALLBACK_NAME, "Shared modlist",
        "the nameless-code fallback must match the expected string"
    );

    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for scene in SCENES {
        for cell in CELLS {
            written.push(render_cell(scene, cell, &out_dir));
        }
    }

    assert_written(&written, 6, "expected 6 PNGs");
}

fn render_cell(scene: Scene, cell: Cell, out_dir: &Path) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(cell.size())
        .with_pixels_per_point(1.0)
        .build(move |ctx| render_frame(ctx, &mut frame, scene.clean));

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!(
        "stage4_completion_banner__{}__{}.png",
        scene.name, cell.suffix
    ));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}  stage-4 completion [{}: C3 {}]  -> {}",
        cell.suffix,
        scene.name,
        scene.label,
        abs.display()
    );
    path
}

fn render_frame(ctx: &egui::Context, frame: &mut u64, clean: bool) {
    if render_font_warmup(ctx, frame) {
        return;
    }

    let mut wizard_state = c3_state(clean);
    let entry = nameless_pasted_entry();
    let mut console_view = Step5ConsoleViewState::default();
    let palette = ThemePalette::Dark;
    let dev_mode = false;
    let exe_fingerprint = String::new();

    render_shell(ctx, |ui| {
        let sub = format!("{FALLBACK_NAME} \u{00B7} live install console");
        render_screen_title(ui, palette, "Installing modlist", Some(&sub));
        ui.add_space(10.0);
        render_completion_body(
            ui,
            palette,
            &mut wizard_state,
            &entry,
            &mut console_view,
            dev_mode,
            &exe_fingerprint,
        );
    });
}

fn render_completion_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    wizard_state: &mut WizardState,
    entry: &ModlistEntry,
    console_view: &mut Step5ConsoleViewState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            success_banner::render(ui, palette, wizard_state, entry);
            let _ = post_install_actions::render(ui, palette, wizard_state, entry);
            let _ = bio::ui::step5::page_step5::render(
                ui,
                wizard_state,
                console_view,
                None,
                None,
                Step5RenderCtx {
                    dev_mode,
                    exe_fingerprint,
                    palette: ThemePalette::Dark,
                },
            );
        });
}

fn render_font_warmup(ctx: &egui::Context, frame: &mut u64) -> bool {
    if *frame != 0 {
        *frame += 1;
        return false;
    }

    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
    *frame += 1;
    true
}

fn render_shell(body_ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
    render_top_panel(body_ctx, "scaffold_titlebar", REDESIGN_TITLEBAR_HEIGHT_PX);
    render_bottom_panel(body_ctx, "scaffold_statusbar", REDESIGN_STATUSBAR_HEIGHT_PX);

    let mut body = Some(body);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(body_ctx, |ui| {
            render_nav_rail(ui);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    if let Some(body) = body.take() {
                        body(ui);
                    }
                });
        });
}

fn render_top_panel(ctx: &egui::Context, id: &'static str, height: f32) {
    egui::TopBottomPanel::top(id)
        .exact_height(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn render_bottom_panel(ctx: &egui::Context, id: &'static str, height: f32) {
    egui::TopBottomPanel::bottom(id)
        .exact_height(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn render_nav_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("scaffold_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            fill_panel(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
        });
}

fn fill_panel(ui: &mut egui::Ui) {
    let r = ui.max_rect();
    ui.painter()
        .rect_filled(r, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn assert_written(written: &[PathBuf], expected: usize, message: &str) {
    for p in written {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(
            meta.len() > 0,
            "rendered PNG is empty (renderer produced no pixels): {}",
            p.display()
        );
    }
    assert_eq!(written.len(), expected, "{message}");
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
