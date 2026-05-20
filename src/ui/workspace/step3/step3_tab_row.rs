// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::app::step3_toolbar::Step3ToolbarSummary;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_pill_danger, redesign_pill_text, redesign_pill_warn,
};
use crate::ui::step2::prompt_popup_step2::open_toolbar_prompt_popup;
use crate::ui::step3::toolbar_support_step3;
use crate::ui::workspace::step4::workspace_step4;
use crate::ui::workspace::widgets::game_tab::game_tab;

const TAB_GAP: f32 = 4.0;
const ITEM_GAP: f32 = 8.0;
const ACTION_LEFT_PAD: f32 = 12.0;
const BTN_GAP: f32 = 6.0;

pub(crate) fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    palette: ThemePalette,
    summary: &Step3ToolbarSummary,
    rect: egui::Rect,
) {
    let row = Step3RowState::from_state(state, summary);
    let mut pending: Option<Step3RowIntent> = None;

    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        ui.horizontal(|ui| {
            render_tabs(ui, state, palette, row.dual_game);
            if render_conflict_pill(ui, state, palette, &row).is_some_and(|r| r.clicked()) {
                pending = Some(Step3RowIntent::OpenConflict);
            }
            if render_prompt_pill(ui, palette, row.prompt_count).is_some_and(|r| r.clicked()) {
                pending = Some(Step3RowIntent::OpenPrompt);
            }
            render_actions(ui, palette, &row, &mut pending);
        });
    });

    if let Some(intent) = pending {
        match intent {
            Step3RowIntent::OpenConflict => {
                if let Some(target) = row.conflict_target.as_ref() {
                    toolbar_support_step3::open_toolbar_issue_popup(state, target);
                }
            }
            Step3RowIntent::OpenPrompt => {
                open_toolbar_prompt_popup(
                    state,
                    &format!("Prompt Components ({})", state.step3.active_game_tab),
                );
            }
            Step3RowIntent::Undo => toolbar_support_step3::undo_active(state),
            Step3RowIntent::Redo => toolbar_support_step3::redo_active(state),
            Step3RowIntent::CollapseAll => toolbar_support_step3::collapse_all_active(state),
            Step3RowIntent::ExpandAll => toolbar_support_step3::expand_all_active(state),
        }
    }
}

struct Step3RowState {
    conflict_count: usize,
    conflict_target: Option<crate::app::step3_toolbar::Step3ToolbarIssueTarget>,
    prompt_count: usize,
    can_undo: bool,
    can_redo: bool,
    dual_game: bool,
}

impl Step3RowState {
    fn from_state(state: &WizardState, summary: &Step3ToolbarSummary) -> Self {
        let active_is_bgee = state.step3.active_game_tab == "BGEE";
        let (conflict_count, conflict_target) = if active_is_bgee {
            (summary.bgee_summary.0, summary.bgee_target.clone())
        } else {
            (summary.bg2ee_summary.0, summary.bg2ee_target.clone())
        };
        Self {
            conflict_count,
            conflict_target,
            prompt_count: if active_is_bgee {
                summary.bgee_prompt_count
            } else {
                summary.bg2ee_prompt_count
            },
            can_undo: if active_is_bgee {
                !state.step3.bgee_undo_stack.is_empty()
            } else {
                !state.step3.bg2ee_undo_stack.is_empty()
            },
            can_redo: if active_is_bgee {
                !state.step3.bgee_redo_stack.is_empty()
            } else {
                !state.step3.bg2ee_redo_stack.is_empty()
            },
            dual_game: workspace_step4::is_dual_game(state),
        }
    }
}

fn render_tabs(ui: &mut egui::Ui, state: &mut WizardState, palette: ThemePalette, dual_game: bool) {
    ui.spacing_mut().item_spacing.x = TAB_GAP;
    if dual_game {
        game_tab(ui, palette, "BGEE", &mut state.step3.active_game_tab);
        game_tab(ui, palette, "BG2EE", &mut state.step3.active_game_tab);
        ui.add_space(ACTION_LEFT_PAD - TAB_GAP);
    }
    ui.spacing_mut().item_spacing.x = ITEM_GAP;
}

fn render_conflict_pill(
    ui: &mut egui::Ui,
    state: &WizardState,
    palette: ThemePalette,
    row: &Step3RowState,
) -> Option<egui::Response> {
    (row.conflict_count > 0).then(|| {
        let word = if row.conflict_count == 1 {
            "conflict"
        } else {
            "conflicts"
        };
        let issue_word = if row.conflict_count == 1 {
            "issue"
        } else {
            "issues"
        };
        clickable_pill(
            ui,
            palette,
            &format!("{} {}", row.conflict_count, word),
            redesign_pill_danger(palette),
            &format!(
                "{} compatibility {} in the {} Step 3 tab.",
                row.conflict_count, issue_word, state.step3.active_game_tab
            ),
        )
    })
}

fn render_prompt_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    prompt_count: usize,
) -> Option<egui::Response> {
    (prompt_count > 0).then(|| {
        let word = if prompt_count == 1 {
            "prompt"
        } else {
            "prompts"
        };
        clickable_pill(
            ui,
            palette,
            &format!("{prompt_count} {word}"),
            redesign_pill_warn(palette),
            crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS,
        )
    })
}

fn render_actions(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    row: &Step3RowState,
    pending: &mut Option<Step3RowIntent>,
) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = BTN_GAP;
        if step3_button(
            ui,
            palette,
            "Expand All",
            true,
            crate::ui::shared::tooltip_global::STEP3_EXPAND_ALL,
        ) {
            *pending = Some(Step3RowIntent::ExpandAll);
        }
        if step3_button(
            ui,
            palette,
            "Collapse All",
            true,
            crate::ui::shared::tooltip_global::STEP3_COLLAPSE_ALL,
        ) {
            *pending = Some(Step3RowIntent::CollapseAll);
        }
        if step3_button(
            ui,
            palette,
            "Redo",
            row.can_redo,
            crate::ui::shared::tooltip_global::STEP3_REDO,
        ) {
            *pending = Some(Step3RowIntent::Redo);
        }
        if step3_button(
            ui,
            palette,
            "Undo",
            row.can_undo,
            crate::ui::shared::tooltip_global::STEP3_UNDO,
        ) {
            *pending = Some(Step3RowIntent::Undo);
        }
    });
}

fn step3_button(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    enabled: bool,
    tooltip: &str,
) -> bool {
    redesign_btn(
        ui,
        palette,
        label,
        BtnOpts {
            small: true,
            disabled: !enabled,
            ..Default::default()
        },
    )
    .on_hover_text(tooltip)
    .clicked()
        && enabled
}

#[derive(Clone, Copy)]
enum Step3RowIntent {
    OpenConflict,
    OpenPrompt,
    Undo,
    Redo,
    CollapseAll,
    ExpandAll,
}

fn clickable_pill(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    label: &str,
    fill: egui::Color32,
    tooltip: &str,
) -> egui::Response {
    let pad_x = 6.0;
    let pad_y = 1.0;
    let text_color = redesign_pill_text(palette);
    let font = egui::FontId::new(10.0, egui::FontFamily::Name("poppins_medium".into()));
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), font.clone(), text_color);
    let size = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, egui::CornerRadius::same(7), fill);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            text_color,
        );
    }
    response.on_hover_text(tooltip.to_owned())
}
