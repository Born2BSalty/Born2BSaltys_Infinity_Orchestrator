// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::layout_tokens_global::*;
use crate::ui::state::WizardState;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::service_list_ops_step2::{clear_all, select_visible};
use crate::ui::step2::state_step2::active_mods_mut;
use crate::ui::step2::toolbar_compat_step2::{
    active_tab_compat_summary, draw_active_tab_issue_badge, first_active_tab_issue_target,
};
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

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

    let button = egui::Button::new(crate::ui::shared::typography_global::plain(value).color(text_color))
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
    ui.scope_builder(egui::UiBuilder::new().max_rect(title_rect), |ui| {
        ui.horizontal(|ui| {
            ui.heading("Step2: Scan and Select");
            if dev_mode {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Export diagnostics").clicked() {
                        match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
                            Ok(path) => {
                                state.step5.last_status_text =
                                    format!("Diagnostics exported: {}", path.display());
                            }
                            Err(err) => {
                                state.step5.last_status_text =
                                    format!("Diagnostics export failed: {err}");
                            }
                        }
                    }
                });
            }
        });
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(subtitle_rect), |ui| {
        ui.label("Choose components to install.")
            .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SUBTITLE);
    });
    ui.scope_builder(egui::UiBuilder::new().max_rect(search_rect), |ui| {
        let search_w = search_rect.width().min(STEP2_SEARCH_MAX_W);
        let resp = ui.add_sized(
            [search_w, STEP2_SEARCH_INPUT_H],
            egui::TextEdit::singleline(&mut state.step2.search_query)
                .hint_text("Search mods or components..."),
        );
        resp.on_hover_text(crate::ui::shared::tooltip_global::STEP2_SEARCH);
    });
}

pub fn render_controls(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    controls_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(controls_rect), |ui| {
        let mut refresh_compat = false;
        let bgee_scanned = !state.step2.bgee_mods.is_empty();
        let bg2_scanned = !state.step2.bg2ee_mods.is_empty();
        let has_any_checked = active_mods_ref(state)
            .iter()
            .any(|m| m.checked || m.components.iter().any(|c| c.checked));
        ui.horizontal(|ui| {
            if ui
                .add_sized([STEP2_SCAN_BTN_W, STEP2_BTN_H], egui::Button::new("Scan Mods Folder"))
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
            let has_scanned = bgee_scanned || bg2_scanned;
            if has_any_checked
                && ui
                    .add_enabled(
                        has_scanned,
                        egui::Button::new("Clear All").min_size(egui::vec2(84.0, STEP2_BTN_H)),
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_CLEAR_ALL)
                    .clicked()
            {
                let mut next_order = state.step2.next_selection_order;
                let mods = active_mods_mut(&mut state.step2);
                clear_all(mods, &mut next_order);
                state.step2.next_selection_order = next_order;
                state.step2.selected = None;
                refresh_compat = true;
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Select Visible").min_size(egui::vec2(108.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_VISIBLE)
                .clicked()
            {
                let filter = state.step2.search_query.trim().to_lowercase();
                let mut next_order = state.step2.next_selection_order;
                let mods = active_mods_mut(&mut state.step2);
                select_visible(mods, &filter, &mut next_order);
                state.step2.next_selection_order = next_order;
                refresh_compat = true;
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Collapse All").min_size(egui::vec2(104.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_COLLAPSE_ALL)
                .clicked()
            {
                state.step2.collapse_default_open = false;
                state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
            }
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Expand All").min_size(egui::vec2(94.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_EXPAND_ALL)
                .clicked()
            {
                state.step2.collapse_default_open = true;
                state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
            }
            if ui
                .add_enabled(
                    state.step2.selected.is_some(),
                    egui::Button::new("Jump to Selected").min_size(egui::vec2(132.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_JUMP_SELECTED)
                .clicked()
            {
                state.step2.jump_to_selected_requested = true;
            }
        });
        if refresh_compat {
            crate::ui::step2::service_compat_rules_step2::apply_compat_rules(
                &state.step1,
                &mut state.step2.bgee_mods,
                &mut state.step2.bg2ee_mods,
            );
        }
    });
}

pub fn render_tabs(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    tabs_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(tabs_rect), |ui| {
        ui.horizontal(|ui| {
            ui.label(crate::ui::shared::typography_global::section_title("Mods / Components"))
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_MODS_COMPONENTS);
            let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
            let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
            let active_is_bgee = state.step2.active_game_tab == "BGEE";
            let active_is_bg2 = state.step2.active_game_tab == "BG2EE";
            let bgee_scanned = !state.step2.bgee_mods.is_empty();
            let bg2_scanned = !state.step2.bg2ee_mods.is_empty();

            if show_bgee && show_bg2ee {
                draw_tab(ui, &mut state.step2.active_game_tab, "BGEE");
                draw_tab(ui, &mut state.step2.active_game_tab, "BG2EE");
            } else if show_bgee {
                ui.label(crate::ui::shared::typography_global::monospace("BGEE"));
            } else if show_bg2ee {
                ui.label(crate::ui::shared::typography_global::monospace("BG2EE"));
            }

            let issue_summary = active_tab_compat_summary(active_mods_ref(state));
            let target_filter = if state.step2.compat_popup_filter.eq_ignore_ascii_case("All") {
                issue_summary.dominant_filter
            } else {
                state.step2.compat_popup_filter.as_str()
            };
            let issue_target = first_active_tab_issue_target(
                active_mods_ref(state),
                target_filter,
            );

            ui.add_space(10.0);
            if active_is_bgee
                && ui
                    .add_enabled(
                        bgee_scanned,
                        egui::Button::new("Select BGEE via WeiDU Log")
                            .min_size(egui::vec2(STEP2_TABS_LOG_BTN_BGEE_W, 24.0)),
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BGEE_LOG)
                    .clicked()
            {
                *action = Some(Step2Action::SelectBgeeViaLog);
            }
            if active_is_bg2
                && ui
                    .add_enabled(
                        bg2_scanned,
                        egui::Button::new("Select BG2EE via WeiDU Log")
                            .min_size(egui::vec2(STEP2_TABS_LOG_BTN_BG2EE_W, 24.0)),
                    )
                    .on_hover_text(crate::ui::shared::tooltip_global::STEP2_SELECT_BG2EE_LOG)
                    .clicked()
            {
                *action = Some(Step2Action::SelectBg2eeViaLog);
            }
            if draw_active_tab_issue_badge(
                ui,
                &state.step2.active_game_tab,
                &issue_summary,
                &state.step2.compat_popup_filter,
            )
                && let Some(target) = issue_target
            {
                if state.step2.compat_popup_filter.eq_ignore_ascii_case("All") {
                    state.step2.compat_popup_filter = issue_summary.dominant_filter.to_string();
                }
                *action = Some(Step2Action::OpenCompatForComponent {
                    game_tab: state.step2.active_game_tab.clone(),
                    tp_file: target.tp_file,
                    component_id: target.component_id,
                    component_key: target.component_key,
                });
            }
        });
    });
}

fn active_mods_ref(state: &WizardState) -> &Vec<crate::ui::state::Step2ModState> {
    if state.step2.active_game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    }
}

pub fn render_compat_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.compat_popup_open {
        return;
    }

    let mut open = state.step2.compat_popup_open;
    egui::Window::new("Step 2 Compatibility")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(620.0, 300.0))
        .min_width(420.0)
        .min_height(200.0)
        .show(ui.ctx(), |ui| {
            let details_h = (ui.available_height() - 58.0).max(40.0);
            egui::ScrollArea::vertical()
                .max_height(details_h)
                .show(ui, |ui| {
                    crate::ui::step2::content_step2::compat_popup_details::render_details(ui, state);
                });

            ui.add_space(10.0);
            crate::ui::step2::content_step2::compat_popup_action_row::render_action_row(ui, state);
        });
    state.step2.compat_popup_open = open && state.step2.compat_popup_open;
    if !state.step2.compat_popup_open {
        state.step2.compat_popup_issue_override = None;
    }
}

pub fn render_details_pane(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    right_rect: egui::Rect,
) {
    ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
        let details: Step2Details =
            crate::ui::step2::content_step2::step2_details_select::selected_details(state);
        ui.group(|ui| {
            ui.set_min_size(right_rect.size() - egui::vec2(12.0, 12.0));
            ui.label(crate::ui::shared::typography_global::section_title("Details"));
            ui.add_space(4.0);
            crate::ui::step2::content_step2::details_pane_content::render(ui, &details, action);
        });
    });
}

pub(crate) use crate::ui::step2::state_step2::Step2Details;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_action_row;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_details;
pub(crate) use crate::ui::step2::details_pane_step2::details_pane_content;

pub(crate) mod step2_details_select {
    use crate::ui::state::WizardState;
    use crate::ui::step2::state_step2::Step2Details;

    pub fn selected_details(state: &WizardState) -> Step2Details {
        crate::ui::step2::service_details_step2::selected_details(state)
    }
}
