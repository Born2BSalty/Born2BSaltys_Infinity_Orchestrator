// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Render-gate snapshot: lock-toggle icon for the Updates popup Source Choices grid.
//!
//! Renders a minimal grid with one locked mod row and one unlocked mod row,
//! mirroring the icon rendering added to `update_check_popup_lists_step2.rs`
//! so the orchestrator can verify the glyph, colors, and hover text visually.

use std::path::PathBuf;

use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::ThemePalette;
use bio::ui::shared::{theme_global, typography_global};

use eframe::egui;
use egui_kittest::Harness;

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("update-lock-icon")
}

fn render_lock_grid(ui: &mut egui::Ui, _palette: ThemePalette) {
    egui::Grid::new("snapshot-lock-grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("CDTweaks (locked)");
            let locked_icon = typography_global::strong("🔒").color(theme_global::warning());
            ui.small_button(locked_icon).on_hover_text("Unlock updates");
            ui.end_row();

            ui.label("SCS (unlocked)");
            let unlocked_icon =
                typography_global::strong("🔓").color(theme_global::text_disabled());
            ui.small_button(unlocked_icon).on_hover_text("Lock updates");
            ui.end_row();
        });
}

fn snap_lock_icon(palette_tag: &str, palette: ThemePalette) -> PathBuf {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create target/update-lock-icon dir");

    let mut frame = 0u64;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(480.0, 120.0))
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
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
                .show(ctx, |ui| {
                    render_lock_grid(ui, palette);
                });
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = dir.join(format!("update_lock_icon__{palette_tag}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT update-lock-icon {palette_tag} -> {}",
        abs.display()
    );
    path
}

#[test]
fn render_update_lock_icon() {
    let palettes: &[(&str, ThemePalette)] =
        &[("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];
    for (tag, palette) in palettes {
        let path = snap_lock_icon(tag, *palette);
        let meta = std::fs::metadata(&path)
            .unwrap_or_else(|_| panic!("PNG must exist: {}", path.display()));
        assert!(
            meta.len() > 0,
            "render-gate PNG is empty: {}",
            path.display()
        );
    }
}
