// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_popup_nav;
use crate::app::prompt_popup_text::{
    PromptToolbarModEntry, format_prompt_toolbar_row, prompt_toolbar_count,
};
use crate::app::state::{PromptPopupMode, WizardState};
use crate::ui::shared::layout_tokens_global::{SPACE_MD, SPACE_SM, SPACE_XS};

const PROMPT_TOOLBAR_ROW_H: f32 = 22.0;
const MOD_LEVEL_PROMPT_ROW: &str = "Mod-level prompt";

pub fn render_prompt_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    if !state.step2.prompt_popup_open {
        return;
    }
    if state.step2.prompt_popup_mode == PromptPopupMode::ToolbarIndex {
        render_prompt_toolbar_popup(ui, state);
        return;
    }
    let title = state.step2.prompt_popup_title.clone();
    let text = state.step2.prompt_popup_text.clone();
    let jump_ids = prompt_popup_nav::collect_text_prompt_jump_ids(state, &title, &text);
    let mut open = state.step2.prompt_popup_open;
    let mut jump_to_component_id: Option<u32> = None;
    let window_title = format!("Parsed prompts - {title}");
    let trailing_height_id =
        egui::Id::new(&window_title).with(("trailing_height", jump_ids.is_empty()));
    egui::Window::new(&window_title)
        .open(&mut open)
        .resizable(true)
        .collapsible(true)
        .default_width(700.0)
        .default_height(320.0)
        .min_size(egui::vec2(320.0, 220.0))
        .show(ui.ctx(), |ui| {
            ui.label("Prompt summary from Lapdu parser:");
            ui.separator();
            let trailing_default = if jump_ids.is_empty() {
                SPACE_SM
            } else {
                PROMPT_POPUP_FOOTER_RESERVE
            };
            let reserved_height = ui
                .data(|data| data.get_temp::<f32>(trailing_height_id))
                .unwrap_or(trailing_default)
                + SPACE_SM;
            let max_scroll_height =
                (ui.available_height() - reserved_height).max(PROMPT_POPUP_MIN_SCROLL_HEIGHT);
            let scroll_width = ui.available_width();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(max_scroll_height)
                .show(ui, |ui| {
                    ui.set_min_width(scroll_width);
                    ui.label(&text);
                });
            let trailing_top = ui.cursor().top();
            if jump_ids.is_empty() {
                ui.add_space(SPACE_SM);
            } else {
                jump_to_component_id = render_jump_footer(ui, &jump_ids);
            }
            let trailing_height = ui.cursor().top() - trailing_top;
            ui.data_mut(|data| data.insert_temp(trailing_height_id, trailing_height));
        });
    state.step2.prompt_popup_open = open;
    if let Some(component_id) = jump_to_component_id {
        prompt_popup_nav::apply_text_prompt_jump(state, &title, component_id);
    }
}

const PROMPT_POPUP_FOOTER_RESERVE: f32 = 100.0;
const PROMPT_POPUP_MIN_SCROLL_HEIGHT: f32 = 60.0;

fn render_jump_footer(ui: &mut egui::Ui, jump_ids: &[u32]) -> Option<u32> {
    let mut jump_to_component_id = None;
    ui.add_space(SPACE_MD);
    ui.separator();
    ui.add_space(SPACE_SM);
    ui.label(crate::ui::shared::typography_global::strong(
        "Jump to component",
    ));
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
                jump_to_component_id = Some(*component_id);
            }
        }
    });
    jump_to_component_id
}

pub(crate) fn open_text_prompt_popup(state: &mut WizardState, title: String, text: String) {
    prompt_popup_nav::open_text_prompt_popup(state, title, text);
}

pub(crate) fn open_toolbar_prompt_popup(state: &mut WizardState, title: &str) {
    prompt_popup_nav::open_toolbar_prompt_popup(state, title);
}

pub(crate) fn draw_prompt_toolbar_badge(ui: &mut egui::Ui, count: usize) -> bool {
    if count == 0 {
        return false;
    }
    let prompt_text = crate::ui::shared::typography_global::strong(format!("PROMPT {count}"))
        .color(crate::ui::shared::theme_global::prompt_text())
        .size(crate::ui::shared::typography_global::SIZE_PILL_TEXT);
    ui.add(
        egui::Button::new(prompt_text)
            .fill(crate::ui::shared::theme_global::prompt_fill())
            .stroke(egui::Stroke::new(
                crate::ui::shared::layout_tokens_global::BORDER_THIN,
                crate::ui::shared::theme_global::prompt_stroke(),
            ))
            .corner_radius(egui::CornerRadius::same(7))
            .min_size(egui::vec2(0.0, 18.0)),
    )
    .on_hover_text(crate::ui::shared::tooltip_global::SHOW_PARSED_PROMPTS)
    .clicked()
}

pub(crate) fn collect_step2_prompt_toolbar_entries(
    state: &WizardState,
) -> Vec<PromptToolbarModEntry> {
    let prompt_eval = build_prompt_eval_context(state);
    crate::app::prompt_popup_text::collect_step2_prompt_toolbar_entries(
        prompt_popup_nav::active_step2_mods(state),
        &prompt_eval,
    )
}

fn prompt_toolbar_row_text(ui: &egui::Ui, row: &str) -> egui::WidgetText {
    let (id_text, label_text) = row.split_once(' ').unwrap_or((row, ""));
    let mut job = egui::text::LayoutJob::default();
    job.append(
        id_text,
        0.0,
        egui::TextFormat {
            font_id: egui::TextStyle::Monospace.resolve(ui.style()),
            color: crate::ui::shared::theme_global::accent_numbers(),
            ..Default::default()
        },
    );
    if !label_text.is_empty() {
        job.append(
            label_text,
            SPACE_XS,
            egui::TextFormat {
                font_id: egui::TextStyle::Body.resolve(ui.style()),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
    }
    egui::WidgetText::from(job)
}

fn render_prompt_toolbar_row(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    hover: &str,
) -> bool {
    let width = ui.available_width();
    let fill = ui.visuals().widgets.inactive.bg_fill;
    let stroke = ui.visuals().widgets.inactive.bg_stroke;
    ui.add(
        egui::Button::new(text)
            .truncate()
            .min_size(egui::vec2(width, PROMPT_TOOLBAR_ROW_H))
            .fill(fill)
            .stroke(stroke),
    )
    .on_hover_text(hover)
    .clicked()
}

fn render_prompt_toolbar_popup(ui: &egui::Ui, state: &mut WizardState) {
    let title = state.step2.prompt_popup_title.clone();
    let entries = prompt_popup_nav::collect_active_prompt_toolbar_entries(state);
    let mut open = state.step2.prompt_popup_open;
    let mut jump_target: Option<(String, Option<u32>)> = None;
    egui::Window::new(title)
        .open(&mut open)
        .resizable(true)
        .collapsible(true)
        .default_width(420.0)
        .default_height(320.0)
        .min_size(egui::vec2(320.0, 180.0))
        .show(ui.ctx(), |ui| {
            if entries.is_empty() {
                ui.label("No prompts in the active tab.");
                return;
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in &entries {
                        let header = format!(
                            "{} ({})",
                            entry.mod_name,
                            prompt_toolbar_count(std::slice::from_ref(entry))
                        );
                        egui::CollapsingHeader::new(header)
                            .default_open(false)
                            .show(ui, |ui| {
                                if entry.mod_level
                                    && render_prompt_toolbar_row(
                                        ui,
                                        MOD_LEVEL_PROMPT_ROW,
                                        MOD_LEVEL_PROMPT_ROW,
                                    )
                                {
                                    jump_target = Some((entry.tp_file.clone(), None));
                                }
                                for component in &entry.components {
                                    let row =
                                        format_prompt_toolbar_row(component.id, &component.label);
                                    let row_text = prompt_toolbar_row_text(ui, &row);
                                    if render_prompt_toolbar_row(ui, row_text, &row) {
                                        jump_target =
                                            Some((entry.tp_file.clone(), Some(component.id)));
                                    }
                                }
                            });
                    }
                });
        });
    state.step2.prompt_popup_open = open;
    if let Some((mod_ref, component_id)) = jump_target {
        prompt_popup_nav::apply_toolbar_prompt_jump(state, &mod_ref, component_id);
    }
}
