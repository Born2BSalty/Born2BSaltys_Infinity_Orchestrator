// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    ThemePalette, redesign_conditional, redesign_conditional_fill, redesign_conflict,
    redesign_conflict_fill, redesign_error, redesign_error_emphasis, redesign_game_mismatch,
    redesign_game_mismatch_fill, redesign_included, redesign_included_fill, redesign_info,
    redesign_info_fill, redesign_page_bg, redesign_pill_warn, redesign_prompt_fill,
    redesign_prompt_stroke, redesign_prompt_text, redesign_status_idle, redesign_status_preparing,
    redesign_status_running, redesign_success, redesign_success_bright, redesign_text_disabled,
    redesign_text_muted, redesign_warning, redesign_warning_soft,
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
        .join("render_gate/p8_t1_t4_compat_tree_details")
}

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_compat_color_swatches(ctx: &egui::Context, palette: ThemePalette) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.label("Compat fill swatches");
            let swatches = [
                (
                    "conflict",
                    redesign_conflict(palette),
                    redesign_conflict_fill(palette),
                ),
                (
                    "included",
                    redesign_included(palette),
                    redesign_included_fill(palette),
                ),
                ("info", redesign_info(palette), redesign_info_fill(palette)),
                (
                    "game_mismatch",
                    redesign_game_mismatch(palette),
                    redesign_game_mismatch_fill(palette),
                ),
                (
                    "conditional",
                    redesign_conditional(palette),
                    redesign_conditional_fill(palette),
                ),
            ];
            for (label, stroke_color, fill_color) in swatches {
                ui.horizontal(|ui| {
                    ui.colored_label(stroke_color, label);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(24.0, 14.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, fill_color);
                });
            }
        });
}

fn render_header_marker_swatches(ctx: &egui::Context, palette: ThemePalette) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.label("Tree header marker swatches");
            ui.colored_label(redesign_text_muted(palette), "update_locked (lock icon)");
            ui.colored_label(redesign_success(palette), "+ marker");
            ui.colored_label(redesign_error(palette), "! marker");
            ui.colored_label(
                redesign_error_emphasis(palette),
                "error emphasis (popup filter)",
            );
            ui.colored_label(
                redesign_warning_soft(palette),
                "warning soft (popup filter)",
            );
            ui.colored_label(redesign_text_disabled(palette), "disabled component label");
        });
}

fn render_details_status_swatches(ctx: &egui::Context, palette: ThemePalette) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.label("Details pane status swatches");
            ui.colored_label(redesign_success_bright(palette), "exact log ready");
            ui.colored_label(redesign_error(palette), "exact log not ready");
            ui.colored_label(
                redesign_pill_warn(palette),
                "exact log warning / path missing",
            );
            ui.colored_label(redesign_warning(palette), "reason / state warning");
            ui.colored_label(redesign_text_muted(palette), "unchecked state muted");
            ui.colored_label(redesign_status_running(palette), "status running");
            ui.colored_label(redesign_status_preparing(palette), "status preparing");
            ui.colored_label(redesign_status_idle(palette), "status idle");
        });
}

fn render_prompt_pill_swatches(ctx: &egui::Context, palette: ThemePalette) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.label("Prompt pill swatches");
            let fill = redesign_prompt_fill(palette);
            let stroke_color = redesign_prompt_stroke(palette);
            let text_color = redesign_prompt_text(palette);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 22.0), egui::Sense::hover());
            ui.painter().rect(
                rect,
                4.0,
                fill,
                egui::Stroke::new(1.0, stroke_color),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "PROMPT",
                egui::FontId::proportional(10.0),
                text_color,
            );
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
fn render_p8_t1_t4_compat_tree_details() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create render_gate dir");

    let mut written: Vec<PathBuf> = Vec::new();

    for (label, palette) in PALETTES {
        written.push(snap(
            &dir,
            &format!("compat_color_swatches__{label}"),
            move |ctx| render_compat_color_swatches(ctx, palette),
        ));
        written.push(snap(
            &dir,
            &format!("header_marker_swatches__{label}"),
            move |ctx| render_header_marker_swatches(ctx, palette),
        ));
        written.push(snap(
            &dir,
            &format!("details_status_swatches__{label}"),
            move |ctx| render_details_status_swatches(ctx, palette),
        ));
        written.push(snap(
            &dir,
            &format!("prompt_pill_swatches__{label}"),
            move |ctx| render_prompt_pill_swatches(ctx, palette),
        ));
    }

    assert_eq!(written.len(), 8, "expected 8 PNGs (4 scenes x 2 palettes)");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG empty: {}", path.display());
    }
}
