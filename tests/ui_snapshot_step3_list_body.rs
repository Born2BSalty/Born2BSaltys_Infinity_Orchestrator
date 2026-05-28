// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use bio::app::state::Step3ItemState;
use bio::ui::shared::redesign_fonts::install_redesign_fonts;
use bio::ui::shared::redesign_tokens::{
    ThemePalette, redesign_border_strong, redesign_page_bg, redesign_rail_bg, redesign_shell_bg,
    redesign_text_disabled, redesign_text_faint, redesign_text_fainter, redesign_warning,
    redesign_with_alpha,
};
use bio::ui::shared::theme_global::accent_path;
use bio::ui::step3::state_blocks_step3 as blocks;

use eframe::egui;
use egui_kittest::Harness;

const WIDTH: f32 = 1000.0;
const HEIGHT: f32 = 640.0;
const GATE_HEADER_VPAD_TOP: f32 = 6.0;
const GATE_HEADER_VPAD_BOT: f32 = 2.0;
const GATE_GROUP_GAP: f32 = 6.0;
const GATE_DOT_STEP_PX: f32 = 7.0;
const GATE_DOT_RADIUS: f32 = 0.7;

const PALETTES: [(&str, ThemePalette); 2] =
    [("dark", ThemePalette::Dark), ("light", ThemePalette::Light)];

/// Extracts the version token from a `WeiDU` raw install line comment, if present.
///
/// Mirrors the logic of `bio::parser::weidu_version::parse_version` without relying on
/// a `pub(crate)` symbol.
fn gate_parse_version(raw_line: &str) -> Option<String> {
    let comment = raw_line.split_once("//")?.1.trim().to_ascii_lowercase();
    let segment = comment.split(':').next_back()?;
    segment.split_whitespace().rev().find_map(|tok| {
        let tok = tok.trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
            )
        });
        let lower = tok;
        let (stripped, had_v) = lower.strip_prefix("version").map_or_else(
            || {
                if lower.starts_with('v')
                    && lower.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
                {
                    (&lower[1..], true)
                } else {
                    (lower, false)
                }
            },
            |rest| (rest.trim_start_matches([' ', '-', '_', ':']), true),
        );
        let start = stripped.find(|c: char| c.is_ascii_digit())?;
        let tail = &stripped[start..];
        let end = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .unwrap_or(tail.len());
        let candidate = &tail[..end];
        (candidate.chars().any(|c| c.is_ascii_digit()) && (candidate.contains('.') || had_v))
            .then(|| candidate.to_string())
    })
}

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
    let avail_w = ui.available_width();

    ui.horizontal(|ui| {
        ui.add_space(4.0);

        // Lock glyph from FiraCode Nerd Font PUA range.
        let lock_color = if is_locked {
            redesign_warning(palette)
        } else {
            redesign_text_disabled(palette)
        };
        let lock_glyph = if is_locked { "\u{F023}" } else { "\u{F09C}" };
        let (lock_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        if ui.is_rect_visible(lock_rect) {
            ui.painter().text(
                lock_rect.center(),
                egui::Align2::CENTER_CENTER,
                lock_glyph,
                egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
                lock_color,
            );
        }
        ui.add_space(4.0);

        // Chevron using FiraCode Nerd Font (Geometric Shapes block — font-covered).
        let chevron = if is_collapsed { "▸" } else { "▾" };
        let (chev_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        if ui.is_rect_visible(chev_rect) {
            ui.painter().text(
                chev_rect.center(),
                egui::Align2::CENTER_CENTER,
                chevron,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
                redesign_text_faint(palette),
            );
        }
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

    // Dotted separator: tiny filled circles at 50% alpha of text_fainter.
    let sep_y = ui.cursor().min.y;
    let x0 = ui.cursor().left();
    let x1 = ui.cursor().right();
    let base_color = redesign_text_fainter(palette);
    let dot_color = redesign_with_alpha(base_color, 1, 2);
    let dot_step = 3.5_f32;
    let dot_radius = 0.9_f32;
    for x in std::iter::successors(Some(x0), |&prev| {
        let next = prev + dot_step;
        if next <= x1 { Some(next) } else { None }
    }) {
        ui.painter()
            .circle_filled(egui::pos2(x, sep_y), dot_radius, dot_color);
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

#[test]
fn render_step3_list_body_fix2() {
    let dir = evidence_dir();
    std::fs::create_dir_all(&dir).expect("create evidence dir");

    for (label, palette) in PALETTES {
        let path = snap(&dir, &format!("item2-impl-fix2-{label}"), move |ctx| {
            render_list_body_preview(ctx, palette);
        });
        assert!(
            path.exists() && path.metadata().is_ok_and(|m| m.len() > 0),
            "render-gate PNG must be non-empty: {}",
            path.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Polish-round-2 render gate
// ---------------------------------------------------------------------------

fn render_gate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("render-gate")
}

fn make_polish_scene() -> (Vec<Step3ItemState>, Vec<String>, Vec<String>, Vec<usize>) {
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
        make_parent("SCS", false),
        make_child(
            "SCS",
            "10",
            "Smarter Mages",
            "~stratagems\\setup-stratagems.tp2~ #0 #4000 // Smarter general AI for mages: v34.3",
        ),
        make_child(
            "SCS",
            "20",
            "Better calls for help",
            "~stratagems\\setup-stratagems.tp2~ #0 #4020 // Better calls for help: v34.3",
        ),
        make_parent("SoD", false),
        make_child(
            "SoD",
            "100",
            "Hidden Component",
            "~SoD\\setup-SoD.tp2~ #0 #100 // Hidden: v1.0",
        ),
    ];
    let collapsed = vec!["SoD::block0".to_string()];
    let locked = vec!["BG2FixPack::block0".to_string()];
    let visible = blocks::visible_indices(&items, &collapsed);
    (items, collapsed, locked, visible)
}

fn render_polish_groups(
    ui: &mut egui::Ui,
    items: &[Step3ItemState],
    collapsed: &[String],
    locked: &[String],
    visible: &[usize],
    palette: ThemePalette,
) {
    ui.set_min_width(ui.available_width());
    let mut child_counter = 0usize;
    let mut first_group = true;
    let mut pos = 0;
    while pos < visible.len() {
        let idx = visible[pos];
        if !items[idx].is_parent {
            child_counter += 1;
            let fallback_x = (ui.cursor().left(), ui.cursor().right());
            render_polish_child_row(ui, items, idx, child_counter, fallback_x, false, palette);
            pos += 1;
            continue;
        }
        if !first_group {
            ui.add_space(GATE_GROUP_GAP);
        }
        first_group = false;
        let block_id = items[idx].block_id.clone();

        // Header bar: viewport-clamped, bg fill, dotted border on header only.
        let viewport_w = ui.available_width().max(0.0);
        let top_cursor = ui.cursor().min;

        // Reserve a paint slot so the bg paints behind widgets.
        let bg_shape_id = ui.painter().add(egui::Shape::Noop);

        let scope_resp = ui.scope(|ui| {
            ui.set_min_width(viewport_w);
            ui.add_space(GATE_HEADER_VPAD_TOP);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                render_polish_header_row(ui, items, idx, locked, collapsed, palette);
            });
            ui.add_space(GATE_HEADER_VPAD_BOT);
        });
        pos += 1;

        let header_rect = egui::Rect::from_min_size(
            top_cursor,
            egui::vec2(viewport_w, scope_resp.response.rect.height()),
        );

        ui.painter().set(
            bg_shape_id,
            egui::Shape::rect_filled(
                header_rect,
                egui::CornerRadius::ZERO,
                redesign_rail_bg(palette),
            ),
        );

        let dot_color = redesign_with_alpha(accent_path(), 1, 2);
        paint_gate_dotted_rect(
            ui,
            header_rect,
            dot_color,
            GATE_DOT_STEP_PX,
            GATE_DOT_RADIUS,
        );

        let x_bounds = (header_rect.left(), header_rect.right());

        while pos < visible.len() {
            let child_idx = visible[pos];
            if items[child_idx].is_parent || items[child_idx].block_id != block_id {
                break;
            }
            child_counter += 1;
            let next_pos = pos + 1;
            let is_last = next_pos >= visible.len()
                || items[visible[next_pos]].is_parent
                || items[visible[next_pos]].block_id != block_id;
            render_polish_child_row(
                ui,
                items,
                child_idx,
                child_counter,
                x_bounds,
                is_last,
                palette,
            );
            pos += 1;
        }
    }
}

/// Renders the polished Step-3 list body scene with a locked group, an unlocked group,
/// and a collapsed group — showing header-only dotted borders and dotted row separators.
fn render_polish_preview(ctx: &egui::Context, palette: ThemePalette) {
    let (items, collapsed, locked, visible) = make_polish_scene();

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
            child.add_space(3.0);
            egui::ScrollArea::both()
                .id_salt("render_gate_polish")
                .auto_shrink([false, false])
                .show(&mut child, |ui| {
                    render_polish_groups(ui, &items, &collapsed, &locked, &visible, palette);
                });
        });
}

/// Renders the header-row content — lock glyph, chevron, title label, and optional version.
///
/// Must be called from inside a `ui.horizontal` closure in the caller.
fn render_polish_header_row(
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
    let mod_name = &items[idx].mod_name;

    let mod_version = items
        .iter()
        .filter(|i| !i.is_parent && &i.block_id == block_id)
        .find_map(|i| gate_parse_version(&i.raw_line));

    // Lock glyph from FiraCode Nerd Font PUA range.
    let lock_color = if is_locked {
        redesign_warning(palette)
    } else {
        redesign_text_disabled(palette)
    };
    let lock_glyph = if is_locked { "\u{F023}" } else { "\u{F09C}" };
    let (lock_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    if ui.is_rect_visible(lock_rect) {
        ui.painter().text(
            lock_rect.center(),
            egui::Align2::CENTER_CENTER,
            lock_glyph,
            egui::FontId::new(12.0, egui::FontFamily::Name("firacode_nerd".into())),
            lock_color,
        );
    }
    ui.add_space(4.0);

    // Chevron.
    let chevron = if is_collapsed { "\u{25B8}" } else { "\u{25BE}" };
    let (chev_rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    if ui.is_rect_visible(chev_rect) {
        ui.painter().text(
            chev_rect.center(),
            egui::Align2::CENTER_CENTER,
            chevron,
            egui::FontId::new(12.0, egui::FontFamily::Monospace),
            redesign_text_faint(palette),
        );
    }
    ui.add_space(4.0);

    let title = format!("{mod_name} ({child_count})");
    let label = if is_locked {
        format!("{title} [locked]")
    } else {
        title
    };
    ui.label(
        egui::RichText::new(label)
            .strong()
            .color(redesign_text_faint(palette)),
    );

    if let Some(ref v) = mod_version {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("v{v}")).color(redesign_text_faint(palette)));
    }
}

fn render_polish_child_row(
    ui: &mut egui::Ui,
    items: &[Step3ItemState],
    idx: usize,
    lineno: usize,
    (group_x0, group_x1): (f32, f32),
    is_last_in_group: bool,
    palette: ThemePalette,
) {
    let item = &items[idx];
    ui.horizontal(|ui| {
        ui.add_space(18.0);

        let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 15.0), egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().text(
                egui::pos2(rect.right(), rect.center().y),
                egui::Align2::RIGHT_CENTER,
                lineno.to_string(),
                egui::FontId::new(11.0, egui::FontFamily::Name("firacode_nerd".into())),
                redesign_text_faint(palette),
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

    if !is_last_in_group {
        // Full-width dotted separator: wider spacing, smaller dots, very faint.
        let sep_y = ui.cursor().min.y;
        let base_color = redesign_text_fainter(palette);
        let dot_color = redesign_with_alpha(base_color, 1, 8);
        let dot_step = 7.0_f32;
        let dot_radius = 0.5_f32;
        for x in std::iter::successors(Some(group_x0), |&prev| {
            let next = prev + dot_step;
            if next <= group_x1 { Some(next) } else { None }
        }) {
            ui.painter()
                .circle_filled(egui::pos2(x, sep_y), dot_radius, dot_color);
        }
    }
    ui.add_space(1.0);
}

/// Paints a dotted rectangle border, placing filled circles along each edge.
///
/// Corners are traversed in order; the dot cycle restarts at each corner.
fn paint_gate_dotted_rect(
    ui: &egui::Ui,
    rect: egui::Rect,
    color: egui::Color32,
    step_px: f32,
    radius: f32,
) {
    let painter = ui.painter();
    let corners = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];
    for (from, to) in corners {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let edge_len = dx.hypot(dy);
        if edge_len < 1.0 {
            continue;
        }
        let ux = dx / edge_len;
        let uy = dy / edge_len;
        for t in std::iter::successors(Some(0.0_f32), |&prev| {
            let next = prev + step_px;
            if next <= edge_len { Some(next) } else { None }
        }) {
            let pt = egui::pos2(ux.mul_add(t, from.x), uy.mul_add(t, from.y));
            painter.circle_filled(pt, radius, color);
        }
    }
}

#[test]
fn render_step3_polish_round2() {
    let dir = render_gate_dir();
    std::fs::create_dir_all(&dir).expect("create render-gate dir");

    let mut written: Vec<PathBuf> = Vec::new();

    for (label, palette) in PALETTES {
        let path = snap(&dir, &format!("step3_polish_{label}"), move |ctx| {
            render_polish_preview(ctx, palette);
        });
        assert!(
            path.exists() && path.metadata().is_ok_and(|m| m.len() > 0),
            "render-gate PNG must be non-empty: {}",
            path.display()
        );
        written.push(path);
    }

    assert_eq!(written.len(), 2, "expected 2 PNGs (dark + light)");
}
