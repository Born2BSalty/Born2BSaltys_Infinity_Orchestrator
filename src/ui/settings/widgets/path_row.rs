// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::widgets::{InputOpts, redesign_text_input};
use crate::ui::settings::state_settings::PathStatusTone;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent_deep, redesign_border_strong,
    redesign_input_bg, redesign_pill_danger, redesign_success_soft, redesign_text_faint,
    redesign_text_primary, redesign_warning_soft,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathRowMode {
    Folder,
    File,
}

const LABEL_WIDTH_PX: f32 = 160.0;
const BROWSE_W_PX: f32 = 80.0;
const ROW_GAP_PX: f32 = 6.0;
const WARNING_LINE_HEIGHT_PX: f32 = 14.0;
const ROW_VERTICAL_GAP_PX: f32 = 6.0;

pub struct PathRow<'a> {
    pub label: &'a str,
    pub mono_value: &'a mut String,
    pub hint: Option<&'a str>,
    pub tone: PathStatusTone,
    pub mode: PathRowMode,
}

pub fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: PathRow<'_>,
    mut on_change: impl FnMut(),
) {
    let PathRow {
        label,
        mono_value,
        hint,
        tone,
        mode,
    } = row;

    ui.vertical(|ui| {
        let input_x_range =
            render_row_controls(ui, palette, label, mono_value, tone, mode, &mut on_change);
        render_hint(ui, palette, hint, tone, input_x_range);
    });
    ui.add_space(ROW_VERTICAL_GAP_PX);
}

fn render_row_controls(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    mono_value: &mut String,
    tone: PathStatusTone,
    mode: PathRowMode,
    on_change: &mut impl FnMut(),
) -> Option<(f32, f32)> {
    let mut input_x_range = None;
    ui.horizontal(|ui| {
        render_label(ui, palette, label);
        let edit_width = (ui.available_width() - BROWSE_W_PX - ROW_GAP_PX).max(120.0);
        input_x_range = Some(render_path_input(
            ui, palette, mono_value, tone, edit_width, on_change,
        ));
        render_browse_button(ui, palette, mono_value, mode, on_change);
    });
    input_x_range
}

fn render_label(ui: &mut egui::Ui, palette: ThemePalette, label: &str) {
    let (label_rect, _) =
        ui.allocate_exact_size(egui::vec2(LABEL_WIDTH_PX, 26.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(label_rect.left(), label_rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
        redesign_text_primary(palette),
    );
}

fn render_path_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    mono_value: &mut String,
    tone: PathStatusTone,
    edit_width: f32,
    on_change: &mut impl FnMut(),
) -> (f32, f32) {
    let border_color = match tone {
        PathStatusTone::Neutral => redesign_border_strong(palette),
        PathStatusTone::Success => redesign_success_soft(palette),
        PathStatusTone::Warning => redesign_warning_soft(palette),
        PathStatusTone::Error => redesign_pill_danger(palette),
    };
    let pre = mono_value.clone();
    let margin = egui::Margin::symmetric(6, 4);
    let response = redesign_text_input(
        ui,
        palette,
        InputOpts {
            edit: egui::TextEdit::singleline(mono_value)
                .font(egui::FontId::new(
                    12.0,
                    egui::FontFamily::Name("firacode_nerd".into()),
                ))
                .text_color(redesign_text_primary(palette))
                .background_color(redesign_input_bg(palette))
                .margin(margin),
            margin,
            size: egui::vec2(edit_width, 24.0),
            border: Some(border_color),
        },
    );
    if response.changed() || *mono_value != pre {
        on_change();
    }
    (response.rect.left(), response.rect.right())
}

fn render_browse_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    mono_value: &mut String,
    mode: PathRowMode,
    on_change: &mut impl FnMut(),
) {
    if !ui
        .add(
            egui::Button::new(
                egui::RichText::new("browse\u{2026}")
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_accent_deep(palette)),
            )
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::new(
                REDESIGN_BORDER_WIDTH_PX,
                redesign_border_strong(palette),
            )),
        )
        .clicked()
    {
        return;
    }
    let picked = match mode {
        PathRowMode::Folder => rfd::FileDialog::new().pick_folder(),
        PathRowMode::File => rfd::FileDialog::new().pick_file(),
    };
    if let Some(path) = picked {
        let s = path.to_string_lossy().to_string();
        if s != *mono_value {
            *mono_value = s;
            on_change();
        }
    }
}

fn render_hint(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    hint: Option<&str>,
    tone: PathStatusTone,
    input_x_range: Option<(f32, f32)>,
) {
    let (warning_rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), WARNING_LINE_HEIGHT_PX),
        egui::Sense::hover(),
    );
    if let (Some(hint_text), Some((x0, x1))) = (hint, input_x_range)
        && !hint_text.is_empty()
    {
        let clip = egui::Rect::from_min_max(
            egui::pos2(x0, warning_rect.top()),
            egui::pos2(x1, warning_rect.bottom()),
        );
        ui.painter().with_clip_rect(clip).text(
            egui::pos2(x0 + 2.0, warning_rect.center().y),
            egui::Align2::LEFT_CENTER,
            hint_text,
            egui::FontId::new(11.0, egui::FontFamily::Proportional),
            hint_color(palette, tone),
        );
    }
}

const fn hint_color(palette: ThemePalette, tone: PathStatusTone) -> egui::Color32 {
    match tone {
        PathStatusTone::Neutral => redesign_text_faint(palette),
        PathStatusTone::Success => redesign_success_soft(palette),
        PathStatusTone::Warning => redesign_warning_soft(palette),
        PathStatusTone::Error => redesign_pill_danger(palette),
    }
}
