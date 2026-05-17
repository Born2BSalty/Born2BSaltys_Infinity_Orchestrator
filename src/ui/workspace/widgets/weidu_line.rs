// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `weidu_line` — the three-colour WeiDU-line renderer for the Step-4 review
// list (P6.T2b, SPEC §6.7 / §8.1).
//
// ## What this is (and is NOT)
//
// This is **net-new redesign chrome**, the C4 approach: it reuses BIO's
// canonical line *formatter* (`bio::app::step5::diagnostics::format_step4_item`
// — the exact `pub(crate) fn` BIO's own Step-4 / diagnostics export use, so
// the textual content is bit-for-bit BIO-fidelity) but renders the colours
// with the **redesign tokens**, not BIO's legacy `theme_global` palette. BIO's
// `bio::ui::step4::content_step4::render_weidu_colored_line` (which reads
// `theme_global::accent_path()` / `accent_numbers()` / `success()`) is
// **not** called — Step 4's rendering surface is net-new per C4.
//
// ## The canonical line shape (BIO `format_step4_item` output)
//
//   ~<FOLDER>\<FILE>~ #<lang> #<id> // <comment>
//   e.g. ~EEFIXPACK\EEFIXPACK.TP2~ #0 #2 // Game Text Update: Beta 2
//
// (A pure-comment line — `// ...` with no `~...~` — is rendered entirely in
// the comment colour, mirroring BIO's `render_weidu_colored_line` early-out.)
//
// ## The three-colour split (SPEC §6.7, plan-pinned redesign-token mapping)
//
// SPEC §6.7 names the three parts (TP2 path / component numbers / comment).
// Its literal hex values (`#d4a35c` / `#2f6fb7` / success-green) are BIO's
// **legacy** renderer's colours; the redesign uses tokens. P6.T2b + the
// phase-06 `weidu_line.rs` file-inventory row pin the redesign token mapping
// (the work order is authoritative on which token paints each segment in the
// orchestrator's net-new surface):
//   - `<tp2_file>`   (`~...~`)         → `redesign_accent_deep`
//   - `<component_id>` (`#0 #1030`)    → `redesign_text_muted`
//   - `<component_label>` (`// ...`)   → `redesign_text_primary`
//   - optional line-number prefix      → `redesign_text_faint`
//
// Monospace (FiraCode Nerd) per the wireframe `WeiduLine`
// (`screens.jsx:795-802`) — `firacode_nerd` is the full bundled build, so the
// `~` `#` `/` ASCII it contains render correctly (HANDOFF symbol-glyph rule:
// base-FiraCode ASCII is covered; no vector needed here).
//
// SPEC: §6.7 (WeiDU-line three-colour syntax), §8.1 (Step-4 review list),
//       §1 (decision order — reuse BIO's formatter, net-new render).

// rationale: f32→u8 line-number-column rounding of a small positive constant —
// correct by construction (Cat 2).
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use eframe::egui;

use crate::app::state::Step3ItemState;
use crate::app::step5::diagnostics::format_step4_item;
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_accent_deep, redesign_text_faint, redesign_text_muted,
    redesign_text_primary,
};

/// Monospace font size for the WeiDU line (wireframe `WeiduLine` `fontSize:
/// 13`).
const LINE_FONT_SIZE: f32 = 13.0;
/// Line-number prefix font size (wireframe `OrderPanel` line-number span
/// `fontSize: 12`).
const LINENO_FONT_SIZE: f32 = 12.0;
/// Per-digit pixel budget for the right-aligned line-number column (wireframe
/// `OrderPanel`: `lineNumWidth = String(selected.length).length * 9 + 4`).
const LINENO_DIGIT_PX: f32 = 9.0;
/// Constant slack added to the line-number column width (wireframe `+ 4`).
const LINENO_PAD_PX: f32 = 4.0;
/// Gap between the line-number column and the WeiDU line (wireframe
/// `OrderPanel` row `gap: 10`).
const LINENO_GAP_PX: f32 = 10.0;

/// Width of the right-aligned line-number column for a list whose largest
/// number has `max_digits` digits (wireframe `String(n).length * 9 + 4`).
pub fn lineno_column_width(max_digits: usize) -> f32 {
    max_digits as f32 * LINENO_DIGIT_PX + LINENO_PAD_PX
}

/// Render one WeiDU line for `item`, optionally prefixed by a right-aligned
/// `line_number` (1-based) in a `lineno_col_w`-wide faint column.
///
/// The textual content comes from BIO's canonical `format_step4_item`
/// (BIO-fidelity); the colours are the redesign tokens (net-new surface).
pub fn render_weidu_line(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    item: &Step3ItemState,
    line_number: Option<usize>,
    lineno_col_w: f32,
) {
    let text = format_step4_item(item);

    // Wireframe `OrderPanel` row: `display:flex; alignItems:baseline;
    // gap:10`. egui has no baseline-align; `Align::Min` (top) with a single
    // monospace size keeps the line-number and the line on the same visual
    // line (identical glyph metrics ⇒ same baseline in practice).
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        if let Some(n) = line_number {
            // Right-aligned, fixed-width, faint line-number column
            // (wireframe `OrderPanel`: `minWidth: lineNumWidth; textAlign:
            // right; color: var(--text-faint)`).
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

        // The three-colour WeiDU line itself, laid out as a single
        // `LayoutJob` so the segments flow inline (no inter-segment spacing,
        // exactly like the wireframe `WeiduLine`'s adjacent `<span>`s).
        let job = build_weidu_job(ui, palette, &text);
        ui.label(egui::WidgetText::from(job));
    });
}

/// Build the coloured `LayoutJob` for a single WeiDU-format `text` line.
///
/// Mirrors the **parse** of BIO's `content_step4::render_weidu_colored_line`
/// (`~...~` = path, the run up to `//` = component numbers, `// ...` =
/// comment), but assigns the **redesign** tokens per SPEC §6.7 / P6.T2b. A
/// line that doesn't contain a `~...~` path is treated as a pure comment
/// (BIO's same early-out).
fn build_weidu_job(ui: &egui::Ui, palette: ThemePalette, text: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let mono = egui::FontId::new(
        LINE_FONT_SIZE,
        egui::FontFamily::Name("firacode_nerd".into()),
    );
    // Suppress egui's default wrap so the (already `whiteSpace:nowrap`)
    // wireframe line never folds; the Box's ScrollArea provides horizontal
    // overflow.
    job.wrap.max_width = f32::INFINITY;
    let _ = ui;

    let path_color = redesign_accent_deep(palette); // <tp2_file>
    let nums_color = redesign_text_muted(palette); // <component_id>
    let comment_color = redesign_text_primary(palette); // <component_label>

    // Pure-comment line (no `~...~`) → whole line in the comment colour
    // (BIO `render_weidu_colored_line`'s `starts_with("//")` early-out, plus
    // the `else` fallback for any other path-less line).
    let trimmed_start = text.trim_start();
    if trimmed_start.starts_with("//") {
        append(&mut job, text, &mono, comment_color);
        return job;
    }

    // `~<path>~ <numbers> // <comment>` split — the exact byte-index walk
    // BIO's renderer performs (`text.find('~')` → next `~` → optional `//`).
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
        // No path delimiters at all — render flat in the comment colour
        // (BIO's final `else`).
        append(&mut job, text, &mono, comment_color);
    }

    job
}

/// Append a coloured run to the `LayoutJob` (skips empty runs — matches BIO's
/// `content_step4::append_text`).
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

    /// SPEC §8.1: the line-number column auto-sizes to the total count and
    /// uses no leading zeros. The width formula mirrors the wireframe
    /// (`String(n).length * 9 + 4`).
    #[test]
    fn lineno_column_width_scales_with_digit_count() {
        // 1..9 → 1 digit; 10..99 → 2; 100..999 → 3.
        assert_eq!(lineno_column_width(1), 1.0 * 9.0 + 4.0);
        assert_eq!(lineno_column_width(2), 2.0 * 9.0 + 4.0);
        assert_eq!(lineno_column_width(3), 3.0 * 9.0 + 4.0);
        assert!(lineno_column_width(3) > lineno_column_width(1));
    }

    /// The renderer's text content is BIO's canonical `format_step4_item`
    /// (BIO-fidelity): when `raw_line` is empty it synthesises the
    /// `~<folder>\<file>~ #0 #<id> // <label>` form.
    #[test]
    fn synthesised_line_matches_bio_format() {
        let it = item("", "EEFIXPACK.TP2", "EEFixPack", "2", "Game Text Update");
        let line = format_step4_item(&it);
        assert_eq!(line, "~EEFixPack\\EEFIXPACK.TP2~ #0 #2 // Game Text Update");
    }

    /// A non-empty `raw_line` is normalised (not synthesised) — the same
    /// BIO-fidelity contract `build_weidu_export_lines` relies on.
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
        // `normalize_weidu_like_line` collapses the path to `<folder>\<file>`.
        assert_eq!(
            line,
            "~EEFIXPACK\\EEFIXPACK.TP2~ #0 #5 // Drow Item Restorations"
        );
    }

    /// The colour split must produce exactly three runs (path / numbers /
    /// comment) for a canonical line, and the runs must use the **redesign**
    /// tokens (SPEC §6.7 / P6.T2b mapping), NOT BIO's legacy palette.
    #[test]
    fn three_colour_split_uses_redesign_tokens() {
        let palette = ThemePalette::Dark;
        // Build the job through a real (headless) egui context.
        let ctx = egui::Context::default();
        let mut produced: Vec<(String, egui::Color32)> = Vec::new();
        ctx.run(Default::default(), |ctx| {
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
        assert_eq!(produced[0].1, redesign_accent_deep(palette));
        assert!(produced[1].0.contains("#0 #2"));
        assert_eq!(produced[1].1, redesign_text_muted(palette));
        assert!(produced[2].0.contains("// Game Text Update"));
        assert_eq!(produced[2].1, redesign_text_primary(palette));
    }

    /// A pure-comment line renders as a single comment-coloured run (BIO's
    /// `starts_with("//")` early-out, mirrored).
    #[test]
    fn pure_comment_line_is_single_run() {
        let palette = ThemePalette::Dark;
        let ctx = egui::Context::default();
        let mut runs = 0usize;
        let mut color = egui::Color32::TRANSPARENT;
        ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let job = build_weidu_job(ui, palette, "// Log of Currently Installed WeiDU Mods");
                runs = job.sections.len();
                if let Some(s) = job.sections.first() {
                    color = s.format.color;
                }
            });
        });
        assert_eq!(runs, 1);
        assert_eq!(color, redesign_text_primary(palette));
    }
}
