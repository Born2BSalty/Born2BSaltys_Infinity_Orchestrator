// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bio::app::state::{Step3ItemState, Step3State};
use bio::registry::store::RegistryStore;
use bio::settings::redesign_store::RedesignSettingsStore;
use bio::ui::orchestrator::orchestrator_app::OrchestratorApp;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::shared::redesign_visuals;
use bio::ui::workspace::step3::workspace_step3;

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 820.0;

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const PALETTES: [(&str, ThemePalette); 2] =
    [("light", ThemePalette::Light), ("dark", ThemePalette::Dark)];

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("step3-polish")
}

fn temp_store_path(label: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bio_step3_polish_{}_{}_{}.json",
        std::process::id(),
        n,
        label
    ))
}

fn temp_orchestrator() -> OrchestratorApp {
    let mut app = OrchestratorApp::new(false);
    app.registry_store = RegistryStore::new_with_path(temp_store_path("registry"));
    app.redesign_settings_store = RedesignSettingsStore::new_with_path(temp_store_path("redesign"));
    app
}

fn make_parent(mod_name: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}.tp2"),
        component_id: "__PARENT__".to_string(),
        mod_name: mod_name.to_string(),
        component_label: String::new(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: format!("{mod_name}::block0"),
        is_parent: true,
        parent_placeholder: false,
    }
}

fn make_child(mod_name: &str, id: &str, label: &str, raw: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}.tp2"),
        component_id: id.to_string(),
        mod_name: mod_name.to_string(),
        component_label: label.to_string(),
        raw_line: raw.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: format!("{mod_name}::block0"),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn many_items() -> Vec<Step3ItemState> {
    let entries = [
        ("BG2FixPack", "setup-BG2FixPack.tp2", "v13.4"),
        ("SCS", "setup-stratagems.tp2", "v34.3"),
        ("EET", "setup-EET.tp2", "v13.1"),
        ("Wheels", "setup-wheels.tp2", "v1.0"),
        ("UB", "setup-UB.tp2", "v28"),
        ("IWDification", "setup-IWDification.tp2", "v7"),
        ("BGQE", "setup-BGQE.tp2", "v25"),
        ("Ascension", "setup-ascension.tp2", "v2.0.22"),
    ];

    let mut items: Vec<Step3ItemState> = Vec::new();
    for (i, (name, tp, ver)) in entries.iter().enumerate() {
        items.push(make_parent(name));
        for j in 0..3usize {
            items.push(make_child(
                name,
                &(i * 3 + j).to_string(),
                &format!("{name} Component {j}"),
                &format!("~{name}\\{tp}~ #0 #{j} // {name} Component {j}: {ver}"),
            ));
        }
    }
    items
}

fn setup_step3(app: &mut OrchestratorApp) {
    app.wizard_state.step1.game_install = "EET".to_string();
    let items = many_items();
    app.wizard_state.step3 = Step3State {
        active_game_tab: "BGEE".to_string(),
        bgee_items: items.clone(),
        bg2ee_items: items,
        ..Step3State::default()
    };
}

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("s3polish_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
            ui.allocate_space(ui.available_size());
        });
}

fn render_statusbar(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("s3polish_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
            ui.allocate_space(ui.available_size());
        });
}

fn render_shell(ctx: &egui::Context, palette: ThemePalette, page: impl FnOnce(&mut egui::Ui)) {
    ctx.set_visuals(redesign_visuals::build_for(palette));
    render_titlebar(ctx);
    render_statusbar(ctx);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            egui::SidePanel::left("s3polish_rail")
                .exact_width(REDESIGN_NAV_WIDTH_PX)
                .resizable(false)
                .show_separator_line(false)
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    let rect = ui.max_rect();
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
                    ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
                    ui.allocate_space(ui.available_size());
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, page);
        });
}

fn snap_step3(name: &str, palette_tag: &str, palette: ThemePalette) -> PathBuf {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");

    let mut app = temp_orchestrator();
    app.theme_palette = palette;
    setup_step3(&mut app);

    let app_cell = RefCell::new(app);
    let mut frame = 0u64;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(WIDTH, HEIGHT))
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
            let mut app = app_cell.borrow_mut();
            render_shell(ctx, palette, |ui| {
                workspace_step3::render(ui, &mut app);
            });
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest render must produce an image");
    let path = dir.join(format!("{name}_{palette_tag}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    println!("SNAPSHOT  {}", path.display());
    path
}

#[test]
fn render_step3_polish() {
    for (tag, palette) in PALETTES {
        let path = snap_step3("step3-polish", tag, palette);
        let meta = std::fs::metadata(&path)
            .unwrap_or_else(|_| panic!("PNG must exist: {}", path.display()));
        assert!(
            meta.len() > 0,
            "render-gate PNG is empty: {}",
            path.display()
        );
    }
}
