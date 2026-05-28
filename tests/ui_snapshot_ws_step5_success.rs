// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
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

const CELLS: [Cell; 2] = [
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

fn c3_clean_state() -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = false;
    s.step5.last_exit_code = Some(0);
    s.step5.last_install_failed = false;
    s.step1.game_install = "EET".to_string();
    s
}

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

#[test]
fn render_workspace_step5_post_install_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for cell in CELLS {
        written.push(render_cell(cell, &out_dir));
    }

    assert_written(&written, 2, "expected 2 matrix-cell PNGs");
}

fn render_cell(cell: Cell, out_dir: &Path) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(cell.size())
        .with_pixels_per_point(1.0)
        .build(move |ctx| render_frame(ctx, &mut frame));

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!("workspace_step5_success__{}.png", cell.suffix));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}  post-install (C3 TRUE)  -> {}",
        cell.suffix,
        abs.display()
    );
    path
}

fn render_frame(ctx: &egui::Context, frame: &mut u64) {
    if render_font_warmup(ctx, frame) {
        return;
    }

    let mut wizard_state = c3_clean_state();
    let entry = installed_entry();
    let mut console_view = Step5ConsoleViewState::default();
    let palette = ThemePalette::Dark;
    let dev_mode = false;
    let exe_fingerprint = String::new();

    render_shell(ctx, |ui| {
        render_share_header(ui, palette);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                success_banner::render(ui, palette, &wizard_state, &entry);
                let _ = post_install_actions::render(ui, palette, &wizard_state, &entry);
                let _ = bio::ui::step5::page_step5::render(
                    ui,
                    &mut wizard_state,
                    &mut console_view,
                    None,
                    None,
                    Step5RenderCtx {
                        dev_mode,
                        exe_fingerprint: &exe_fingerprint,
                        palette: ThemePalette::Dark,
                    },
                );
            });
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

fn render_share_header(ui: &mut egui::Ui, palette: ThemePalette) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 28.0),
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
