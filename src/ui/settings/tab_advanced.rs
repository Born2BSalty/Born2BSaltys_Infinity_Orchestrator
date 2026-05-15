// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `tab_advanced` — Advanced sub-tab renderer.
//
// Per Phase 4 P4.T6 (SPEC §11.5):
//   - Two equal-width columns (`ui.columns(2, ...)`) matching the wireframe's
//     `gridTemplateColumns: "1fr 1fr"`.
//   - Left column "Timing & limits": ValueRows with absorb-the-gate pattern.
//     An empty input means "use BIO default" — internally maps to
//     `<field>_enabled = false` + `<field> = default`. A filled input maps to
//     `enabled = true` + parsed value. We do **not** remove the boolean
//     `_enabled` fields from `Step1Settings` (CRITICAL DIRECTIVE).
//   - Right column "Install behavior" + "WeiDU command-line flags":
//     ToggleRows.
//
// Each row uses an end-capped layout (label left-aligned, optional hint mid,
// control flush-right) so longer hint copy never pushes the column wider —
// the right column's control always stops at the column's right edge.
//
// SPEC: §11.5.

// rationale: `f32 as u8` casts are pixel roundings of small positive
// constants — correct by construction (Cat 2); the tab render fn's argument
// count / length mirror the sub-tab's field set and a split would not aid
// readability (Cat 3).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::{
    redesign_accent, redesign_border_strong, redesign_chrome_bg, redesign_input_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary, ThemePalette,
    REDESIGN_BORDER_RADIUS_PX, REDESIGN_BORDER_WIDTH_PX,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    ui.columns(2, |cols| {
        // ---------------- Left column: Timing & limits ----------------
        {
            let ui = &mut cols[0];
            col_header(ui, palette, "Timing & limits");
            value_row_gated_usize(
                ui,
                palette,
                "Custom scan depth",
                None,
                &mut orchestrator.wizard_state.step1.depth,
                &mut orchestrator.wizard_state.step1.custom_scan_depth,
                5,
                "default 5",
            );
            value_row_gated_usize(
                ui,
                palette,
                "Mod install timeout",
                Some("sec"),
                &mut orchestrator.wizard_state.step1.timeout,
                &mut orchestrator.wizard_state.step1.timeout_per_mod_enabled,
                3600,
                "default 3600",
            );
            value_row_gated_usize(
                ui,
                palette,
                "Auto-answer initial delay",
                Some("ms"),
                &mut orchestrator.wizard_state.step1.auto_answer_initial_delay_ms,
                &mut orchestrator
                    .wizard_state
                    .step1
                    .auto_answer_initial_delay_enabled,
                2000,
                "default 2000",
            );
            value_row_gated_usize(
                ui,
                palette,
                "Auto-answer post-send delay",
                Some("ms"),
                &mut orchestrator
                    .wizard_state
                    .step1
                    .auto_answer_post_send_delay_ms,
                &mut orchestrator
                    .wizard_state
                    .step1
                    .auto_answer_post_send_delay_enabled,
                5000,
                "default 5000",
            );
            value_row_gated_u64(
                ui,
                palette,
                "Tick (dev)",
                Some("ms"),
                &mut orchestrator.wizard_state.step1.tick,
                &mut orchestrator.wizard_state.step1.tick_dev_enabled,
                500,
                "default 500",
            );
            value_row_gated_usize(
                ui,
                palette,
                "Prompt context lookback",
                None,
                &mut orchestrator.wizard_state.step1.lookback,
                &mut orchestrator.wizard_state.step1.lookback_enabled,
                10,
                "default 10",
            );
        }

        // ---------------- Right column: Install behavior + WeiDU flags ----
        {
            let ui = &mut cols[1];
            col_header(ui, palette, "Install behavior");
            toggle_row(
                ui,
                palette,
                "Prompt sound cue",
                Some("beep when a prompt needs you"),
                &mut orchestrator
                    .wizard_state
                    .step1
                    .prompt_required_sound_enabled,
            );
            toggle_row(
                ui,
                palette,
                "Download missing mods",
                Some("fetch GitHub/Weasel/Morpheus during install"),
                &mut orchestrator.wizard_state.step1.download,
            );
            toggle_row(
                ui,
                palette,
                "Casefold filename matching",
                Some("ASCII case-insensitive lookups"),
                &mut orchestrator.wizard_state.step1.casefold,
            );

            ui.add_space(10.0);
            col_header(ui, palette, "WeiDU command-line flags");
            toggle_row(
                ui,
                palette,
                "-a  abort on warnings",
                None,
                &mut orchestrator.wizard_state.step1.abort_on_warnings,
            );
            toggle_row(
                ui,
                palette,
                "-x  strict matching",
                None,
                &mut orchestrator.wizard_state.step1.strict_matching,
            );
            toggle_row(
                ui,
                palette,
                "-o  overwrite",
                None,
                &mut orchestrator.wizard_state.step1.overwrite,
            );
        }
    });
}

fn col_header(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
}

/// End-capped row: label left-aligned, optional hint mid, control flush-right.
/// The control is painted by `add_control` inside a `right_to_left` ui so it
/// stops at the column's right edge regardless of label / hint length.
fn end_capped_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: Option<&str>,
    add_control: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.set_min_height(26.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            add_control(ui);
            if let Some(h) = hint {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(h)
                        .size(11.0)
                        .family(egui::FontFamily::Proportional)
                        .color(redesign_text_faint(palette)),
                );
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_text_primary(palette)),
                );
            });
        });
    });
    ui.add_space(2.0);
}

fn paint_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    buf: &mut String,
    placeholder: &str,
) -> bool {
    let pre = buf.clone();
    let edit = egui::TextEdit::singleline(buf)
        .font(egui::FontId::new(
            12.0,
            egui::FontFamily::Name("firacode_nerd".into()),
        ))
        .text_color(redesign_text_primary(palette))
        .background_color(redesign_input_bg(palette))
        .margin(egui::Margin::symmetric(6, 4))
        .horizontal_align(egui::Align::RIGHT)
        .hint_text(
            egui::RichText::new(placeholder)
                .size(12.0)
                .family(egui::FontFamily::Name("firacode_nerd".into()))
                .color(redesign_text_faint(palette)),
        );
    let response = ui.add_sized(egui::vec2(110.0, 22.0), edit);
    ui.painter().rect_stroke(
        response.rect,
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_PX as u8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Outside,
    );
    response.changed() || *buf != pre
}

fn paint_toggle(ui: &mut egui::Ui, palette: ThemePalette, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, 22.0), egui::Sense::click());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same((REDESIGN_BORDER_RADIUS_PX + 8.0) as u8);
    let track_fill = if *on {
        redesign_accent(palette)
    } else {
        redesign_chrome_bg(palette)
    };
    painter.rect_filled(rect, radius, track_fill);
    painter.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Inside,
    );
    let knob_size = 16.0;
    let knob_center = if *on {
        egui::pos2(rect.right() - knob_size * 0.5 - 4.0, rect.center().y)
    } else {
        egui::pos2(rect.left() + knob_size * 0.5 + 4.0, rect.center().y)
    };
    painter.circle_filled(
        knob_center,
        knob_size * 0.5,
        egui::Color32::from_rgb(0xE6, 0xED, 0xF3),
    );
    painter.circle_stroke(
        knob_center,
        knob_size * 0.5,
        egui::Stroke::new(1.0, redesign_border_strong(palette)),
    );
    if response.clicked() {
        *on = !*on;
        true
    } else {
        false
    }
}

/// Absorb-the-gate row for a `usize` field with an `enabled: bool` companion.
fn value_row_gated_usize(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: Option<&str>,
    value: &mut usize,
    enabled: &mut bool,
    default_value: usize,
    placeholder: &str,
) {
    let mut buf = if *enabled {
        value.to_string()
    } else {
        String::new()
    };
    end_capped_row(ui, palette, label, hint, |ui| {
        let _ = paint_input(ui, palette, &mut buf, placeholder);
    });
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        *enabled = false;
        *value = default_value;
    } else if let Ok(parsed) = trimmed.parse::<usize>() {
        *enabled = true;
        *value = parsed;
    }
}

/// Absorb-the-gate row for a `u64` field with an `enabled: bool` companion.
fn value_row_gated_u64(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: Option<&str>,
    value: &mut u64,
    enabled: &mut bool,
    default_value: u64,
    placeholder: &str,
) {
    let mut buf = if *enabled {
        value.to_string()
    } else {
        String::new()
    };
    end_capped_row(ui, palette, label, hint, |ui| {
        let _ = paint_input(ui, palette, &mut buf, placeholder);
    });
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        *enabled = false;
        *value = default_value;
    } else if let Ok(parsed) = trimmed.parse::<u64>() {
        *enabled = true;
        *value = parsed;
    }
}

fn toggle_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    hint: Option<&str>,
    on: &mut bool,
) {
    end_capped_row(ui, palette, label, hint, |ui| {
        let _ = paint_toggle(ui, palette, on);
    });
}
