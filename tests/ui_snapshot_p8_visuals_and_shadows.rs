// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::orchestrator::widgets::{
    BtnOpts, redesign_btn, redesign_section_header, redesign_window_title,
};
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_primary,
};
use bio::ui::shared::redesign_visuals;

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 720.0;
const HEIGHT: f32 = 480.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("light", ThemePalette::Light), ("dark", ThemePalette::Dark)];

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .map_or_else(|| PathBuf::from("target/tmp"), Path::to_path_buf)
        .join("render_gate/p8_visuals_and_shadows")
}

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_popup_demo(ctx: &egui::Context, palette: ThemePalette) {
    ctx.set_visuals(redesign_visuals::build_for(palette));

    egui::CentralPanel::default().show(ctx, |_ui| {});

    egui::Window::new(redesign_window_title(
        palette,
        "Popup Demo \u{2014} no shadow",
    ))
    .collapsible(true)
    .resizable(false)
    .default_pos(egui::pos2(40.0, 40.0))
    .show(ctx, |ui| {
        ui.label(
            egui::RichText::new("Header strong (explicit color)")
                .strong()
                .color(redesign_text_primary(palette)),
        );
        ui.label(egui::RichText::new("Header strong (no explicit color)").strong());
        ui.label("This is a plain label.");
        ui.add_space(8.0);

        redesign_section_header(ui, palette, "Sample Section", Some(3));
        ui.add_space(8.0);
        egui::Frame::group(ui.style())
            .fill(redesign_shell_bg(palette))
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            ))
            .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.label("Item one");
                ui.label("Item two");
                ui.label("Item three");
            });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let _ = redesign_btn(
                ui,
                palette,
                "Primary Button",
                BtnOpts {
                    primary: true,
                    small: true,
                    ..Default::default()
                },
            );
            let _ = redesign_btn(
                ui,
                palette,
                "Secondary",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            );
            let _ = redesign_btn(
                ui,
                palette,
                "Close",
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            );
        });
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
fn render_p8_visuals_and_shadows() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create render_gate dir");

    let mut written: Vec<PathBuf> = Vec::new();

    for (label, palette) in PALETTES {
        written.push(snap(&dir, &format!("popup_demo__{label}"), move |ctx| {
            render_popup_demo(ctx, palette);
        }));
    }

    assert_eq!(written.len(), 2, "expected 2 PNGs (1 scene x 2 palettes)");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG empty: {}", path.display());
    }
}
