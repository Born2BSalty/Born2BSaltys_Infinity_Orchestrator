// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::typography_global as typo;
use crate::ui::step2::prompt_popup_step2::{draw_prompt_toolbar_badge, open_toolbar_prompt_popup};
use crate::ui::step3::list_step3;
use crate::ui::step3::{state_step3, toolbar_support_step3};

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str) {
    let is_active = active == value;
    let fill = if is_active {
        ui.visuals().widgets.active.bg_fill
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let stroke = if is_active {
        ui.visuals().widgets.active.bg_stroke
    } else {
        ui.visuals().widgets.inactive.bg_stroke
    };
    let text_color = if is_active {
        ui.visuals().widgets.active.fg_stroke.color
    } else {
        ui.visuals().widgets.inactive.fg_stroke.color
    };
    let button =
        egui::Button::new(crate::ui::shared::typography_global::plain(value).color(text_color))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(
                crate::ui::shared::layout_tokens_global::RADIUS_SM_U8,
            ));
    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}

fn draw_tab_issue_badge(
    ui: &mut egui::Ui,
    active: &mut String,
    value: &str,
    issue_count: usize,
    has_blocking: bool,
) -> bool {
    if issue_count == 0 {
        return false;
    }

    let (text_color, fill_color) = if has_blocking {
        (
            crate::ui::shared::theme_global::conflict(),
            crate::ui::shared::theme_global::conflict_fill(),
        )
    } else {
        (
            crate::ui::shared::theme_global::warning(),
            crate::ui::shared::theme_global::warning_fill(),
        )
    };

    let badge_text = crate::ui::shared::typography_global::strong(format!("{value} {issue_count}"))
        .color(text_color)
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let issue_label = if issue_count == 1 { "issue" } else { "issues" };
    let badge = egui::Button::new(badge_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(
            crate::ui::shared::layout_tokens_global::BORDER_THIN,
            fill_color,
        ))
        .corner_radius(egui::CornerRadius::same(7))
        .min_size(egui::vec2(0.0, 18.0));
    if ui
        .add(badge)
        .on_hover_text(format!(
            "{issue_count} compatibility {issue_label} in the {value} Step 3 tab."
        ))
        .clicked()
    {
        *active = value.to_string();
        return true;
    }
    false
}

fn render_toolbar(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
) {
    ui.horizontal(|ui| {
        render_toolbar_tabs(ui, state, summary);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            render_toolbar_actions(ui, state, dev_mode, exe_fingerprint);
        });
    });
}

fn render_toolbar_tabs(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
) {
    let (first_issue_count, first_has_blocking) = summary.bgee_summary;
    let (second_issue_count, second_has_blocking) = summary.bg2ee_summary;
    if summary.show_bgee && summary.show_bg2ee {
        draw_tab(ui, &mut state.step3.active_game_tab, "BGEE");
        draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE");
        ui.add_space(8.0);
        render_issue_badge(
            ui,
            state,
            "BGEE",
            first_issue_count,
            first_has_blocking,
            summary.bgee_target.as_ref(),
        );
        render_issue_badge(
            ui,
            state,
            "BG2EE",
            second_issue_count,
            second_has_blocking,
            summary.bg2ee_target.as_ref(),
        );
        render_dual_prompt_badge(ui, state, summary);
    } else if summary.show_bgee {
        render_single_tab_status(
            ui,
            state,
            "BGEE",
            first_issue_count,
            first_has_blocking,
            summary,
        );
    } else if summary.show_bg2ee {
        render_single_tab_status(
            ui,
            state,
            "BG2EE",
            second_issue_count,
            second_has_blocking,
            summary,
        );
    }
}

fn render_issue_badge(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    value: &str,
    issue_count: usize,
    has_blocking: bool,
    target: Option<&crate::app::step3_toolbar::Step3ToolbarIssueTarget>,
) {
    if draw_tab_issue_badge(
        ui,
        &mut state.step3.active_game_tab,
        value,
        issue_count,
        has_blocking,
    ) && let Some(target) = target
    {
        toolbar_support_step3::open_toolbar_issue_popup(state, target);
    }
}

fn render_dual_prompt_badge(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
) {
    let active_prompt_count = if state.step3.active_game_tab == "BGEE" {
        summary.bgee_prompt_count
    } else {
        summary.bg2ee_prompt_count
    };
    if draw_prompt_toolbar_badge(ui, active_prompt_count) {
        open_toolbar_prompt_popup(
            state,
            &format!("Prompt Components ({})", state.step3.active_game_tab),
        );
    }
}

fn render_single_tab_status(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    tab: &str,
    issue_count: usize,
    has_blocking: bool,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
) {
    ui.label(typo::monospace(tab));
    let (target, prompt_count) = if tab == "BGEE" {
        (summary.bgee_target.as_ref(), summary.bgee_prompt_count)
    } else {
        (summary.bg2ee_target.as_ref(), summary.bg2ee_prompt_count)
    };
    render_issue_badge(ui, state, tab, issue_count, has_blocking, target);
    if draw_prompt_toolbar_badge(ui, prompt_count) {
        open_toolbar_prompt_popup(state, &format!("Prompt Components ({tab})"));
    }
}

fn render_toolbar_actions(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let (can_undo, can_redo) = active_history_status(state);
    render_diagnostics_action(ui, state, dev_mode, exe_fingerprint);
    if ui
        .button("Expand All")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPAND_ALL)
        .clicked()
    {
        toolbar_support_step3::expand_all_active(state);
    }
    if ui
        .button("Collapse All")
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_COLLAPSE_ALL)
        .clicked()
    {
        toolbar_support_step3::collapse_all_active(state);
    }
    if ui
        .add_enabled(can_redo, egui::Button::new("Redo"))
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_REDO)
        .clicked()
    {
        toolbar_support_step3::redo_active(state);
    }
    if ui
        .add_enabled(can_undo, egui::Button::new("Undo"))
        .on_hover_text(crate::ui::shared::tooltip_global::STEP3_UNDO)
        .clicked()
    {
        toolbar_support_step3::undo_active(state);
    }
}

fn active_history_status(state: &WizardState) -> (bool, bool) {
    if state.step3.active_game_tab == "BGEE" {
        (
            !state.step3.bgee_undo_stack.is_empty(),
            !state.step3.bgee_redo_stack.is_empty(),
        )
    } else {
        (
            !state.step3.bg2ee_undo_stack.is_empty(),
            !state.step3.bg2ee_redo_stack.is_empty(),
        )
    }
}

fn render_diagnostics_action(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    if dev_mode
        && ui
            .button("Export diagnostics")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPORT_DIAGNOSTICS)
            .clicked()
    {
        toolbar_support_step3::export_diagnostics_from_step3(state, dev_mode, exe_fingerprint);
    }
}

pub fn render(ui: &mut egui::Ui, state: &mut WizardState, dev_mode: bool, exe_fingerprint: &str) {
    state_step3::normalize_active_tab(state);
    let toolbar_summary = toolbar_support_step3::build_toolbar_summary(state);
    let active_markers = if state.step3.active_game_tab == "BGEE" {
        &toolbar_summary.bgee_markers
    } else {
        &toolbar_summary.bg2ee_markers
    };
    state.step3.bgee_has_conflict = toolbar_summary.show_bgee
        && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bgee_markers);
    state.step3.bg2ee_has_conflict = toolbar_summary.show_bg2ee
        && toolbar_support_step3::tab_has_conflict(&toolbar_summary.bg2ee_markers);

    ui.heading("Step 3: Reorder and Resolve");
    ui.label("Review and adjust install order. Drag and drop components to reorder them.");
    ui.label(crate::ui::shared::typography_global::weak(
        "Right-click a component for more actions, including uncheck and prompt tools.",
    ));
    ui.add_space(8.0);

    render_toolbar(ui, state, dev_mode, exe_fingerprint, &toolbar_summary);
    if let Some(err) = toolbar_summary.compat_rules_error.as_deref() {
        ui.add_space(4.0);
        ui.label(
            crate::ui::shared::typography_global::weak(format!("Compat rules load failed: {err}"))
                .color(crate::ui::shared::theme_global::warning()),
        );
    }

    ui.add_space(6.0);
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    state.step3.jump_to_selected_requested = false;
    list_step3::render(ui, state, &mut jump_to_selected_requested, active_markers);
    crate::ui::step2::content_step2::render_compat_popup(ui, state);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state);
    state.step3.jump_to_selected_requested =
        state.step3.jump_to_selected_requested || jump_to_selected_requested;
}
