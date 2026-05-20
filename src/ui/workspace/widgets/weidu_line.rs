// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::app::step5::diagnostics::format_step4_item;
use crate::ui::shared::redesign_tokens::{ThemePalette, redesign_success, redesign_text_faint};

const LINE_FONT_SIZE: f32 = 13.0;
const LINENO_FONT_SIZE: f32 = 12.0;
const LINENO_DIGIT_PX: f32 = 9.0;
const LINENO_PAD_PX: f32 = 4.0;
const LINENO_GAP_PX: f32 = 10.0;

#[must_use]
pub fn lineno_column_width(max_digits: usize) -> f32 {
    let digit_count = f32::from(u16::try_from(max_digits).unwrap_or(u16::MAX));
    digit_count.mul_add(LINENO_DIGIT_PX, LINENO_PAD_PX)
}

pub fn render_weidu_line(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    item: &Step3ItemState,
    line_number: Option<usize>,
    lineno_col_w: f32,
) {
    let text = format_step4_item(item);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        if let Some(n) = line_number {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(lineno_col_w, LINENO_FONT_SIZE + 4.0),
                egui::Sense::hover(),
            );
            if ui.is_rect_visible(rect) {
                ui.painter().text(
                    egui::pos2(rect.right(), rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    n.to_string(),
                    egui::FontId::new(
                        LINENO_FONT_SIZE,
                        egui::FontFamily::Name("firacode_nerd".into()),
                    ),
                    redesign_text_faint(palette),
                );
            }
            ui.add_space(LINENO_GAP_PX);
        }

        let job = build_weidu_job(ui, palette, &text);
        ui.label(egui::WidgetText::from(job));
    });
}

fn build_weidu_job(ui: &egui::Ui, palette: ThemePalette, text: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::FontId::new(
        LINE_FONT_SIZE,
        egui::FontFamily::Name("firacode_nerd".into()),
    );
    job.wrap.max_width = f32::INFINITY;
    let _ = ui;

    let path_color = egui::Color32::from_rgb(0xD4, 0xA3, 0x5C);
    let nums_color = egui::Color32::from_rgb(0x2F, 0x6F, 0xB7);
    let comment_color = redesign_success(palette);

    let trimmed_start = text.trim_start();
    if trimmed_start.starts_with("//") {
        append(&mut job, text, &mono, comment_color);
        return job;
    }

    if let Some(path_start) = text.find('~')
        && let Some(path_end_rel) = text[path_start + 1..].find('~')
    {
        let path_end = path_start + path_end_rel + 2;
        let comment_start = text[path_end..].find("//").map(|idx| path_end + idx);

        append(&mut job, &text[..path_start], &mono, comment_color);
        append(&mut job, &text[path_start..path_end], &mono, path_color);
        if let Some(comment_start) = comment_start {
            append(&mut job, &text[path_end..comment_start], &mono, nums_color);
            append(&mut job, &text[comment_start..], &mono, comment_color);
        } else {
            append(&mut job, &text[path_end..], &mono, nums_color);
        }
    } else {
        append(&mut job, text, &mono, comment_color);
    }

    job
}

fn append(
    job: &mut egui::text::LayoutJob,
    text: &str,
    font_id: &egui::FontId,
    color: egui::Color32,
) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font_id.clone(),
            color,
            ..Default::default()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn item(raw: &str, tp_file: &str, mod_name: &str, id: &str, label: &str) -> Step3ItemState {
        Step3ItemState {
            tp_file: tp_file.to_string(),
            component_id: id.to_string(),
            mod_name: mod_name.to_string(),
            component_label: label.to_string(),
            raw_line: raw.to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 1,
            block_id: String::new(),
            is_parent: false,
            parent_placeholder: false,
        }
    }

    #[test]
    fn lineno_column_width_scales_with_digit_count() {
        assert_f32_eq(lineno_column_width(1), 13.0);
        assert_f32_eq(lineno_column_width(2), 22.0);
        assert_f32_eq(lineno_column_width(3), 31.0);
        assert!(lineno_column_width(3) > lineno_column_width(1));
    }

    #[test]
    fn synthesised_line_matches_bio_format() {
        let it = item("", "EEFIXPACK.TP2", "EEFixPack", "2", "Game Text Update");
        let line = format_step4_item(&it);
        assert_eq!(line, "~EEFixPack\\EEFIXPACK.TP2~ #0 #2 // Game Text Update");
    }

    #[test]
    fn raw_line_is_normalised_not_resynthesised() {
        let it = item(
            "~/abs/path/EEFIXPACK/EEFIXPACK.TP2~ #0 #5 // Drow Item Restorations",
            "EEFIXPACK.TP2",
            "EEFixPack",
            "5",
            "Drow Item Restorations",
        );
        let line = format_step4_item(&it);
        assert_eq!(
            line,
            "~EEFIXPACK\\EEFIXPACK.TP2~ #0 #5 // Drow Item Restorations"
        );
    }

    #[test]
    fn three_colour_split_matches_wireframe_hues() {
        let palette = ThemePalette::Dark;
        let ctx = egui::Context::default();
        let mut produced: Vec<(String, egui::Color32)> = Vec::new();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let job = build_weidu_job(
                    ui,
                    palette,
                    "~EEFIXPACK\\EEFIXPACK.TP2~ #0 #2 // Game Text Update",
                );
                for s in &job.sections {
                    let txt = job.text[s.byte_range.clone()].to_string();
                    produced.push((txt, s.format.color));
                }
            });
        });
        assert_eq!(produced.len(), 3, "path + numbers + comment = 3 runs");
        assert!(produced[0].0.starts_with('~') && produced[0].0.ends_with('~'));
        assert_eq!(produced[0].1, egui::Color32::from_rgb(0xD4, 0xA3, 0x5C));
        assert!(produced[1].0.contains("#0 #2"));
        assert_eq!(produced[1].1, egui::Color32::from_rgb(0x2F, 0x6F, 0xB7));
        assert!(produced[2].0.contains("// Game Text Update"));
        assert_eq!(produced[2].1, redesign_success(palette));
    }

    #[test]
    fn pure_comment_line_is_single_run() {
        let palette = ThemePalette::Dark;
        let ctx = egui::Context::default();
        let mut runs = 0usize;
        let mut color = egui::Color32::TRANSPARENT;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let job = build_weidu_job(ui, palette, "// Log of Currently Installed WeiDU Mods");
                runs = job.sections.len();
                if let Some(s) = job.sections.first() {
                    color = s.format.color;
                }
            });
        });
        assert_eq!(runs, 1);
        assert_eq!(color, redesign_success(palette));
    }
}
