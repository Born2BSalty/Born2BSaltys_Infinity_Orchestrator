// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    ThemePalette, redesign_border_strong, redesign_page_bg, redesign_rail_bg, redesign_shell_bg,
    redesign_text_disabled, redesign_text_faint, redesign_warning,
};

use bio::app::state::Step3ItemState;
use bio::ui::step3::state_blocks_step3 as blocks;

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 1000.0;
const HEIGHT: f32 = 640.0;

const PALETTES: [(&str, ThemePalette); 2] =
    [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];

fn evidence_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("infinity_orchestrator")
        .join("phase8-evidence")
}

fn make_parent(mod_name: &str, placeholder: bool) -> Step3ItemState {
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
        parent_placeholder: placeholder,
    }
}

fn make_child(mod_name: &str, id: &str, label: &str, raw_line: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}.tp2"),
        component_id: id.to_string(),
        mod_name: mod_name.to_string(),
        component_label: label.to_string(),
        raw_line: raw_line.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: format!("{mod_name}::block0"),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn render_font_frame(ctx: &egui::Context) {
    install_redesign_fonts(ctx);
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.allocate_space(ui.available_size());
    });
}

fn render_list_body_preview(ctx: &egui::Context, palette: ThemePalette) {
    let items = vec![
        make_parent("BG2FixPack", false),
        make_child(
            "BG2FixPack",
            "1",
            "Core Fixes",
            "~BG2FixPack\\setup-BG2FixPack.tp2~ #0 #0 // BG2 Fixpack - Core Fixes: v13.4",
        ),
        make_child(
            "BG2FixPack",
            "2",
            "Creature Corrections",
            "~BG2FixPack\\setup-BG2FixPack.tp2~ #0 #2 // Creature Corrections: v13.4",
        ),
        make_child(
            "BG2FixPack",
            "3",
            "NPC-Related Fixes",
            "~BG2FixPack\\setup-BG2FixPack.tp2~ #0 #3 // NPC-Related Fixes: v13.4",
        ),
        // Collapsed mod group:
        make_parent("SCS", false),
        make_child(
            "SCS",
            "10",
            "Smarter Mages",
            "~stratagems\\setup-stratagems.tp2~ #0 #4000 // Smarter general AI for mages: v34.3",
        ),
        // Split-target (placeholder) group:
        make_parent("SCS", true),
        make_child(
            "SCS",
            "20",
            "Better calls for help",
            "~stratagems\\setup-stratagems.tp2~ #0 #4020 // Better calls for help: v34.3",
        ),
    ];

    let collapsed = vec!["SCS::block0".to_string()];
    let locked = vec!["BG2FixPack::block0".to_string()];

    let visible = blocks::visible_indices(&items, &collapsed);

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(redesign_page_bg(palette)))
        .show(ctx, |ui| {
            ui.set_width(ui.available_width());

            let avail = ui.available_size();
            let (box_rect, _) = ui.allocate_exact_size(avail, egui::Sense::hover());

            if ui.is_rect_visible(box_rect) {
                let painter = ui.painter();
                let radius = egui::CornerRadius::same(3);
                painter.rect_filled(box_rect, radius, redesign_shell_bg(palette));
                painter.rect_stroke(
                    box_rect,
                    radius,
                    egui::Stroke::new(1.5, redesign_border_strong(palette)),
                    egui::StrokeKind::Inside,
                );
            }

            let inner = box_rect.shrink(10.0);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(inner.intersect(ui.clip_rect()));

            egui::ScrollArea::both()
                .id_salt("render_gate_step3")
                .auto_shrink([false, false])
                .show(&mut child, |ui| {
                    for &idx in &visible {
                        if items[idx].is_parent {
                            render_mock_header_row(ui, &items, idx, &locked, &collapsed, palette);
                        } else {
                            render_mock_child_row(ui, &items, idx, palette);
                        }
                    }
                });
        });
}

fn render_mock_header_row(
    ui: &mut egui::Ui,
    items: &[Step3ItemState],
    idx: usize,
    locked: &[String],
    collapsed: &[String],
    palette: ThemePalette,
) {
    let block_id = &items[idx].block_id;
    let is_locked = locked.contains(block_id);
    let is_collapsed = collapsed.contains(block_id);
    let child_count = blocks::count_children_in_block(items, idx);
    let parent_placeholder = items[idx].parent_placeholder;
    let mod_name = &items[idx].mod_name;

    ui.add_space(2.0);

    let bg = redesign_rail_bg(palette);
    let header_height = 26.0;
    let avail_w = ui.available_width();
    let (bg_rect, _) = ui.allocate_space(egui::vec2(avail_w, 0.0));
    let _ = bg_rect;

    ui.horizontal(|ui| {
        ui.add_space(4.0);

        let lock_color = if is_locked {
            redesign_warning(palette)
        } else {
            redesign_text_disabled(palette)
        };
        let lock_glyph = if is_locked { "🔒" } else { "🔓" };
        ui.colored_label(lock_color, lock_glyph);
        ui.add_space(4.0);

        let chevron = if is_collapsed { "🔗 ▸" } else { "🔗 ▾" };
        ui.colored_label(redesign_text_faint(palette), chevron);
        ui.add_space(4.0);

        let title = if parent_placeholder {
            format!("{mod_name} (split target) ({child_count})")
        } else {
            format!("{mod_name} ({child_count})")
        };

        let locked_label = if is_locked {
            format!("{title} [locked]")
        } else {
            title
        };

        let _ = header_height;
        let bg_rect2 = ui.cursor().with_min_y(ui.cursor().min.y - 2.0);
        let _ = bg_rect2;
        if ui.is_rect_visible(ui.cursor()) {
            let row_rect =
                egui::Rect::from_min_size(ui.cursor().min, egui::vec2(avail_w - 20.0, 22.0));
            ui.painter().rect_filled(row_rect, 2.0, bg);
        }

        ui.label(
            egui::RichText::new(locked_label)
                .strong()
                .color(redesign_text_faint(palette)),
        );
    });

    let dashed_color = redesign_text_faint(palette);
    ui.painter().hline(
        ui.cursor().x_range(),
        ui.cursor().min.y,
        egui::Stroke::new(1.0, dashed_color),
    );
}

fn render_mock_child_row(
    ui: &mut egui::Ui,
    items: &[Step3ItemState],
    idx: usize,
    palette: ThemePalette,
) {
    let item = &items[idx];
    let faint = redesign_text_faint(palette);

    ui.horizontal(|ui| {
        ui.add_space(18.0);

        ui.colored_label(faint, "≡");
        ui.add_space(4.0);

        let lineno = idx;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 15.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                egui::pos2(rect.right(), rect.center().y),
                egui::Align2::RIGHT_CENTER,
                lineno.to_string(),
                egui::FontId::new(11.0, egui::FontFamily::Name("firacode_nerd".into())),
                faint,
            );
        }

        ui.add_space(6.0);

        let text = if item.raw_line.is_empty() {
            format!(
                "~{}\\{}~ #0 #{} // {}",
                item.mod_name, item.tp_file, item.component_id, item.component_label
            )
        } else {
            item.raw_line.clone()
        };
        ui.label(
            egui::RichText::new(text)
                .size(13.0)
                .family(egui::FontFamily::Monospace),
        );
    });

    let sep_y = ui.cursor().min.y;
    let x0 = ui.cursor().left();
    let x1 = ui.cursor().right();
    let stroke = egui::Stroke::new(1.0, faint);
    for x in std::iter::successors(Some(x0), |&prev| {
        let next = prev + 6.0;
        if next < x1 { Some(next) } else { None }
    }) {
        let end = (x + 3.0).min(x1);
        ui.painter()
            .line_segment([egui::pos2(x, sep_y), egui::pos2(end, sep_y)], stroke);
    }
    ui.add_space(1.0);
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
fn render_step3_list_body_item2_impl() {
    let dir = evidence_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");

    for (label, palette) in PALETTES {
        let path = snap(&dir, &format!("item2-impl-step3-{label}"), move |ctx| {
            render_list_body_preview(ctx, palette);
        });
        assert!(
            path.exists() && path.metadata().is_ok_and(|m| m.len() > 0),
            "render-gate PNG must be non-empty: {}",
            path.display()
        );
    }
}
