// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BIO_PILL_HEIGHT_PX, REDESIGN_BIO_PILL_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX,
    ThemePalette, redesign_accent, redesign_border_soft, redesign_compat_conflict,
    redesign_compat_conflict_fill, redesign_compat_warning, redesign_compat_warning_fill,
    redesign_shell_bg, redesign_text_on_accent, redesign_text_primary, redesign_warning,
};
use crate::ui::shared::typography_global as typo;
use crate::ui::step2::prompt_popup_step2::{draw_prompt_toolbar_badge, open_toolbar_prompt_popup};
use crate::ui::step3::list_step3;
use crate::ui::step3::{state_step3, toolbar_support_step3};

pub fn draw_tab(ui: &mut egui::Ui, active: &mut String, value: &str, palette: ThemePalette) {
    let is_active = active == value;
    let fill = if is_active {
        redesign_accent(palette)
    } else {
        redesign_shell_bg(palette)
    };
    let stroke = if is_active {
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_accent(palette))
    } else {
        egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, redesign_border_soft(palette))
    };
    let text_color = if is_active {
        redesign_text_on_accent(palette)
    } else {
        redesign_text_primary(palette)
    };
    let button =
        egui::Button::new(crate::ui::shared::typography_global::plain(value).color(text_color))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(
                crate::ui::shared::layout_tokens_global::radius_u8(
                    crate::ui::shared::layout_tokens_global::RADIUS_SM,
                ),
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
    palette: ThemePalette,
) -> bool {
    if issue_count == 0 {
        return false;
    }

    let (text_color, fill_color) = if has_blocking {
        (
            redesign_compat_conflict(palette),
            redesign_compat_conflict_fill(palette),
        )
    } else {
        (
            redesign_compat_warning(palette),
            redesign_compat_warning_fill(palette),
        )
    };

    let badge_text = crate::ui::shared::typography_global::strong(format!("{value} {issue_count}"))
        .color(text_color)
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    let issue_label = if issue_count == 1 { "issue" } else { "issues" };
    let badge = egui::Button::new(badge_text)
        .fill(fill_color)
        .stroke(egui::Stroke::new(REDESIGN_BORDER_WIDTH_PX, fill_color))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BIO_PILL_RADIUS_U8))
        .min_size(egui::vec2(0.0, REDESIGN_BIO_PILL_HEIGHT_PX));
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
    palette: ThemePalette,
) {
    ui.horizontal(|ui| {
        render_tab_controls(ui, state, summary, palette);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            render_toolbar_actions(ui, state, dev_mode, exe_fingerprint);
        });
    });
}

fn render_tab_controls(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
    palette: ThemePalette,
) {
    let (primary_issue_count, primary_has_blocking) = summary.bgee_summary;
    let (secondary_issue_count, secondary_has_blocking) = summary.bg2ee_summary;
    if summary.show_bgee && summary.show_bg2ee {
        draw_tab(ui, &mut state.step3.active_game_tab, "BGEE", palette);
        draw_tab(ui, &mut state.step3.active_game_tab, "BG2EE", palette);
        ui.add_space(8.0);
        render_issue_badges(ui, state, summary, palette);
        let active_prompt_count = if state.step3.active_game_tab == "BGEE" {
            summary.bgee_prompt_count
        } else {
            summary.bg2ee_prompt_count
        };
        if draw_prompt_toolbar_badge(ui, active_prompt_count, palette) {
            open_toolbar_prompt_popup(
                state,
                &format!("Prompt Components ({})", state.step3.active_game_tab),
            );
        }
    } else if summary.show_bgee {
        ui.label(typo::monospace("BGEE"));
        render_single_tab_badges(
            ui,
            state,
            &SingleTabBadges {
                value: "BGEE",
                issue_count: primary_issue_count,
                has_blocking: primary_has_blocking,
                target: summary.bgee_target.as_ref(),
                prompt_count: summary.bgee_prompt_count,
                prompt_title: "Prompt Components (BGEE)",
                palette,
            },
        );
    } else if summary.show_bg2ee {
        ui.label(typo::monospace("BG2EE"));
        render_single_tab_badges(
            ui,
            state,
            &SingleTabBadges {
                value: "BG2EE",
                issue_count: secondary_issue_count,
                has_blocking: secondary_has_blocking,
                target: summary.bg2ee_target.as_ref(),
                prompt_count: summary.bg2ee_prompt_count,
                prompt_title: "Prompt Components (BG2EE)",
                palette,
            },
        );
    }
}

fn render_issue_badges(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    summary: &toolbar_support_step3::Step3ToolbarSummary,
    palette: ThemePalette,
) {
    let (primary_issue_count, primary_has_blocking) = summary.bgee_summary;
    let (secondary_issue_count, secondary_has_blocking) = summary.bg2ee_summary;
    if draw_tab_issue_badge(
        ui,
        &mut state.step3.active_game_tab,
        "BGEE",
        primary_issue_count,
        primary_has_blocking,
        palette,
    ) && let Some(target) = summary.bgee_target.as_ref()
    {
        toolbar_support_step3::open_toolbar_issue_popup(state, target);
    }
    if draw_tab_issue_badge(
        ui,
        &mut state.step3.active_game_tab,
        "BG2EE",
        secondary_issue_count,
        secondary_has_blocking,
        palette,
    ) && let Some(target) = summary.bg2ee_target.as_ref()
    {
        toolbar_support_step3::open_toolbar_issue_popup(state, target);
    }
}

struct SingleTabBadges<'a> {
    value: &'a str,
    issue_count: usize,
    has_blocking: bool,
    target: Option<&'a crate::app::step3_toolbar::Step3ToolbarIssueTarget>,
    prompt_count: usize,
    prompt_title: &'a str,
    palette: ThemePalette,
}

fn render_single_tab_badges(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    badges: &SingleTabBadges<'_>,
) {
    if draw_tab_issue_badge(
        ui,
        &mut state.step3.active_game_tab,
        badges.value,
        badges.issue_count,
        badges.has_blocking,
        badges.palette,
    ) && let Some(target) = badges.target
    {
        toolbar_support_step3::open_toolbar_issue_popup(state, target);
    }
    if draw_prompt_toolbar_badge(ui, badges.prompt_count, badges.palette) {
        open_toolbar_prompt_popup(state, badges.prompt_title);
    }
}

fn render_toolbar_actions(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let can_undo = if state.step3.active_game_tab == "BGEE" {
        !state.step3.bgee_undo_stack.is_empty()
    } else {
        !state.step3.bg2ee_undo_stack.is_empty()
    };
    let can_redo = if state.step3.active_game_tab == "BGEE" {
        !state.step3.bgee_redo_stack.is_empty()
    } else {
        !state.step3.bg2ee_redo_stack.is_empty()
    };
    render_diagnostics_button(ui, state, dev_mode, exe_fingerprint);
    render_expand_collapse_buttons(ui, state);
    render_undo_redo_buttons(ui, state, can_undo, can_redo);
}

fn render_diagnostics_button(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    if dev_mode {
        if ui
            .button("Export diagnostics")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP3_EXPORT_DIAGNOSTICS)
            .clicked()
        {
            toolbar_support_step3::export_diagnostics_from_step3(state, dev_mode, exe_fingerprint);
        }
    } else if ui.button("Restart App With Diagnostics").clicked() {
        toolbar_support_step3::restart_app_with_diagnostics_from_step3(state);
    }
}

fn render_expand_collapse_buttons(ui: &mut egui::Ui, state: &mut WizardState) {
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
}

fn render_undo_redo_buttons(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    can_undo: bool,
    can_redo: bool,
) {
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

pub fn render(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
    palette: ThemePalette,
) {
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

    render_toolbar(
        ui,
        state,
        dev_mode,
        exe_fingerprint,
        &toolbar_summary,
        palette,
    );
    if let Some(err) = toolbar_summary.compat_rules_error.as_deref() {
        ui.add_space(4.0);
        ui.label(
            crate::ui::shared::typography_global::weak(format!("Compat rules load failed: {err}"))
                .color(redesign_warning(palette)),
        );
    }

    ui.add_space(6.0);
    let mut jump_to_selected_requested = state.step3.jump_to_selected_requested;
    state.step3.jump_to_selected_requested = false;
    list_step3::render(
        ui,
        state,
        &mut jump_to_selected_requested,
        active_markers,
        palette,
    );
    crate::ui::step2::compat_window_step2::render(ui, state, palette);
    crate::ui::step2::prompt_popup_step2::render_prompt_popup(ui, state, palette);
    state.step3.jump_to_selected_requested =
        state.step3.jump_to_selected_requested || jump_to_selected_requested;
}
