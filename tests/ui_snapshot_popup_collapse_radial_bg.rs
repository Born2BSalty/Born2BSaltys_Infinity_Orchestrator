// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::shared::redesign_dot_background::paint_dot_background;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_page_bg,
    redesign_shell_bg,
};

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 820.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("light", ThemePalette::Light), ("dark", ThemePalette::Dark)];

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .map_or_else(|| PathBuf::from("target/tmp"), Path::to_path_buf)
        .join("render_gate/popup_collapse_and_radial_bg")
}

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_shell_with_dot_bg(ctx: &egui::Context, palette: ThemePalette) {
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    paint_dot_background(&bg_painter, ctx.screen_rect(), palette);

    egui::TopBottomPanel::top("snap_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(redesign_shell_bg(palette)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("Infinity Orchestrator");
            });
        });

    egui::TopBottomPanel::bottom("snap_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .show_separator_line(false)
        .frame(egui::Frame::NONE.fill(redesign_shell_bg(palette)))
        .show(ctx, |ui| {
            ui.label("weidu v249 · all paths ok");
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.label("Shell with dotted radial background");
        });
}

fn render_popup_collapsed(ctx: &egui::Context, palette: ThemePalette) {
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    paint_dot_background(&bg_painter, ctx.screen_rect(), palette);

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |_ui| {});

    egui::Window::new("Compat Popup — collapsed")
        .collapsible(true)
        .resizable(false)
        .default_pos(egui::pos2(80.0, 80.0))
        .show(ctx, |ui| {
            ui.label("Issue detail body — hidden when collapsed");
        });
}

fn render_popup_expanded(ctx: &egui::Context, palette: ThemePalette) {
    let bg_painter = ctx.layer_painter(egui::LayerId::background());
    paint_dot_background(&bg_painter, ctx.screen_rect(), palette);

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |_ui| {});

    egui::Window::new("Compat Popup — expanded")
        .collapsible(true)
        .resizable(false)
        .default_pos(egui::pos2(80.0, 80.0))
        .show(ctx, |ui| {
            ui.label("Issue detail body — visible when expanded");
            ui.label("Second line of issue details content");
        });
}

fn snap(out_dir: &Path, name: &str, render: impl Fn(&egui::Context) + 'static) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(WIDTH, HEIGHT))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if frame == 0 {
                render_font_frame(ctx);
            } else {
                render(ctx);
            }
            frame += 1;
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = out_dir.join(format!("{name}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!("SNAPSHOT  {}", abs.display());
    path
}

#[test]
fn render_popup_collapse_and_radial_bg() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create render_gate dir");

    let mut written: Vec<PathBuf> = Vec::new();

    for (label, palette) in PALETTES {
        written.push(snap(&dir, &format!("shell_dot_bg__{label}"), move |ctx| {
            render_shell_with_dot_bg(ctx, palette)
        }));
        written.push(snap(
            &dir,
            &format!("popup_expanded__{label}"),
            move |ctx| render_popup_expanded(ctx, palette),
        ));
        written.push(snap(
            &dir,
            &format!("popup_collapsed__{label}"),
            move |ctx| render_popup_collapsed(ctx, palette),
        ));
    }

    assert_eq!(written.len(), 6, "expected 6 PNGs (3 scenes x 2 palettes)");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG empty: {}", path.display());
    }
}
