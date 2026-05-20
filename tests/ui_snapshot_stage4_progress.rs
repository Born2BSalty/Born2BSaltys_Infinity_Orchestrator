// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use bio::app::state::WizardState;
use bio::registry::model::{Game, ModlistEntry, ModlistState};
use bio::ui::orchestrator::widgets::render_screen_title;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, REDESIGN_NAV_WIDTH_PX, REDESIGN_STATUSBAR_HEIGHT_PX,
    REDESIGN_TITLEBAR_HEIGHT_PX, ThemePalette, redesign_border_strong, redesign_shell_bg,
    redesign_text_primary,
};
use bio::ui::step5::state_step5::Step5ConsoleViewState;
use bio::ui::workspace::step5::post_install_actions;

use eframe::egui;
use egui_kittest::Harness;
use std::path::{Path, PathBuf};

const CENTRAL_MARGIN: egui::Margin = egui::Margin {
    left: 28,
    right: 28,
    top: 24,
    bottom: 24,
};

const HEADER_NAME: &str = "Polished EET";
const BUTTON_RADIUS: u8 = 8;

const CELLS: [Cell; 2] = [
    Cell {
        w: 1280.0,
        h: 820.0,
        suffix: "1280x820",
    },
    Cell {
        w: 1045.0,
        h: 735.0,
        suffix: "1045x735",
    },
];

#[derive(Clone, Copy)]
struct Cell {
    w: f32,
    h: f32,
    suffix: &'static str,
}

impl Cell {
    const fn size(self) -> egui::Vec2 {
        egui::Vec2 {
            x: self.w,
            y: self.h,
        }
    }
}

fn install_running_state() -> WizardState {
    let mut s = WizardState::default();
    s.step5.install_running = true;
    s.step5.last_status_text = "Installing component 12 / 136".to_string();
    s.step1.game_install = "EET".to_string();
    s
}

fn install_modlist_entry() -> ModlistEntry {
    let mut e = ModlistEntry::default();
    e.id = "SNAP00000004".to_string();
    e.name = HEADER_NAME.to_string();
    e.game = Game::EET;
    e.destination_folder = "D:\\import test".to_string();
    e.state = ModlistState::InProgress;
    e.latest_share_code = Some("BIO-MODLIST-V1:SNAPSHOT".to_string());
    e
}

#[test]
fn render_stage4_progress_screen_matrix() {
    let out_dir = snapshot_out_dir();
    std::fs::create_dir_all(&out_dir).expect("create target/ui-snapshots dir");

    let mut written = Vec::new();

    for cell in CELLS {
        written.push(render_cell(cell, &out_dir));
    }

    assert_written(&written, 2, "expected 2 matrix-cell PNGs");
}

fn render_cell(cell: Cell, out_dir: &Path) -> PathBuf {
    let mut frame = 0;
    let mut harness = Harness::builder()
        .with_size(cell.size())
        .with_pixels_per_point(1.0)
        .build(move |ctx| render_frame(ctx, &mut frame));

    for _ in 0..8 {
        harness.run();
    }

    let img = harness
        .render()
        .expect("egui_kittest wgpu render() must produce an image");
    let path = out_dir.join(format!("stage4_progress__{}.png", cell.suffix));
    img.save(&path)
        .unwrap_or_else(|e| panic!("write PNG {}: {e}", path.display()));

    let abs = path.canonicalize().unwrap_or_else(|_| path.clone());
    println!(
        "SNAPSHOT  {}  stage-4 InstallProgressScreen (install-running)  -> {}",
        cell.suffix,
        abs.display()
    );
    path
}

fn render_frame(ctx: &egui::Context, frame: &mut u64) {
    if render_font_warmup(ctx, frame) {
        return;
    }

    let mut wizard_state = install_running_state();
    let entry = install_modlist_entry();
    let mut console_view = Step5ConsoleViewState::default();
    let palette = ThemePalette::Dark;
    let dev_mode = false;
    let exe_fingerprint = String::new();

    render_shell(ctx, |ui| {
        render_header(ui, palette);
        render_progress_body(
            ui,
            palette,
            &mut wizard_state,
            &entry,
            &mut console_view,
            dev_mode,
            &exe_fingerprint,
        );
    });
}

fn render_header(ui: &mut egui::Ui, palette: ThemePalette) {
    let sub = format!("{HEADER_NAME} \u{00B7} live install console");
    ui.horizontal_top(|ui| {
        let back_btn_w = 130.0;
        let title_w = (ui.available_width() - back_btn_w).max(160.0);
        ui.allocate_ui_with_layout(
            egui::vec2(title_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| render_screen_title(ui, palette, "Installing modlist", Some(&sub)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            ui.add_space(0.0);
            let _ = paint_back_to_import_btn(ui, palette);
        });
    });
    ui.add_space(10.0);
}

fn render_progress_body(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    wizard_state: &mut WizardState,
    entry: &ModlistEntry,
    console_view: &mut Step5ConsoleViewState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let _ = post_install_actions::render(ui, palette, wizard_state, entry);
            let _ = bio::ui::step5::page_step5::render(
                ui,
                wizard_state,
                console_view,
                None,
                None,
                dev_mode,
                exe_fingerprint,
            );
        });
}

fn render_font_warmup(ctx: &egui::Context, frame: &mut u64) -> bool {
    if *frame != 0 {
        *frame += 1;
        return false;
    }

    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
    *frame += 1;
    true
}

fn render_shell(body_ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) {
    render_top_panel(body_ctx, "scaffold_titlebar", REDESIGN_TITLEBAR_HEIGHT_PX);
    render_bottom_panel(body_ctx, "scaffold_statusbar", REDESIGN_STATUSBAR_HEIGHT_PX);

    let mut body = Some(body);
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(egui::Color32::from_rgb(0x0B, 0x11, 0x16)))
        .show(body_ctx, |ui| {
            render_nav_rail(ui);
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.inner_margin(CENTRAL_MARGIN))
                .show_inside(ui, |ui| {
                    if let Some(body) = body.take() {
                        body(ui);
                    }
                });
        });
}

fn render_top_panel(ctx: &egui::Context, id: &'static str, height: f32) {
    egui::TopBottomPanel::top(id)
        .exact_height(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn render_bottom_panel(ctx: &egui::Context, id: &'static str, height: f32) {
    egui::TopBottomPanel::bottom(id)
        .exact_height(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show(ctx, fill_panel);
}

fn render_nav_rail(ui: &mut egui::Ui) {
    egui::SidePanel::left("scaffold_rail")
        .exact_width(REDESIGN_NAV_WIDTH_PX)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::NONE)
        .show_inside(ui, |ui| {
            fill_panel(ui);
            ui.set_min_width(REDESIGN_NAV_WIDTH_PX);
        });
}

fn fill_panel(ui: &mut egui::Ui) {
    let r = ui.max_rect();
    ui.painter()
        .rect_filled(r, 0.0, egui::Color32::from_rgb(0x15, 0x22, 0x2B));
    ui.allocate_space(ui.available_size());
}

fn paint_back_to_import_btn(ui: &mut egui::Ui, palette: ThemePalette) -> egui::Response {
    let pad_x = 10.0;
    let pad_y = 4.0;
    let font_size = 12.0;
    let gap = 5.0;

    let fill = redesign_shell_bg(palette);
    let text_color = redesign_text_primary(palette);
    let border = redesign_border_strong(palette);

    let glyph_font = egui::FontId::new(font_size, egui::FontFamily::Name("firacode_nerd".into()));
    let prose_font = egui::FontId::new(font_size, egui::FontFamily::Name("poppins_medium".into()));
    let glyph_galley =
        ui.painter()
            .layout_no_wrap("\u{2190}".to_string(), glyph_font.clone(), text_color);
    let prose_galley =
        ui.painter()
            .layout_no_wrap("back to import".to_string(), prose_font.clone(), text_color);

    let content_w = glyph_galley.size().x + gap + prose_galley.size().x;
    let content_h = glyph_galley.size().y.max(prose_galley.size().y);
    let desired = egui::vec2(content_w + pad_x * 2.0, content_h + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let spec = BackButtonPaint {
            rect,
            fill,
            border,
            text_color,
            glyph_font,
            prose_font,
            glyph_w: glyph_galley.size().x,
            content_w,
        };
        paint_back_button_rect(ui, spec);
    }

    response
}

struct BackButtonPaint {
    rect: egui::Rect,
    fill: egui::Color32,
    border: egui::Color32,
    text_color: egui::Color32,
    glyph_font: egui::FontId,
    prose_font: egui::FontId,
    glyph_w: f32,
    content_w: f32,
}

fn paint_back_button_rect(ui: &egui::Ui, spec: BackButtonPaint) {
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(BUTTON_RADIUS);
    painter.rect_filled(spec.rect, radius, spec.fill);
    painter.rect_stroke(
        spec.rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, spec.border),
        egui::StrokeKind::Inside,
    );
    let start_x = spec.rect.center().x - spec.content_w / 2.0;
    let cy = spec.rect.center().y;
    painter.text(
        egui::pos2(start_x, cy),
        egui::Align2::LEFT_CENTER,
        "\u{2190}",
        spec.glyph_font,
        spec.text_color,
    );
    painter.text(
        egui::pos2(start_x + spec.glyph_w + 5.0, cy),
        egui::Align2::LEFT_CENTER,
        "back to import",
        spec.prose_font,
        spec.text_color,
    );
}

fn assert_written(written: &[PathBuf], expected: usize, message: &str) {
    for p in written {
        let meta = std::fs::metadata(p)
            .unwrap_or_else(|_| panic!("expected PNG to exist: {}", p.display()));
        assert!(
            meta.len() > 0,
            "rendered PNG is empty (renderer produced no pixels): {}",
            p.display()
        );
    }
    assert_eq!(written.len(), expected, "{message}");
}

fn snapshot_out_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let target_dir = tmp
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(tmp);
    target_dir.join("ui-snapshots")
}
