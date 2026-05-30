// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::orchestrator::widgets::NotificationManager;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{ThemePalette, redesign_text_primary};
use bio::ui::shared::redesign_visuals;

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 520.0;
const HEIGHT: f32 = 360.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .map_or_else(|| PathBuf::from("target/tmp"), Path::to_path_buf)
        .join("render_gate/p8_t10_notifications")
}

/// Drives the production toast stack (`NotificationManager::show`) so the PNG
/// reflects the real `render_custom_toast` path rather than a re-implementation.
///
/// The toasts are enqueued exactly one frame before capture: a brand-new egui
/// `Area` has no size on its first frame and anchors off-screen, so it needs a
/// second frame to place — but `egui_kittest` advances simulated time each frame,
/// so enqueuing any earlier would let the timed toasts auto-dismiss first.
fn snap(out_dir: &Path, name: &str, palette: ThemePalette) -> PathBuf {
    const ENQUEUE_FRAME: u32 = 1;
    const CAPTURE_FRAME: u32 = 2;
    let mut mgr = NotificationManager::new();
    let mut frame: u32 = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(WIDTH, HEIGHT))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if frame == 0 {
                install_redesign_fonts(ctx);
            }
            ctx.set_visuals(redesign_visuals::build_for(palette));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Toast render gate — 4 severities")
                        .size(13.0)
                        .color(redesign_text_primary(palette)),
                );
            });
            if frame == ENQUEUE_FRAME {
                mgr.success(
                    "Success — installation complete: all selected mods were applied without errors",
                );
                mgr.info("Info — scan completed");
                mgr.warn("Warning — mods need attention");
                mgr.error("Error — failed to open folder");
            }
            if frame >= ENQUEUE_FRAME {
                mgr.show(ctx, palette);
            }
            frame += 1;
        });
    // show_progress(true) on timed toasts triggers a repaint request every frame,
    // which exhausts Harness::run's max_steps guard. Drive each frame exactly
    // once with step() instead.
    for _ in 0..=CAPTURE_FRAME {
        harness.step();
    }
    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = out_dir.join(format!("{name}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    println!("SNAPSHOT  {}", path.display());
    path
}

/// Drives the production history popup (`NotificationManager::render_history_popup`)
/// so the PNG reflects the real anchored, fixed-width box rather than a stand-in.
fn snap_history(out_dir: &Path, name: &str, palette: ThemePalette) -> PathBuf {
    const OPEN_FRAME: u32 = 1;
    const CAPTURE_FRAME: u32 = 2;
    let mut mgr = NotificationManager::new();
    let mut frame: u32 = 0;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(WIDTH, HEIGHT))
        .with_pixels_per_point(1.0)
        .build(move |ctx| {
            if frame == 0 {
                install_redesign_fonts(ctx);
            }
            ctx.set_visuals(redesign_visuals::build_for(palette));
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("History popup render gate")
                        .size(13.0)
                        .color(redesign_text_primary(palette)),
                );
            });
            if frame == OPEN_FRAME {
                mgr.success("Copied import code for \"Shared modlist\"");
                mgr.error("No import code yet for \"Polished BG2EE\"");
                mgr.info("Scan completed: 142 components across 6 mods, no conflicts");
                mgr.history_open = true;
            }
            if frame >= OPEN_FRAME {
                mgr.render_history_popup(ctx, palette, true);
            }
            frame += 1;
        });
    for _ in 0..=CAPTURE_FRAME {
        harness.step();
    }
    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = out_dir.join(format!("{name}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));
    println!("SNAPSHOT  {}", path.display());
    path
}

#[test]
fn render_p8_t10_notifications() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create render_gate dir");
    let mut written: Vec<PathBuf> = Vec::new();
    for (label, palette) in PALETTES {
        written.push(snap(&dir, &format!("four_severities__{label}"), palette));
        written.push(snap_history(
            &dir,
            &format!("history_popup__{label}"),
            palette,
        ));
    }
    assert_eq!(written.len(), 4, "expected 4 PNGs");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG empty: {}", path.display());
    }
}

#[test]
fn notification_manager_basic_api() {
    let mut mgr = NotificationManager::new();
    assert!(!mgr.has_history());
    mgr.success("good thing");
    mgr.error("bad thing");
    assert!(mgr.has_history());
    assert_eq!(mgr.history().len(), 2);
}
