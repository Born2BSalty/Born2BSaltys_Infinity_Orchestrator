// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::dialogs::confirm_dialog::{self, ConfirmOutcome};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, WORKSPACE_CONTENT_TEXT_INSET, redesign_border_strong,
    redesign_text_faint, redesign_text_primary, redesign_warning_soft,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::step2::{step2_log_confirm, step2_search, step2_tab_row};

const DETAILS_PANE_W: f32 = 420.0;
const TITLE_H: f32 = 24.0;
const TITLE_GAP: f32 = 8.0;
const SEARCH_H: f32 = 30.0;
const SEARCH_GAP: f32 = 10.0;
const TAB_ROW_H: f32 = 30.0;
const TAB_TO_GRID_OVERLAP: f32 = 1.5;
const GRID_GAP: f32 = 12.0;
const FOOTER_H: f32 = 20.0;
const FOOTER_GAP: f32 = 8.0;
const LEFT_MIN_W: f32 = 420.0;
const RIGHT_MIN_W: f32 = 240.0;
const CONTENT_MIN_H: f32 = 160.0;

pub fn render(ui: &mut egui::Ui, orchestrator: &mut OrchestratorApp) -> Option<Step2Action> {
    let palette = orchestrator.theme_palette;
    let dev_mode = orchestrator.dev_mode;

    crate::ui::step2::state_step2::normalize_active_tab(&mut orchestrator.wizard_state);

    let rects = Step2LayoutRects::from_root(ui.available_rect_before_wrap());
    let mut action: Option<Step2Action> = None;

    render_title(ui, palette, rects.title);

    if let Some(a) = step2_search::render(ui, orchestrator, palette, rects.search, dev_mode) {
        action = Some(a);
    }

    if let Some(a) = step2_tab_row::render(ui, orchestrator, palette, rects.tab_row) {
        action = Some(a);
    }

    sync_details_open(orchestrator);

    let details_open = orchestrator.workspace_view.step2.details_open;
    let panes = Step2PaneRects::from_content(rects.content, details_open);

    clipped_pane(ui, panes.left, |ui| {
        crate::ui::step2::list_pane_step2::render_list_pane(
            ui,
            &mut orchestrator.wizard_state,
            &mut action,
            panes.left,
        );
    });

    if let Some(right_rect) = panes.right {
        clipped_pane(ui, right_rect, |ui| {
            crate::ui::step2::details_pane_step2::render_pane(
                ui,
                &mut orchestrator.wizard_state,
                &mut action,
                right_rect,
            );
        });
        paint_details_splitter(ui, palette, panes.left);
    }

    let ctx = ui.ctx().clone();
    render_popups(ui, orchestrator, &ctx, &mut action);
    if let Some(a) = render_weidu_log_confirm(orchestrator, &ctx) {
        action = Some(a);
    }

    crate::ui::step2::service_list_ops_step2::recompute_selection_counts(
        &mut orchestrator.wizard_state,
    );
    render_footer(ui, orchestrator, palette, rects.footer);

    action
}

#[derive(Clone, Copy)]
struct Step2LayoutRects {
    title: egui::Rect,
    search: egui::Rect,
    tab_row: egui::Rect,
    content: egui::Rect,
    footer: egui::Rect,
}

impl Step2LayoutRects {
    fn from_root(root: egui::Rect) -> Self {
        let x = root.left();
        let w = root.width();
        let mut y = root.top();

        let title = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TITLE_H));
        y += TITLE_H + TITLE_GAP;

        let search = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, SEARCH_H));
        y += SEARCH_H + SEARCH_GAP;

        let tab_row = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, TAB_ROW_H));
        y += TAB_ROW_H - TAB_TO_GRID_OVERLAP;

        let content_h = (root.bottom() - y - FOOTER_H - FOOTER_GAP).max(CONTENT_MIN_H);
        let content = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, content_h));
        let footer = egui::Rect::from_min_size(
            egui::pos2(x, content.bottom() + FOOTER_GAP),
            egui::vec2(w, FOOTER_H),
        );

        Self {
            title,
            search,
            tab_row,
            content,
            footer,
        }
    }
}

#[derive(Clone, Copy)]
struct Step2PaneRects {
    left: egui::Rect,
    right: Option<egui::Rect>,
}

impl Step2PaneRects {
    fn from_content(content: egui::Rect, details_open: bool) -> Self {
        if !details_open {
            return Self {
                left: content,
                right: None,
            };
        }
        let right_w = DETAILS_PANE_W.min((content.width() - LEFT_MIN_W).max(0.0));
        let right_w = right_w.max(RIGHT_MIN_W.min(content.width()));
        let left_w = (content.width() - right_w - GRID_GAP).max(LEFT_MIN_W);
        let left = egui::Rect::from_min_size(content.min, egui::vec2(left_w, content.height()));
        let right = egui::Rect::from_min_size(
            egui::pos2(left.right() + GRID_GAP, content.top()),
            egui::vec2(right_w, content.height()),
        );
        Self {
            left,
            right: Some(right),
        }
    }
}

fn render_title(
    ui: &mut egui::Ui,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    rect: egui::Rect,
) {
    let title_text_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(WORKSPACE_CONTENT_TEXT_INSET, 0.0),
        rect.max,
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_text_rect), |ui| {
        ui.label(
            egui::RichText::new("Mods / Components")
                .size(15.0)
                .family(egui::FontFamily::Name("poppins_medium".into()))
                .color(redesign_text_primary(palette)),
        );
    });
}

fn sync_details_open(orchestrator: &mut OrchestratorApp) {
    let live_sel = orchestrator.wizard_state.step2.selected.clone();
    if live_sel.is_some() && live_sel != orchestrator.workspace_view.step2.last_selection {
        orchestrator.workspace_view.step2.details_open = true;
    }
    orchestrator.workspace_view.step2.last_selection = live_sel;
}

fn paint_details_splitter(
    ui: &egui::Ui,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    left_rect: egui::Rect,
) {
    let dx = left_rect.right() - 1.0;
    ui.painter().line_segment(
        [
            egui::pos2(dx, left_rect.top() + 1.0),
            egui::pos2(dx, left_rect.bottom() - 1.0),
        ],
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_strong(palette)),
    );
}

fn render_popups(
    ui: &mut egui::Ui,
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
    action: &mut Option<Step2Action>,
) {
    crate::ui::step2::compat_window_step2::render(ui, &mut orchestrator.wizard_state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, &mut orchestrator.wizard_state);
    crate::ui::step2::update_check_popup_step2::render(ctx, &mut orchestrator.wizard_state, action);
}

fn render_footer(
    ui: &mut egui::Ui,
    orchestrator: &OrchestratorApp,
    palette: crate::ui::shared::redesign_tokens::ThemePalette,
    footer_rect: egui::Rect,
) {
    let drop_warning = orchestrator
        .workspace_view
        .step2
        .rescan_drop_warning
        .clone();
    ui.scope_builder(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&orchestrator.wizard_state.step2.scan_status)
                    .size(12.0)
                    .family(egui::FontFamily::Name("poppins_medium".into()))
                    .color(redesign_text_faint(palette)),
            );
            if let Some(warning) = drop_warning.as_deref() {
                ui.label(
                    egui::RichText::new(format!("\u{2014} {warning}"))
                        .size(12.0)
                        .family(egui::FontFamily::Name("poppins_medium".into()))
                        .color(redesign_warning_soft(palette)),
                );
            }
        });
    });
}

fn clipped_pane(ui: &mut egui::Ui, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let clip = rect.intersect(ui.clip_rect());
    child.set_clip_rect(clip);
    add(&mut child);
    ui.allocate_rect(rect, egui::Sense::hover());
}

fn render_weidu_log_confirm(
    orchestrator: &mut OrchestratorApp,
    ctx: &egui::Context,
) -> Option<Step2Action> {
    let bgee = orchestrator
        .workspace_view
        .step2
        .pending_weidu_log_confirm?;

    let (title, body) = step2_log_confirm::weidu_log_dialog_text(bgee);
    let dialog = step2_log_confirm::weidu_log_confirm(&title, &body);
    let outcome = confirm_dialog::render(ctx, orchestrator.theme_palette, &dialog);

    match outcome {
        ConfirmOutcome::Confirmed => {
            orchestrator.workspace_view.step2.pending_weidu_log_confirm = None;
            Some(if bgee {
                Step2Action::SelectBgeeViaLog
            } else {
                Step2Action::SelectBg2eeViaLog
            })
        }
        ConfirmOutcome::Cancelled => {
            orchestrator.workspace_view.step2.pending_weidu_log_confirm = None;
            None
        }
        ConfirmOutcome::Pending => None,
    }
}
