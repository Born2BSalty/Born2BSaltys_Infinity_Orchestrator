// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use eframe::egui;

use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_height};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg, redesign_text_muted, redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step2::step2_log_confirm::{
    WeiduLogImportRow, weidu_log_import_rows, weidu_log_import_text,
};
use crate::ui::workspace::step2_log_glue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportOutcome {
    #[default]
    Pending,
    Import,
    Cancelled,
}

const ID_SALT: &str = "step2_import_weidu_logs";
const MAX_WIDTH_PX: f32 = 560.0;
const ROW_LABEL_W: f32 = 120.0;
const ROW_ITEM_GAP: f32 = 8.0;
const ROW_GAP: f32 = 8.0;
const FOOTER_H: f32 = 30.0;
const CHOOSE_LABEL: &str = "Choose\u{2026}";
const EMPTY_PATH_TEXT: &str = "Click Choose\u{2026} to pick the log";
const IMPORT_LABEL: &str = "Import";
const CANCEL_LABEL: &str = "Cancel";

#[must_use]
pub const fn import_enabled(form: &WeiduLogImportForm) -> bool {
    form.first.is_some() || form.second.is_some()
}

#[must_use]
pub fn render(
    ctx: &egui::Context,
    palette: ThemePalette,
    game_install: &str,
    form: &mut WeiduLogImportForm,
    start_paths: &WeiduLogImportForm,
) -> ImportOutcome {
    let mut outcome = ImportOutcome::Pending;
    let (title, body) = weidu_log_import_text(game_install);
    let rows = weidu_log_import_rows(game_install);

    let frame = egui::Frame::default()
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(18));

    egui::Window::new(title.as_str())
        .id(egui::Id::new(ID_SALT))
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_max_width(MAX_WIDTH_PX);

            ui.label(
                egui::RichText::new(title.as_str())
                    .size(15.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_primary(palette)),
            );
            ui.add_space(8.0);

            ui.label(
                egui::RichText::new(body.as_str())
                    .size(13.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(redesign_text_muted(palette)),
            );
            ui.add_space(12.0);

            for (index, row) in rows.iter().enumerate() {
                if index > 0 {
                    ui.add_space(ROW_GAP);
                }
                let start = if row.first_slot {
                    start_paths.first.as_deref()
                } else {
                    start_paths.second.as_deref()
                };
                render_row(ui, palette, *row, form, start);
            }
            ui.add_space(16.0);

            outcome = render_footer(ui, palette, import_enabled(form));
        });

    outcome
}

fn render_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: WeiduLogImportRow,
    form: &mut WeiduLogImportForm,
    start: Option<&Path>,
) {
    let slot = if row.first_slot {
        &mut form.first
    } else {
        &mut form.second
    };
    let row_h = redesign_btn_height(ui, true);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ROW_ITEM_GAP;
        paint_row_label(ui, palette, row.tab, row_h);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), row_h),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = ROW_ITEM_GAP;
                let choose = redesign_btn(
                    ui,
                    palette,
                    CHOOSE_LABEL,
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                );
                paint_row_path(ui, palette, slot.as_deref(), row_h);
                if choose.clicked()
                    && let Some(picked) =
                        step2_log_glue::pick_weidu_log_file(slot.as_deref().or(start), row.tab)
                {
                    *slot = Some(picked);
                }
            },
        );
    });
}

fn paint_row_label(ui: &mut egui::Ui, palette: ThemePalette, tab: &str, row_h: f32) {
    let galley = ui.painter().layout_no_wrap(
        format!("{tab} weidu.log"),
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_medium".into())),
        redesign_text_primary(palette),
    );
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ROW_LABEL_W, row_h), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .galley(pos, galley, redesign_text_primary(palette));
    }
}

fn paint_row_path(ui: &mut egui::Ui, palette: ThemePalette, path: Option<&Path>, row_h: f32) {
    let (text, color) = path.map_or_else(
        || (EMPTY_PATH_TEXT.to_string(), redesign_text_muted(palette)),
        |path| (path.display().to_string(), redesign_text_primary(palette)),
    );
    let width = ui.available_width().max(0.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_h), egui::Sense::hover());
    let mut job = egui::text::LayoutJob::simple_singleline(
        text,
        egui::FontId::new(13.0, egui::FontFamily::Name("poppins_light".into())),
        color,
    );
    job.wrap =
        egui::text::TextWrapping::from_wrap_mode_and_width(egui::TextWrapMode::Truncate, width);
    let galley = ui.painter().layout_job(job);
    if ui.is_rect_visible(rect) {
        let pos = egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, color);
    }
}

fn render_footer(ui: &mut egui::Ui, palette: ThemePalette, enabled: bool) -> ImportOutcome {
    let mut outcome = ImportOutcome::Pending;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), FOOTER_H),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if render_import_button(ui, palette, enabled) {
                outcome = ImportOutcome::Import;
            }
            if redesign_btn(
                ui,
                palette,
                CANCEL_LABEL,
                BtnOpts {
                    small: true,
                    ..Default::default()
                },
            )
            .clicked()
            {
                outcome = ImportOutcome::Cancelled;
            }
        },
    );
    outcome
}

fn render_import_button(ui: &mut egui::Ui, palette: ThemePalette, enabled: bool) -> bool {
    redesign_btn(
        ui,
        palette,
        IMPORT_LABEL,
        BtnOpts {
            small: true,
            danger: true,
            disabled: !enabled,
            ..Default::default()
        },
    )
    .clicked()
        && enabled
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn import_needs_at_least_one_log() {
        assert!(!import_enabled(&WeiduLogImportForm::default()));
        assert!(import_enabled(&WeiduLogImportForm {
            first: Some(PathBuf::from("bgee.log")),
            second: None,
        }));
        assert!(import_enabled(&WeiduLogImportForm {
            first: None,
            second: Some(PathBuf::from("bg2ee.log")),
        }));
        assert!(import_enabled(&WeiduLogImportForm {
            first: Some(PathBuf::from("bgee.log")),
            second: Some(PathBuf::from("bg2ee.log")),
        }));
    }
}
