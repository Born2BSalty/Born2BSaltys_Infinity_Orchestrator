// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::WizardState;
use crate::ui::shared::layout_tokens_global::*;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_accent, redesign_border_soft,
    redesign_shell_bg, redesign_text_on_accent, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::prompt_popup_step2::{
    collect_step2_prompt_toolbar_entries, draw_prompt_toolbar_badge,
};
use crate::ui::step2::state_step2::{non_scan_controls_locked, review_edit_scan_complete};
use crate::ui::step2::toolbar_actions_step2;
use crate::ui::step2::toolbar_compat_step2::{
    active_tab_compat_summary, draw_active_tab_issue_badge, first_active_tab_issue_target,
};

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
                crate::ui::shared::layout_tokens_global::RADIUS_SM as u8,
            ));

    if ui.add_sized([58.0, 24.0], button).clicked() {
        *active = value.to_string();
    }
}

pub fn render_header(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    title_rect: egui::Rect,
    subtitle_rect: egui::Rect,
    search_rect: egui::Rect,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    let mut title_text_rect = None;
    let mut export_button_rect = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        ui.horizontal(|ui| {
            title_text_rect = Some(ui.heading("Step2: Scan and Select").rect);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if dev_mode {
                    let export_response = ui.button("Export diagnostics");
                    export_button_rect = Some(export_response.rect);
                    if export_response.clicked() {
                        toolbar_actions_step2::export_diagnostics_from_step2(
                            state,
                            dev_mode,
                            exe_fingerprint,
                        );
                    }
                } else {
                    let restart_response = ui.button("Restart App With Diagnostics");
                    export_button_rect = Some(restart_response.rect);
                    if restart_response.clicked() {
                        toolbar_actions_step2::restart_app_with_diagnostics_from_step2(state);
                    }
                }
            });
        });
    });
    clear_selection_from_empty_header_space(
        ui,
        state,
        title_rect,
        title_text_rect,
        export_button_rect,
    );
    let mut subtitle_text_rect = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(subtitle_rect), |ui| {
        let subtitle_response = ui
            .label("Choose components to install.")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SUBTITLE);
        subtitle_text_rect = Some(subtitle_response.rect);
    });
    clear_selection_from_empty_header_space(ui, state, subtitle_rect, subtitle_text_rect, None);
    ui.scope_builder(egui::UiBuilder::new().max_rect(search_rect), |ui| {
        let search_w = search_rect.width().min(STEP2_SEARCH_MAX_W);
        let resp = ui
            .add_enabled_ui(!state.step1.installs_exactly_from_weidu_logs(), |ui| {
                ui.add_sized(
                    [search_w, STEP2_SEARCH_INPUT_H],
                    egui::TextEdit::singleline(&mut state.step2.search_query)
                        .hint_text("Search mods or components..."),
                )
            })
            .inner;
        let search_field_rect = resp.rect;
        resp.on_hover_text(crate::ui::shared::tooltip_global::STEP2_SEARCH);
        clear_selection_from_empty_header_space(
            ui,
            state,
            search_rect,
            Some(search_field_rect),
            None,
        );
    });
}

pub fn render_controls(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    controls_rect: egui::Rect,
    _palette: ThemePalette,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(controls_rect), |ui| {
        let ui_locked =
            non_scan_controls_locked(state) || state.step1.installs_exactly_from_weidu_logs();
        let bgee_scanned = !state.step2.bgee_mods.is_empty();
        let bg2_scanned = !state.step2.bg2ee_mods.is_empty();
        let has_completed_scan = bgee_scanned || bg2_scanned || review_edit_scan_complete(state);
        let has_any_checked = active_mods_ref(state)
            .iter()
            .any(|m| m.checked || m.components.iter().any(|c| c.checked));
        let can_scan_mods_folder = if state.step1.bootstraps_from_weidu_logs() {
            !state.step2.is_scanning
        } else {
            !state.step2.is_scanning && state.step1_mods_folder_has_tp2 != Some(false)
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    can_scan_mods_folder,
                    egui::Button::new("Scan Mods Folder")
                        .min_size(egui::vec2(STEP2_SCAN_BTN_W, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SCAN)
                .clicked()
            {
                *action = Some(Step2Action::StartScan);
            }
            if state.step2.is_scanning
                && ui
                    .add_sized(
                        [STEP2_CANCEL_SCAN_BTN_W, STEP2_BTN_H],
                        egui::Button::new("Cancel Scan"),
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_CANCEL_SCAN)
                    .clicked()
            {
                *action = Some(Step2Action::CancelScan);
            }
            if has_any_checked
                && ui
                    .add_enabled(
                        has_completed_scan && !ui_locked,
                        egui::Button::new("Clear All").min_size(egui::vec2(84.0, STEP2_BTN_H)),
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_CLEAR_ALL)
                    .clicked()
            {
                toolbar_actions_step2::clear_all_and_refresh_compat(state);
            }
            if ui
                .add_enabled(
                    has_completed_scan && !ui_locked,
                    egui::Button::new("Select Visible").min_size(egui::vec2(108.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_VISIBLE)
                .clicked()
            {
                toolbar_actions_step2::select_visible_and_refresh_compat(state);
            }
            if ui
                .add_enabled(
                    has_completed_scan && !ui_locked,
                    egui::Button::new("Collapse All").min_size(egui::vec2(104.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_COLLAPSE_ALL)
                .clicked()
            {
                toolbar_actions_step2::collapse_all(state);
            }
            if ui
                .add_enabled(
                    has_completed_scan && !ui_locked,
                    egui::Button::new("Expand All").min_size(egui::vec2(94.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_EXPAND_ALL)
                .clicked()
            {
                toolbar_actions_step2::expand_all(state);
            }
            let jump_response = ui
                .add_enabled(
                    state.step2.selected.is_some() && !ui_locked,
                    egui::Button::new("Jump to Selected").min_size(egui::vec2(132.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_JUMP_SELECTED);
            if jump_response.clicked() {
                state.step2.jump_to_selected_requested = true;
            }
            clear_selection_from_empty_header_space(
                ui,
                state,
                controls_rect,
                Some(jump_response.rect),
                None,
            );
        });
    });
}

pub fn render_tabs(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    tabs_rect: egui::Rect,
    palette: ThemePalette,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(tabs_rect), |ui| {
        ui.horizontal(|ui| {
            let title_response = ui
                .label(crate::ui::shared::typography_global::section_title(
                    "Mods / Components",
                ))
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_MODS_COMPONENTS);
            ui.add_enabled_ui(!non_scan_controls_locked(state), |ui| {
                let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
                let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
                let active_is_bgee = state.step2.active_game_tab == "BGEE";
                let active_is_bg2 = state.step2.active_game_tab == "BG2EE";
                let bgee_scanned = !state.step2.bgee_mods.is_empty();
                let bg2_scanned = !state.step2.bg2ee_mods.is_empty();
                let has_completed_scan =
                    bgee_scanned || bg2_scanned || review_edit_scan_complete(state);

                if show_bgee && show_bg2ee {
                    draw_tab(ui, &mut state.step2.active_game_tab, "BGEE", palette);
                    draw_tab(ui, &mut state.step2.active_game_tab, "BG2EE", palette);
                } else if show_bgee {
                    ui.label(crate::ui::shared::typography_global::monospace("BGEE"));
                } else if show_bg2ee {
                    ui.label(crate::ui::shared::typography_global::monospace("BG2EE"));
                }

                let issue_summary = active_tab_compat_summary(active_mods_ref(state));
                let prompt_entries = collect_step2_prompt_toolbar_entries(state);
                let prompt_count: usize = prompt_entries
                    .iter()
                    .map(|entry| entry.component_ids.len())
                    .sum();
                let can_bootstrap_from_log = if state.step1.installs_exactly_from_weidu_logs() {
                    false
                } else if state.step1.bootstraps_from_weidu_logs() {
                    review_edit_scan_complete(state)
                } else {
                    has_completed_scan
                };
                let target_filter = if state.step2.compat_popup_filter.eq_ignore_ascii_case("All") {
                    issue_summary.dominant_filter
                } else {
                    state.step2.compat_popup_filter.as_str()
                };
                let issue_target =
                    first_active_tab_issue_target(active_mods_ref(state), target_filter);
                let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
                let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();

                ui.add_space(10.0);
                if !exact_log_mode
                    && active_is_bgee
                    && ui
                        .add_enabled(
                            bgee_scanned || can_bootstrap_from_log,
                            egui::Button::new("Select BGEE via WeiDU Log")
                                .min_size(egui::vec2(STEP2_TABS_LOG_BTN_BGEE_W, 24.0)),
                        )
                        .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BGEE_LOG)
                        .clicked()
                {
                    *action = Some(Step2Action::SelectBgeeViaLog);
                }
                if !exact_log_mode
                    && active_is_bg2
                    && ui
                        .add_enabled(
                            bg2_scanned || can_bootstrap_from_log,
                            egui::Button::new("Select BG2EE via WeiDU Log")
                                .min_size(egui::vec2(STEP2_TABS_LOG_BTN_BG2EE_W, 24.0)),
                        )
                        .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BG2EE_LOG)
                        .clicked()
                {
                    *action = Some(Step2Action::SelectBg2eeViaLog);
                }
                if build_from_scanned_mods {
                    if ui
                        .add_enabled(
                            has_completed_scan && !state.step2.is_scanning,
                            egui::Button::new("Updates...").min_size(egui::vec2(124.0, 24.0)),
                        )
                        .on_hover_text("Open the updates popup.")
                        .clicked()
                    {
                        *action = Some(Step2Action::OpenUpdatePopup);
                    }
                } else if exact_log_mode {
                    if ui
                        .add_enabled(
                            !state.step2.is_scanning,
                            egui::Button::new("Mod List...").min_size(egui::vec2(124.0, 24.0)),
                        )
                        .on_hover_text("Open the exact-log mod list popup.")
                        .clicked()
                    {
                        *action = Some(Step2Action::OpenUpdatePopup);
                    }
                } else if ui
                    .add_enabled(
                        review_edit_scan_complete(state) && !state.step2.is_scanning,
                        egui::Button::new("Updates...").min_size(egui::vec2(124.0, 24.0)),
                    )
                    .on_hover_text("Open the updates popup.")
                    .clicked()
                {
                    *action = Some(Step2Action::OpenUpdatePopup);
                }
                if draw_active_tab_issue_badge(
                    ui,
                    &state.step2.active_game_tab,
                    &issue_summary,
                    &state.step2.compat_popup_filter,
                    palette,
                ) && let Some(target) = issue_target
                {
                    toolbar_actions_step2::open_active_tab_issue(
                        state,
                        &issue_summary,
                        Some(target),
                    );
                }
                if draw_prompt_toolbar_badge(ui, prompt_count, palette) {
                    toolbar_actions_step2::open_prompt_toolbar(state);
                }
            });
            clear_selection_from_empty_header_space(
                ui,
                state,
                tabs_rect,
                Some(title_response.rect),
                None,
            );
        });
    });
}

fn clear_selection_from_empty_header_space(
    ui: &egui::Ui,
    state: &mut WizardState,
    row_rect: egui::Rect,
    protected_a: Option<egui::Rect>,
    protected_b: Option<egui::Rect>,
) {
    let Some(pos) = ui.input(|input| {
        input
            .pointer
            .primary_clicked()
            .then(|| input.pointer.interact_pos())
            .flatten()
    }) else {
        return;
    };
    if !row_rect.contains(pos) {
        return;
    }
    let protected = protected_a
        .map(|rect| rect.expand(2.0).contains(pos))
        .unwrap_or(false)
        || protected_b
            .map(|rect| rect.expand(2.0).contains(pos))
            .unwrap_or(false);
    if !protected {
        state.step2.selected = None;
    }
}

fn active_mods_ref(state: &WizardState) -> &Vec<crate::app::state::Step2ModState> {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

pub fn render_compat_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    crate::ui::step2::compat_window_step2::render(ui, state, ThemePalette::Dark);
}
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_action_row;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_details;

pub(crate) mod step2_details_select {
    use crate::app::state::WizardState;
    use crate::ui::step2::state_step2::Step2Details;

    pub fn selected_details(state: &WizardState) -> Step2Details {
        crate::ui::step2::service_details_step2::selected_details(state)
    }
}
