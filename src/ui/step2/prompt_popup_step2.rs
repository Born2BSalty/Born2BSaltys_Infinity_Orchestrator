// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::prompt_popup_nav;
use crate::app::prompt_popup_text::PromptToolbarModEntry;
use crate::app::state::{PromptPopupMode, WizardState};
use crate::ui::shared::layout_tokens_global::{SPACE_MD, SPACE_SM, SPACE_XS};

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
                ui.label(crate::ui::shared::typography_global::strong(
                    "Jump to component",
                ));
                ui.add_space(SPACE_XS);
                ui.horizontal_wrapped(|ui| {
                    for component_id in jump_ids {
                        let button_text = crate::ui::shared::typography_global::monospace(
                            component_id.to_string(),
                        )
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
        prompt_popup_nav::apply_text_prompt_jump(state, &title, component_id);
    }
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
    crate::app::prompt_popup_text::collect_step2_prompt_toolbar_entries(
        prompt_popup_nav::active_step2_mods(state),
    )
}

fn render_prompt_toolbar_popup(ui: &mut egui::Ui, state: &mut WizardState) {
    let title = state.step2.prompt_popup_title.clone();
    let entries = prompt_popup_nav::collect_active_prompt_toolbar_entries(state);
    let mut open = state.step2.prompt_popup_open;
    let mut jump_target: Option<(String, u32)> = None;
    egui::Window::new(title)
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(420.0)
        .default_height(320.0)
        .show(ui.ctx(), |ui| {
            if entries.is_empty() {
                ui.label("No component prompts in the active tab.");
                return;
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                for entry in &entries {
                    let header = format!("{} ({})", entry.mod_name, entry.component_ids.len());
                    egui::CollapsingHeader::new(header)
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for component_id in &entry.component_ids {
                                    let button_text =
                                        crate::ui::shared::typography_global::monospace(
                                            component_id.to_string(),
                                        )
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
                                        jump_target = Some((entry.tp_file.clone(), *component_id));
                                    }
                                }
                            });
                        });
                }
            });
        });
    state.step2.prompt_popup_open = open;
    if let Some((mod_ref, component_id)) = jump_target {
        prompt_popup_nav::apply_toolbar_prompt_jump(state, &mod_ref, component_id);
    }
}
