// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::app::state::WizardState;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX,
};
use bio::ui::step5::content_step5::Step5RenderCtx;
use bio::ui::step5::page_step5;
use bio::ui::step5::state_step5::Step5ConsoleViewState;

use eframe::egui;
use egui_kittest::Harness;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const LONG_TERMINAL_ERROR: &str = "Initializing terminal... \
    a very very long error message that without an orchestrator-side clipped_pane \
    would paint past the right edge of the central column and bleed into the \
    shell chrome, the rail, and the statusbar. With the clip applied the over-wide \
    text is constrained to the central column's allocated rect.";

struct Cell {
    w: u16,
    h: u16,
}

#[test]
fn render_workspace_step5_console_clip_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();
    for cell in &[
        Cell { w: 1280, h: 820 },
        Cell { w: 1045, h: 735 },
        Cell { w: 960, h: 680 },
    ] {
        written.push(render_cell(&out_dir, cell));
    }
    assert_eq!(
        written.len(),
        3,
        "expected 3 matrix-cell PNGs (widths 1280/1045/960)"
    );
    for p in &written {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(meta.len() > 0, "rendered PNG empty: {}", p.display());
    }
}

fn render_cell(out_dir: &Path, cell: &Cell) -> PathBuf {
    let mut frame: u64 = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(f32::from(cell.w), f32::from(cell.h)))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if install_fonts_frame(ctx, &mut frame) {
                return;
            }
            render_scaffold(ctx);
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!(
        "workspace_step5_console_clip__{}x{}.png",
        cell.w, cell.h
    ));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}x{}  workspace_step5_console_clip  -> {}",
        cell.w,
        cell.h,
        abs.display()
    );
    path
}

fn install_fonts_frame(ctx: &egui::Context, frame: &mut u64) -> bool {
    if *frame == 0 {
        install_redesign_fonts(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_space(ui.available_size());
        });
        *frame += 1;
        return true;
    }
    *frame += 1;
    false
}

fn render_scaffold(ctx: &egui::Context) {
    egui::TopBottomPanel::top("scaffold_titlebar_console_clip")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);

    egui::TopBottomPanel::bottom("scaffold_statusbar_console_clip")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            egui::SidePanel::left("scaffold_rail_console_clip")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    render_flat_band(ui);
                });

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    let mut ws = WizardState::default();
                    let mut console_view = Step5ConsoleViewState::default();
                    let panel_rect = ui.available_rect_before_wrap();
                    clipped_pane(ui, panel_rect, |ui| {
                        let _ = page_step5::render(
                            ui,
                            &mut ws,
                            &mut console_view,
                            None,
                            Some(LONG_TERMINAL_ERROR),
                            Step5RenderCtx {
                                dev_mode: false,
                                exe_fingerprint: "",
                                palette: bio::ui::shared::redesign_tokens::ThemePalette::Dark,
                            },
                        );
                    });
                });
        });
}

fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn render_flat_band(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp.parent().map(Path::to_path_buf).unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
