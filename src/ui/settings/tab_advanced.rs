// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, REDESIGN_PANEL_RADIUS_U8, ThemePalette,
    redesign_accent, redesign_border_strong, redesign_chrome_bg, redesign_input_bg,
    redesign_text_faint, redesign_text_muted, redesign_text_primary,
};

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) {
    let palette = orchestrator.theme_palette;

    ui.columns(2, |cols| {
        render_timing_limits(&mut cols[0], palette, orchestrator);
        render_install_behavior(&mut cols[1], palette, orchestrator);
    });
}

fn render_timing_limits(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
    col_header(ui, palette, "Timing & limits");
    value_row_gated_usize(
        ui,
        palette,
        GatedUsizeRow {
            label: "Custom scan depth",
            hint: None,
            value: &mut orchestrator.wizard_state.step1.depth,
            enabled: &mut orchestrator.wizard_state.step1.custom_scan_depth,
            default_value: 5,
            placeholder: "default 5",
        },
    );
    value_row_gated_usize(
        ui,
        palette,
        GatedUsizeRow {
            label: "Mod install timeout",
            hint: Some("sec"),
            value: &mut orchestrator.wizard_state.step1.timeout,
            enabled: &mut orchestrator.wizard_state.step1.timeout_per_mod_enabled,
            default_value: 3600,
            placeholder: "default 3600",
        },
    );
    value_row_gated_usize(
        ui,
        palette,
        GatedUsizeRow {
            label: "Auto-answer initial delay",
            hint: Some("ms"),
            value: &mut orchestrator.wizard_state.step1.auto_answer_initial_delay_ms,
            enabled: &mut orchestrator
                .wizard_state
                .step1
                .auto_answer_initial_delay_enabled,
            default_value: 2000,
            placeholder: "default 2000",
        },
    );
    value_row_gated_usize(
        ui,
        palette,
        GatedUsizeRow {
            label: "Auto-answer post-send delay",
            hint: Some("ms"),
            value: &mut orchestrator
                .wizard_state
                .step1
                .auto_answer_post_send_delay_ms,
            enabled: &mut orchestrator
                .wizard_state
                .step1
                .auto_answer_post_send_delay_enabled,
            default_value: 5000,
            placeholder: "default 5000",
        },
    );
    value_row_gated_u64(
        ui,
        palette,
        GatedU64Row {
            label: "Tick (dev)",
            hint: Some("ms"),
            value: &mut orchestrator.wizard_state.step1.tick,
            enabled: &mut orchestrator.wizard_state.step1.tick_dev_enabled,
            default_value: 500,
            placeholder: "default 500",
        },
    );
    value_row_gated_usize(
        ui,
        palette,
        GatedUsizeRow {
            label: "Prompt context lookback",
            hint: None,
            value: &mut orchestrator.wizard_state.step1.lookback,
            enabled: &mut orchestrator.wizard_state.step1.lookback_enabled,
            default_value: 10,
            placeholder: "default 10",
        },
    );
}

fn render_install_behavior(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    orchestrator: &mut OrchestratorApp,
) {
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
        Some("--download during install"),
        &mut orchestrator.wizard_state.step1.download,
    );
    let mut keep_archives = orchestrator.wizard_state.step1.download_archive;
    toggle_row(
        ui,
        palette,
        "Keep downloaded mod archives",
        Some("download missing mods and store archives"),
        &mut keep_archives,
    );
    orchestrator.wizard_state.step1.download_archive = keep_archives;
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
        "-s  skip installed",
        Some("skip entries already present in WeiDU logs"),
        &mut orchestrator.wizard_state.step1.skip_installed,
    );
    toggle_row(
        ui,
        palette,
        "-c  check last installed",
        Some("validate the last installed component"),
        &mut orchestrator.wizard_state.step1.check_last_installed,
    );
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

fn col_header(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    ui.label(
        egui::RichText::new(label)
            .size(12.0)
            .family(egui::FontFamily::Name("poppins_medium".into()))
            .color(redesign_text_muted(palette)),
    );
    ui.add_space(4.0);
}

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
        egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8),
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
        egui::StrokeKind::Outside,
    );
    response.changed() || *buf != pre
}

fn paint_toggle(ui: &mut egui::Ui, palette: ThemePalette, on: &mut bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, 22.0), egui::Sense::click());
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(REDESIGN_PANEL_RADIUS_U8);
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

struct GatedUsizeRow<'a> {
    label: &'a str,
    hint: Option<&'a str>,
    value: &'a mut usize,
    enabled: &'a mut bool,
    default_value: usize,
    placeholder: &'a str,
}

struct GatedU64Row<'a> {
    label: &'a str,
    hint: Option<&'a str>,
    value: &'a mut u64,
    enabled: &'a mut bool,
    default_value: u64,
    placeholder: &'a str,
}

fn value_row_gated_usize(ui: &mut egui::Ui, palette: ThemePalette, row: GatedUsizeRow<'_>) {
    let GatedUsizeRow {
        label,
        hint,
        value,
        enabled,
        default_value,
        placeholder,
    } = row;
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

fn value_row_gated_u64(ui: &mut egui::Ui, palette: ThemePalette, row: GatedU64Row<'_>) {
    let GatedU64Row {
        label,
        hint,
        value,
        enabled,
        default_value,
        placeholder,
    } = row;
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
