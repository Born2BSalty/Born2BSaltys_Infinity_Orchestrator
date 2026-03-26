// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::ui::shared::layout_tokens_global::*;
use crate::ui::state::WizardState;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::service_selection_step2::jump_to_target;
use crate::ui::step2::service_step2::{clear_all, select_visible};
use crate::ui::step2::state_step2::active_mods_mut;
use crate::ui::step5::service_step5::export_diagnostics;

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
            if ui
                .add_enabled(
                    has_scanned,
                    egui::Button::new("Export Compat").min_size(egui::vec2(114.0, STEP2_BTN_H)),
                )
                .on_hover_text(crate::ui::shared::tooltip_global::STEP2_EXPORT_COMPAT)
                .clicked()
            {
                *action = Some(Step2Action::ExportCompatReport);
            }
        });
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

pub fn render_prompt_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.prompt_popup_open {
        return;
    }
    let title = state.step2.prompt_popup_title.clone();
    let text = state.step2.prompt_popup_text.clone();
    let jump_ids = collect_prompt_jump_component_ids(state, &title, &text);
    let mut open = state.step2.prompt_popup_open;
    let mut jump_to_component_id: Option<u32> = None;
    egui::Window::new(format!("Parsed prompts - {}", title))
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(700.0)
        .default_height(320.0)
        .show(ui.ctx(), |ui| {
            ui.label("Prompt summary from Lapdu parser:");
            ui.separator();
            let max_scroll_height = (ui.available_height() - 72.0).max(140.0);
            let scroll_width = ui.available_width();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(max_scroll_height)
                .show(ui, |ui| {
                    ui.set_min_width(scroll_width);
                    ui.label(&text);
                });
            if !jump_ids.is_empty() {
                ui.add_space(SPACE_MD);
                ui.separator();
                ui.add_space(SPACE_SM);
                ui.label(crate::ui::shared::typography_global::strong("Jump to component"));
                ui.add_space(SPACE_XS);
                ui.horizontal_wrapped(|ui| {
                    for component_id in jump_ids {
                        let button_text =
                            crate::ui::shared::typography_global::monospace(component_id.to_string())
                                .color(crate::ui::shared::theme_global::accent_numbers());
                        if ui
                            .add(
                                egui::Button::new(button_text)
                                    .min_size(egui::vec2(42.0, 22.0))
                                    .fill(ui.visuals().widgets.inactive.bg_fill)
                                    .stroke(ui.visuals().widgets.inactive.bg_stroke),
                            )
                            .clicked()
                        {
                            jump_to_component_id = Some(component_id);
                        }
                    }
                });
            }
        });
    state.step2.prompt_popup_open = open;
    if let Some(component_id) = jump_to_component_id {
        let game_tab = state.step2.active_game_tab.clone();
        let mod_ref = parse_prompt_popup_mod_ref(&title);
        jump_to_target(state, &game_tab, &mod_ref, Some(component_id));
        state.step2.jump_to_selected_requested = true;
    }
}

fn parse_prompt_popup_mod_ref(title: &str) -> String {
    title
        .split(" #")
        .next()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| title.trim().to_string())
}

fn parse_prompt_jump_component_ids(text: &str) -> Vec<u32> {
    let mut ids = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("Component:") else {
            continue;
        };
        let id_token = rest.split_whitespace().next().unwrap_or_default();
        if let Ok(id) = id_token.parse::<u32>()
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids
}

fn collect_prompt_jump_component_ids(state: &WizardState, title: &str, text: &str) -> Vec<u32> {
    let mut ids = parse_prompt_jump_component_ids(text);
    let mod_ref = parse_prompt_popup_mod_ref(title);
    let target_mod_key = normalize_mod_key(&mod_ref);
    for mod_state in active_mods_ref(state) {
        if normalize_mod_key(&mod_state.tp_file) != target_mod_key {
            continue;
        }
        for component in &mod_state.components {
            let has_prompt = component
                .prompt_summary
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
                || !component.prompt_events.is_empty();
            if !has_prompt {
                continue;
            }
            if let Ok(id) = component.component_id.trim().parse::<u32>()
                && !ids.contains(&id)
            {
                ids.push(id);
            }
        }
    }
    ids.sort_unstable();
    ids
}

fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
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
            crate::ui::step2::content_step2::compat_popup_filter_row::render_filter_row(ui, state);
            ui.add_space(6.0);
            crate::ui::step2::content_step2::compat_popup_action_row::render_action_row(ui, state);
        });
    state.step2.compat_popup_open = open && state.step2.compat_popup_open;
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
            crate::ui::step2::content_step2::details_pane_action_bar::render(ui, &details, action);
            ui.separator();
            crate::ui::step2::content_step2::details_pane_content::render(ui, &details, action);
        });
    });
}

pub(crate) use crate::ui::step2::state_step2::Step2Details;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_action_row;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_details;
pub(crate) use crate::ui::step2::compat_popup_step2::compat_popup_filter_row;
pub(crate) use crate::ui::step2::details_pane_step2::details_pane_action_bar;
pub(crate) use crate::ui::step2::details_pane_step2::details_pane_content;

pub(crate) mod step2_details_select {
    use crate::ui::state::WizardState;
    use crate::ui::step2::state_step2::Step2Details;

    pub fn selected_details(state: &WizardState) -> Step2Details {
        crate::ui::step2::service_step2::selected_details(state)
    }
}
