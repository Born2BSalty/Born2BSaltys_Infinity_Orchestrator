// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::{BtnOpts, InputOpts, redesign_btn, redesign_text_input};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_input_bg, redesign_text_faint, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::{step2_dev_scan, step2_rescan_reconcile};

const SEARCH_INPUT_H: f32 = 30.0;
const ROW_GAP: f32 = 10.0;
const SEARCH_INPUT_TEXT_PAD: i8 = 8;

const RESCAN_DISABLED_TIP: &str = "Available after install prep (Phase 7) \u{2014} \
     the mods folder is extracted per-install at prep time (SPEC \u{00A7}13.12a). \
     Use the dev scan in dev mode to exercise scanning before then.";

pub fn render(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    palette: ThemePalette,
    rect: egui::Rect,
    dev_mode: bool,
) -> Option<Step2Action> {
    let is_scanning = orchestrator.wizard_state.step2.is_scanning;
    let scratch_scan_enabled = scratch_scan_enabled(orchestrator);

    let mut action: Option<Step2Action> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ROW_GAP;

            let scan_btn_label = if is_scanning {
                "Cancel Scan"
            } else {
                "Rescan Mods Folder"
            };
            let scan_btn_w = small_btn_width(ui, scan_btn_label);
            let dev_label = "dev: scan a folder\u{2026}";
            let dev_w = if dev_mode {
                small_btn_width(ui, dev_label) + ROW_GAP
            } else {
                0.0
            };
            let search_w = (rect.width() - scan_btn_w - ROW_GAP - dev_w).max(80.0);

            let search_margin = egui::Margin::symmetric(SEARCH_INPUT_TEXT_PAD, 4);
            let _resp = redesign_text_input(
                ui,
                palette,
                InputOpts {
                    edit: egui::TextEdit::singleline(
                        &mut orchestrator.wizard_state.step2.search_query,
                    )
                    .hint_text("Search mods or components...")
                    .text_color(redesign_text_primary(palette))
                    .background_color(redesign_input_bg(palette))
                    .margin(search_margin)
                    .font(egui::FontId::new(
                        14.0,
                        egui::FontFamily::Name("poppins_medium".into()),
                    )),
                    margin: search_margin,
                    size: egui::vec2(search_w, SEARCH_INPUT_H),
                    border: None,
                },
            );

            if is_scanning {
                if redesign_btn(
                    ui,
                    palette,
                    scan_btn_label,
                    BtnOpts {
                        small: true,
                        ..Default::default()
                    },
                )
                .on_hover_text("Stop the running scan and return to idle.")
                .clicked()
                {
                    action = Some(Step2Action::CancelScan);
                }
            } else {
                let resp = redesign_btn(
                    ui,
                    palette,
                    scan_btn_label,
                    BtnOpts {
                        small: true,
                        disabled: !scratch_scan_enabled,
                        ..Default::default()
                    },
                );
                if scratch_scan_enabled {
                    if resp
                        .on_hover_text("Scan this modlist's prepared mods folder.")
                        .clicked()
                    {
                        step2_rescan_reconcile::snapshot_current_selection(orchestrator);
                        action = Some(Step2Action::StartScan);
                    }
                } else {
                    resp.on_hover_text(RESCAN_DISABLED_TIP);
                }
            }

            if dev_mode && dev_scan_button(ui, palette, dev_label).clicked() {
                step2_dev_scan::pick_folder_and_scan(orchestrator);
            }
        });
    });

    action
}

fn scratch_scan_enabled(orchestrator: &OrchestratorApp) -> bool {
    let id = orchestrator.workspace_view.modlist_id.trim();
    if id.is_empty()
        || orchestrator
            .wizard_state
            .step1
            .mods_folder
            .trim()
            .is_empty()
    {
        return false;
    }
    orchestrator
        .workspace_state
        .get(id)
        .and_then(|workspace| workspace.scratch_mods_folder.as_deref())
        .is_some_and(|folder| !folder.trim().is_empty())
}

fn small_btn_width(ui: &egui::Ui, label: &str) -> f32 {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font, egui::Color32::WHITE);
    10.0_f32.mul_add(2.0, galley.size().x)
}

fn dev_scan_button(ui: &mut egui::Ui, palette: ThemePalette, label: &str) -> egui::Response {
    let font = egui::FontId::new(12.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        font.clone(),
        redesign_text_faint(palette),
    );
    let pad_x = 10.0;
    let pad_y = 4.0;
    let size = egui::vec2(galley.size().x + pad_x * 2.0, SEARCH_INPUT_H);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8);
        painter.rect_filled(rect, radius, redesign_input_bg(palette));
        painter.rect_stroke(
            rect,
            radius,
            egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
            egui::StrokeKind::Inside,
        );
        let _ = pad_y;
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            redesign_text_faint(palette),
        );
    }
    response.on_hover_text(
        "Dev only — pick an existing folder and run BIO's scan against it. \
         Pre-Phase-7 there is no per-install extracted-mods folder to scan \
         (SPEC §13.12a).",
    )
}
