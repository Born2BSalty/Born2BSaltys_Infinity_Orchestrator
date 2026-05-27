// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::app::state::WizardState;
use bio::app::terminal::EmbeddedTerminal;
use bio::registry::model::ModlistEntry;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
    redesign_page_bg,
};
use bio::ui::shared::redesign_visuals;
use bio::ui::step5::content_step5::Step5RenderCtx;
use bio::ui::step5::page_step5;
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::{post_install_actions, success_banner};

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 820.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];

fn evidence_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("infinity_orchestrator/phase8-evidence")
}

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_scaffold(ctx: &egui::Context, palette: ThemePalette) {
    ctx.set_visuals(redesign_visuals::build_for(palette));
    let title_h = REDESIGN_TITLEBAR_HEIGHT_PX;
    let status_h = REDESIGN_STATUSBAR_HEIGHT_PX;

    egui::TopBottomPanel::top("title_bar")
        .exact_height(title_h)
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |_ui| {});

    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(status_h)
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |_ui| {});

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            egui::SidePanel::left("scaffold_rail")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
                .show_inside(ui, |_ui| {});

            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| render_step5_scene(ui, palette));
                });
        });
}

fn render_step5_scene(ui: &mut egui::Ui, palette: ThemePalette) {
    let mut wizard_state = WizardState::default();
    wizard_state.step5.install_running = false;
    wizard_state.step5.last_status_text = "Preflight check".to_string();

    let mut console_view = Step5ConsoleViewState::default();
    let exe_fingerprint = String::new();

    let mut terminal = EmbeddedTerminal::new().expect("embedded terminal");
    terminal.append_marker("INFO Starting install sequence");
    terminal.append_marker("WARN Backup directory not found, continuing");
    terminal.append_marker("DEBUG Loading component list from setup-bg1npc.tp2");
    terminal.append_marker("[SENT] #0 y");
    terminal.append_marker("ERROR Failed to read component header");
    terminal.append_marker("Component #1234 BG1 NPC Project core files");
    terminal.append_marker("~Setup-bg1npc.tp2~ SUCCESSFULLY INSTALLED Component");
    terminal.append_marker("Sand-colored path reference for mod folder");
    terminal.append_marker("MOD_INSTALLER::processing next step");
    terminal.append_marker("Default plain output line from WeiDU");

    let entry = ModlistEntry::default();
    success_banner::render(ui, palette, &wizard_state, &entry);
    let _ = post_install_actions::render(ui, palette, &wizard_state, &entry);
    let _ = page_step5::render(
        ui,
        &mut wizard_state,
        &mut console_view,
        Some(&mut terminal),
        None,
        Step5RenderCtx {
            dev_mode: false,
            exe_fingerprint: &exe_fingerprint,
            palette,
        },
    );
}

fn snap(out_dir: &Path, name: &str, render: impl Fn(&egui::Context) + 'static) -> PathBuf {
    let mut frame = 0u32;
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

    harness.run_steps(8);

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
fn render_item3_step5_evidence() {
    let dir = evidence_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");

    let mut written: Vec<PathBuf> = Vec::new();

    for (label, palette) in PALETTES {
        written.push(snap(&dir, &format!("item3-step5-{label}"), move |ctx| {
            render_scaffold(ctx, palette);
        }));
    }

    assert_eq!(written.len(), 2, "expected 2 PNGs (1 scene x 2 palettes)");
    for path in &written {
        let meta =
            std::fs::metadata(path).unwrap_or_else(|_| panic!("PNG not found: {}", path.display()));
        assert!(meta.len() > 0, "PNG empty: {}", path.display());
    }
}
