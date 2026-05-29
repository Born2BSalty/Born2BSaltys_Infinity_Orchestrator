// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

//! Render-gate for the active-tab open-seam fix.
//!
//! Produces full-surface + zoomed-seam PNGs of every tab surface in both
//! Light and Dark themes, driving the production `pub` render entry points
//! through a temp-dir-repointed `OrchestratorApp`. The orchestrator inspects
//! these to confirm whether the content panel's top stroke bisects the
//! selected tab's bottom edge on each surface.
//!
//! DATA-LOSS guard: every `OrchestratorApp` built here repoints its
//! `registry_store` and `redesign_settings_store` to `std::env::temp_dir()`
//! immediately after construction. The non-repointable `settings_store`
//! (no `new_with_path`) never writes because Step 1 is left pristine, so the
//! bio-settings snapshot equals the last-saved baseline and the drop-time
//! flush is a no-op. No store binds `%APPDATA%\bio` for writes.

use std::cell::RefCell;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bio::app::state::{Step2ComponentState, Step2ModState, Step3ItemState};
use bio::install_runtime::reinstall_route;
use bio::registry::model::{Game, ModlistEntry};
use bio::registry::store::RegistryStore;
use bio::settings::redesign_store::RedesignSettingsStore;
use bio::ui::install::page_install;
use bio::ui::install::state_install::InstallStage;
use bio::ui::orchestrator::nav_destination::NavDestination;
use bio::ui::orchestrator::orchestrator_app::OrchestratorApp;
use bio::ui::settings::page_settings;
use bio::ui::settings::state_settings::SettingsTab;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX, REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette,
};
use bio::ui::shared::redesign_visuals;
use bio::ui::workspace::step2::workspace_step2;
use bio::ui::workspace::step3::workspace_step3;
use bio::ui::workspace::step4::workspace_step4;

use eframe::egui;
use egui_kittest::Harness;
use flate2::{Compression, write::ZlibEncoder};

const WIDTH: f32 = 1280.0;
const HEIGHT: f32 = 820.0;

/// Mirrors the production shell `CentralPanel` inner margin.
const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const PALETTES: [(&str, ThemePalette); 2] =
    [("light", ThemePalette::Light), ("dark", ThemePalette::Dark)];

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn out_dir_before() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tab-seam-before")
}

fn out_dir_after() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tab-seam-samelayer")
}

fn out_dir_corners() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("tab-seam-corners")
}

fn temp_store_path(label: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bio_tab_seam_{}_{}_{}.json",
        std::process::id(),
        n,
        label
    ))
}

/// Builds an `OrchestratorApp` with its writable stores repointed to temp.
fn temp_orchestrator() -> OrchestratorApp {
    let mut app = OrchestratorApp::new(false);
    app.registry_store = RegistryStore::new_with_path(temp_store_path("registry"));
    app.redesign_settings_store = RedesignSettingsStore::new_with_path(temp_store_path("redesign"));
    app
}

// ---------------------------------------------------------------------------
// Production-faithful shell scaffold (titlebar + left rail + statusbar +
// central inner margin), matching the real `render_shell` layout so the page
// content lands at the same available rect.
// ---------------------------------------------------------------------------

fn render_titlebar(ctx: &egui::Context) {
    egui::TopBottomPanel::top("seam_titlebar")
        .exact_height(REDESIGN_TITLEBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_statusbar(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("seam_statusbar")
        .exact_height(REDESIGN_STATUSBAR_HEIGHT_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, render_flat_band);
}

fn render_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("seam_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            render_flat_band(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
        });
}

fn render_flat_band(ui: &mut egui::Ui) {
    let rect = ui.max_rect();
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

/// Runs `page` inside the shell scaffold for the given palette.
fn render_shell(ctx: &egui::Context, palette: ThemePalette, page: impl FnOnce(&mut egui::Ui)) {
    ctx.set_visuals(redesign_visuals::build_for(palette));
    render_titlebar(ctx);
    render_statusbar(ctx);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(ctx, |ui| {
            render_rail(ui);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, page);
        });
}

// ---------------------------------------------------------------------------
// Harness driver: builds the app once, repoints stores, settles 8 frames
// (frame 0 installs fonts), renders, then writes a full PNG + zoomed crop.
//
// `seam_crop` is `(x, y, w, h)` in integer pixels for the zoomed seam shot.
// ---------------------------------------------------------------------------

struct SeamShot {
    full: PathBuf,
    zoom: PathBuf,
}

/// Renders a surface and writes `<name>_<theme>.png` + `<name>_<theme>_seam.png`.
///
/// `setup` mutates the freshly built app for this surface; `page` invokes the
/// production render entry inside the shell. `seam_crop` is the pixel region
/// `(x, y, w, h)` to crop for the zoomed seam shot.
fn snap_surface<S, P>(
    dir: &PathBuf,
    name: &str,
    palette_tag: &str,
    palette: ThemePalette,
    setup: S,
    page: P,
    seam_crop: (u32, u32, u32, u32),
) -> SeamShot
where
    S: FnOnce(&mut OrchestratorApp),
    P: Fn(&mut OrchestratorApp, &egui::Context, &mut egui::Ui) + 'static,
{
    std::fs::create_dir_all(dir).expect("create tab-seam dir");

    let mut app = temp_orchestrator();
    app.theme_palette = palette;
    setup(&mut app);

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
                let ctx = ui.ctx().clone();
                page(&mut app, &ctx, ui);
            });
        });

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");

    let full = dir.join(format!("{name}_{palette_tag}.png"));
    img.save(&full)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", full.display()));

    let zoom = dir.join(format!("{name}_{palette_tag}_seam.png"));
    let cw = img.width();
    let ch = img.height();
    let (cx, cy, crop_w, crop_h) = seam_crop;
    let x = cx.min(cw.saturating_sub(1));
    let y = cy.min(ch.saturating_sub(1));
    let w = crop_w.min(cw.saturating_sub(x)).max(1);
    let h = crop_h.min(ch.saturating_sub(y)).max(1);
    let cropped = image::imageops::crop_imm(&img, x, y, w, h).to_image();
    cropped
        .save(&zoom)
        .unwrap_or_else(|e| panic!("write zoom PNG {}: {e}", zoom.display()));

    println!("SEAM-SHOT  {} -> {}", name, full.display());
    println!("SEAM-ZOOM  {} -> {}", name, zoom.display());

    SeamShot { full, zoom }
}

fn assert_shot(shot: &SeamShot) {
    for p in [&shot.full, &shot.zoom] {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(meta.len() > 0, "rendered PNG is empty: {}", p.display());
    }
}

// ---------------------------------------------------------------------------
// State builders
// ---------------------------------------------------------------------------

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

fn demo_step2_mods() -> Vec<Step2ModState> {
    vec![
        mod_state(
            "BG2FixPack",
            "setup-BG2FixPack.tp2",
            true,
            vec![
                component(
                    "0",
                    "Core Fixes",
                    "~BG2FIXPACK/SETUP-BG2FIXPACK.TP2~ #0 #0 // BG2 Fixpack - Core Fixes: v13.4",
                    true,
                    Some(0),
                ),
                component(
                    "2",
                    "Creature Corrections",
                    "~BG2FIXPACK/SETUP-BG2FIXPACK.TP2~ #0 #2 // Creature Corrections: v13.4",
                    false,
                    None,
                ),
            ],
        ),
        mod_state(
            "SCS",
            "setup-stratagems.tp2",
            true,
            vec![component(
                "4000",
                "Smarter Mages",
                "~STRATAGEMS/SETUP-STRATAGEMS.TP2~ #0 #4000 // Smarter general AI for mages: v34.3",
                true,
                Some(1),
            )],
        ),
    ]
}

fn step3_item(
    mod_name: &str,
    tp_file: &str,
    id: &str,
    label: &str,
    raw: &str,
    order: usize,
    is_parent: bool,
) -> Step3ItemState {
    Step3ItemState {
        tp_file: tp_file.to_string(),
        component_id: id.to_string(),
        mod_name: mod_name.to_string(),
        component_label: label.to_string(),
        raw_line: raw.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: order,
        block_id: format!("{mod_name}::block0"),
        is_parent,
        parent_placeholder: false,
    }
}

fn demo_step3_items() -> Vec<Step3ItemState> {
    vec![
        step3_item(
            "BG2FixPack",
            "setup-BG2FixPack.tp2",
            "__PARENT__",
            "",
            "",
            0,
            true,
        ),
        step3_item(
            "BG2FixPack",
            "setup-BG2FixPack.tp2",
            "0",
            "Core Fixes",
            "~BG2FIXPACK/SETUP-BG2FIXPACK.TP2~ #0 #0 // BG2 Fixpack - Core Fixes: v13.4",
            1,
            false,
        ),
        step3_item("SCS", "setup-stratagems.tp2", "__PARENT__", "", "", 2, true),
        step3_item(
            "SCS",
            "setup-stratagems.tp2",
            "4000",
            "Smarter Mages",
            "~STRATAGEMS/SETUP-STRATAGEMS.TP2~ #0 #4000 // Smarter general AI for mages: v34.3",
            3,
            false,
        ),
    ]
}

fn setup_eet_step2(app: &mut OrchestratorApp) {
    app.wizard_state.step1.game_install = "EET".to_string();
    app.wizard_state.step2.active_game_tab = "BGEE".to_string();
    app.wizard_state.step2.bgee_mods = demo_step2_mods();
    app.wizard_state.step2.bg2ee_mods = demo_step2_mods();
    app.workspace_view.step2.details_open = false;
}

fn setup_eet_step3(app: &mut OrchestratorApp) {
    app.wizard_state.step1.game_install = "EET".to_string();
    app.wizard_state.step3.active_game_tab = "BGEE".to_string();
    app.wizard_state.step3.bgee_items = demo_step3_items();
    app.wizard_state.step3.bg2ee_items = demo_step3_items();
}

fn setup_eet_step4(app: &mut OrchestratorApp) {
    setup_eet_step3(app);
}

/// Produces a valid `BIO-MODLIST-V1:` share code via the production decode
/// contract (zlib + base64url of the format-version-1 JSON payload).
fn make_share_code() -> String {
    let payload = serde_json::json!({
        "format_version": 1,
        "bio_version": "0.1.0-seam",
        "game_install": "EET",
        "install_mode": "start_from_scratch",
        "name": "Tactical EET 2026",
        "author": "@b2bs",
        "weidu_logs": {
            "bgee": "~BG2FIXPACK/SETUP-BG2FIXPACK.TP2~ #0 #0 // BG2 Fixpack - Core Fixes: v13.4\n~STRATAGEMS/SETUP-STRATAGEMS.TP2~ #0 #4000 // Smarter mages: v34.3",
            "bg2ee": "~BG2FIXPACK/SETUP-BG2FIXPACK.TP2~ #0 #0 // BG2 Fixpack - Core Fixes: v13.4"
        },
        "source_overrides": { "mod_downloads_user_toml": "[[mods]]\nname = \"x\"" },
        "installed_refs": { "mod_installed_refs_toml": "[refs]\na = 1" }
    });
    let text = serde_json::to_string(&payload).expect("serialize payload");
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes()).expect("zlib write");
    let compressed = encoder.finish().expect("zlib finish");
    format!("BIO-MODLIST-V1:{}", base64url_encode(&compressed))
}

/// Mirrors the encoder in `core/app/modlist_share.rs` (table-driven base64url,
/// no padding) so the produced code round-trips through the production decoder.
fn base64url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        }
    }
    out
}

fn setup_install_preview(app: &mut OrchestratorApp) {
    let mut entry = ModlistEntry::default();
    entry.id = "SEAMPREVIEW01".to_string();
    entry.name = "Tactical EET 2026".to_string();
    entry.game = Game::EET;
    entry.destination_folder = std::env::temp_dir().to_string_lossy().to_string();
    entry.latest_share_code = Some(make_share_code());
    reinstall_route::start_reinstall(&entry, app);
    debug_assert_eq!(app.install_screen_state.stage, InstallStage::Preview);
    app.nav = NavDestination::Install;
}

// ---------------------------------------------------------------------------
// Tests — "before" set (seam-bug baseline).
// ---------------------------------------------------------------------------

#[test]
fn seam_settings() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_before(),
            "settings",
            tag,
            palette,
            |app| app.settings_screen_state.active_tab = SettingsTab::Paths,
            |app, ctx, ui| page_settings::render(ui, app, ctx),
            // Active tab bottom + panel top band; x=228, y=96, w=520, h=80.
            (228, 96, 520, 80),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_step2() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_before(),
            "step2",
            tag,
            palette,
            setup_eet_step2,
            |app, _ctx, ui| {
                let _ = workspace_step2::render(ui, app);
            },
            // Tab bottom (~y=160) and list-pane top; x=228, y=126, w=420, h=64.
            (228, 126, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_step3() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_before(),
            "step3",
            tag,
            palette,
            setup_eet_step3,
            |app, _ctx, ui| workspace_step3::render(ui, app),
            // Tab bottom (~y=118) and list-body box top; x=228, y=86, w=420, h=64.
            (228, 86, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_step4() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_before(),
            "step4",
            tag,
            palette,
            setup_eet_step4,
            |app, _ctx, ui| {
                let _ = workspace_step4::render(ui, app);
            },
            // Tab bottom (~y=140) and review-list box top; x=228, y=104, w=420, h=64.
            (228, 104, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_install_preview() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_before(),
            "install_preview",
            tag,
            palette,
            setup_install_preview,
            |app, ctx, ui| page_install::render(ui, app, ctx),
            // Preview tab strip (~y=214) and content-frame top; x=228, y=214, w=420, h=52.
            (228, 214, 420, 52),
        );
        assert_shot(&shot);
    }
}

// ---------------------------------------------------------------------------
// Tests — "after" set (seam-fix verification).
// ---------------------------------------------------------------------------

#[test]
fn seam_after_settings() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_after(),
            "settings",
            tag,
            palette,
            |app| app.settings_screen_state.active_tab = SettingsTab::Paths,
            |app, ctx, ui| page_settings::render(ui, app, ctx),
            (228, 96, 520, 80),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_after_step2() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_after(),
            "step2",
            tag,
            palette,
            setup_eet_step2,
            |app, _ctx, ui| {
                let _ = workspace_step2::render(ui, app);
            },
            (228, 126, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_after_step3() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_after(),
            "step3",
            tag,
            palette,
            setup_eet_step3,
            |app, _ctx, ui| workspace_step3::render(ui, app),
            (228, 86, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_after_step4() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_after(),
            "step4",
            tag,
            palette,
            setup_eet_step4,
            |app, _ctx, ui| {
                let _ = workspace_step4::render(ui, app);
            },
            (228, 104, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn seam_after_install_preview() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_after(),
            "install_preview",
            tag,
            palette,
            setup_install_preview,
            |app, ctx, ui| page_install::render(ui, app, ctx),
            (228, 214, 420, 52),
        );
        assert_shot(&shot);
    }
}

// ---------------------------------------------------------------------------
// Tests — corners fix verification (square top, rounded bottom on content boxes).
// ---------------------------------------------------------------------------

#[test]
fn corners_settings() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_corners(),
            "settings",
            tag,
            palette,
            |app| app.settings_screen_state.active_tab = SettingsTab::Paths,
            |app, ctx, ui| page_settings::render(ui, app, ctx),
            (228, 96, 520, 80),
        );
        assert_shot(&shot);
    }
}

#[test]
fn corners_step2() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_corners(),
            "step2",
            tag,
            palette,
            setup_eet_step2,
            |app, _ctx, ui| {
                let _ = workspace_step2::render(ui, app);
            },
            (228, 126, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn corners_step3() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_corners(),
            "step3",
            tag,
            palette,
            setup_eet_step3,
            |app, _ctx, ui| workspace_step3::render(ui, app),
            (228, 86, 420, 64),
        );
        assert_shot(&shot);
    }
}

#[test]
fn corners_step4() {
    for (tag, palette) in PALETTES {
        let shot = snap_surface(
            &out_dir_corners(),
            "step4",
            tag,
            palette,
            setup_eet_step4,
            |app, _ctx, ui| {
                let _ = workspace_step4::render(ui, app);
            },
            (228, 104, 420, 64),
        );
        assert_shot(&shot);
    }
}
