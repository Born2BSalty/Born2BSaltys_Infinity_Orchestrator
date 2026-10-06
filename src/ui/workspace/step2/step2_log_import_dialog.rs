// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::cell::Cell;
use std::path::Path;

use eframe::egui;

use crate::ui::install::destination_not_empty::{
    WARN_BORDER, WARN_INK, paint_warning_triangle, warn_fill,
};
use crate::ui::orchestrator::widgets::drawer::{self, DrawerSpec, DrawerWidth};
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_btn_height, toggle_switch};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_text_muted,
    redesign_text_primary,
};
use crate::ui::workspace::state_workspace::WeiduLogImportForm;
use crate::ui::workspace::step2::step2_log_confirm::{
    WeiduLogImportRow, weidu_log_import_copy, weidu_log_import_rows,
};
use crate::ui::workspace::step2_log_glue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportOutcome {
    #[default]
    Pending,
    Import,
    Cancelled,
}

const ID_SALT: &str = "weidu_log_import_drawer";
const TITLE: &str = "Import from WeiDU logs";
const ROW_LABEL_W: f32 = 120.0;
const ROW_ITEM_GAP: f32 = 8.0;
const ROW_GAP: f32 = 8.0;
const CHOOSE_LABEL: &str = "Choose\u{2026}";
const EMPTY_PATH_TEXT: &str = "Click Choose\u{2026} to pick the log";
const IMPORT_LABEL: &str = "Import";
const CANCEL_LABEL: &str = "Cancel";
const WARNING_TITLE: &str = "Selections will be replaced";
const FETCH_LABEL: &str = "Fetch missing mods";

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
    let (subtitle, warning) = weidu_log_import_copy(game_install);
    let rows = weidu_log_import_rows(game_install);
    let spec = DrawerSpec {
        id_salt: ID_SALT,
        title: TITLE,
        subtitle: &subtitle,
        width: DrawerWidth::Form,
        header_button: None,
        suppress_escape: false,
    };
    let enabled = Cell::new(import_enabled(form));

    let response = drawer::render(
        ctx,
        palette,
        &spec,
        |ui| {
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
            enabled.set(import_enabled(form));
            ui.add_space(ROW_GAP);
            render_fetch_row(ui, palette, &mut form.fetch_missing);
            ui.add_space(16.0);
            render_warning_box(ui, &warning);
        },
        |ui| render_footer(ui, palette, enabled.get()),
    );

    if response.footer == ImportOutcome::Import {
        ImportOutcome::Import
    } else if response.footer == ImportOutcome::Cancelled || response.close_requested {
        ImportOutcome::Cancelled
    } else {
        ImportOutcome::Pending
    }
}

fn render_warning_box(ui: &mut egui::Ui, warning: &str) {
    egui::Frame::default()
        .fill(warn_fill())
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, WARN_BORDER))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 10,
            bottom: 10,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let (icon_rect, _) =
                    ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
                paint_warning_triangle(ui.painter(), icon_rect.center(), WARN_INK);
                ui.label(
                    egui::RichText::new(WARNING_TITLE)
                        .size(13.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(WARN_INK),
                );
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(warning)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_light".into()))
                    .color(egui::Color32::from_rgba_unmultiplied(
                        0xff, 0xff, 0xff, 0xCC,
                    )),
            );
        });
}

pub(crate) fn render_fetch_row(ui: &mut egui::Ui, palette: ThemePalette, fetch_missing: &mut bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = ROW_ITEM_GAP;
        let _ = toggle_switch(ui, palette, fetch_missing);
        ui.label(
            egui::RichText::new(FETCH_LABEL)
                .size(13.0)
                .family(egui::FontFamily::Name("poppins_light".into()))
                .color(redesign_text_primary(palette)),
        );
    });
}

pub(crate) fn render_row(
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
    let cancel_clicked = redesign_btn(
        ui,
        palette,
        CANCEL_LABEL,
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
    .clicked();
    let import_clicked = ui
        .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            redesign_btn(
                ui,
                palette,
                IMPORT_LABEL,
                BtnOpts {
                    primary: true,
                    small: true,
                    disabled: !enabled,
                    ..Default::default()
                },
            )
            .clicked()
        })
        .inner
        && enabled;
    if import_clicked {
        ImportOutcome::Import
    } else if cancel_clicked {
        ImportOutcome::Cancelled
    } else {
        ImportOutcome::Pending
    }
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
            ..WeiduLogImportForm::default()
        }));
        assert!(import_enabled(&WeiduLogImportForm {
            second: Some(PathBuf::from("bg2ee.log")),
            ..WeiduLogImportForm::default()
        }));
        assert!(import_enabled(&WeiduLogImportForm {
            first: Some(PathBuf::from("bgee.log")),
            second: Some(PathBuf::from("bg2ee.log")),
            fetch_missing: false,
        }));
    }
}
