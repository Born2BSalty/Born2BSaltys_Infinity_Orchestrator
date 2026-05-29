// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bio::app::state::{Step2ComponentState, Step2ModState};
use bio::registry::store::RegistryStore;
use bio::settings::redesign_store::RedesignSettingsStore;
use bio::ui::orchestrator::orchestrator_app::OrchestratorApp;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::shared::redesign_visuals;
use bio::ui::workspace::step2::workspace_step2;

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
        .join("step2-polish")
}

fn temp_store_path(label: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bio_step2_polish_{}_{}_{}.json",
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

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("polish_titlebar")
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
    egui::TopBottomPanel::bottom("polish_statusbar")
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
            egui::SidePanel::left("polish_rail")
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

fn component(
    id: &str,
    label: &str,
    raw: &str,
    checked: bool,
    order: Option<usize>,
) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: label.to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        raw_line: raw.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        is_meta_mode_component: false,
        disabled: false,
        compat_kind: None,
        compat_source: None,
        compat_related_mod: None,
        compat_related_component: None,
        compat_graph: None,
        compat_evidence: None,
        disabled_reason: None,
        checked,
        selected_order: order,
    }
}

fn mod_state(
    name: &str,
    tp_file: &str,
    checked: bool,
    components: Vec<Step2ComponentState>,
) -> Step2ModState {
    Step2ModState {
        name: name.to_string(),
        tp_file: tp_file.to_string(),
        tp2_path: format!("{name}\\{tp_file}"),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked,
        hidden_components: Vec::new(),
        components,
    }
}

fn many_mods() -> Vec<Step2ModState> {
    let names = [
        ("BG2FixPack", "setup-BG2FixPack.tp2"),
        ("SCS", "setup-stratagems.tp2"),
        ("EET", "setup-EET.tp2"),
        ("Wheels", "setup-wheels.tp2"),
        ("UB", "setup-UB.tp2"),
        ("IWDification", "setup-IWDification.tp2"),
        ("BGQE", "setup-BGQE.tp2"),
        ("Ascension", "setup-ascension.tp2"),
    ];
    // Repeat the base set so the collapsed rows overflow the box and the scrollbar renders.
    (0..30usize)
        .map(|i| {
            let (base, tp) = names[i % names.len()];
            let name = format!("{base} {}", i + 1);
            mod_state(
                &name,
                tp,
                i % 2 == 0,
                vec![
                    component(
                        "0",
                        &format!("{name} Core"),
                        &format!("~{name}/{tp}~ #0 #0 // {name} Core: v1.0"),
                        true,
                        Some(i * 2),
                    ),
                    component(
                        "1",
                        &format!("{name} Extra"),
                        &format!("~{name}/{tp}~ #0 #1 // {name} Extra: v1.0"),
                        false,
                        None,
                    ),
                ],
            )
        })
        .collect()
}

fn setup_eet_step2(app: &mut OrchestratorApp) {
    app.wizard_state.step1.game_install = "EET".to_string();
    app.wizard_state.step2.active_game_tab = "BGEE".to_string();
    app.wizard_state.step2.bgee_mods = many_mods();
    app.wizard_state.step2.bg2ee_mods = many_mods();
    app.wizard_state.step2.selected_count = 30;
    app.wizard_state.step2.total_count = 60;
    app.workspace_view.step2.details_open = false;
}

fn snap_step2(name: &str, palette_tag: &str, palette: ThemePalette) -> PathBuf {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");

    let mut app = temp_orchestrator();
    app.theme_palette = palette;
    setup_eet_step2(&mut app);

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
                let _ = workspace_step2::render(ui, &mut app);
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
fn render_step2_polish() {
    for (tag, palette) in PALETTES {
        let path = snap_step2("step2-polish", tag, palette);
        let meta = std::fs::metadata(&path)
            .unwrap_or_else(|_| panic!("PNG must exist: {}", path.display()));
        assert!(
            meta.len() > 0,
            "render-gate PNG is empty: {}",
            path.display()
        );
    }
}
