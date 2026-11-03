// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

use eframe::egui;

use crate::app::source_form::SourceForm;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn};
use crate::ui::shared::redesign_tokens::{
    ThemePalette, redesign_border_soft, redesign_input_bg, redesign_text_muted,
    redesign_text_primary,
};

use super::versions_form;

pub(super) fn render(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    configured: &[String],
) {
    ui.horizontal(|ui| {
        ui.label(label("Categories", palette, true));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if button(ui, palette, "Clear categories").clicked() {
                form.kept.categories.clear();
                form.category_editor.clear_requested = true;
            }
        });
    });
    ui.add_space(6.0);
    let available = choices(form, configured);
    let response = render_input(ui, palette, form, &available);
    ui.add_space(10.0);
    if let Some(category) = render_cloud(ui, palette, form, &available) {
        form.add_category(&category);
        form.category_editor.input.clear();
        response.request_focus();
    }
}

fn label(value: &str, palette: ThemePalette, muted: bool) -> egui::RichText {
    egui::RichText::new(value)
        .size(12.0)
        .family(egui::FontFamily::Name("poppins_light".into()))
        .color(if muted {
            redesign_text_muted(palette)
        } else {
            redesign_text_primary(palette)
        })
}

fn button(ui: &mut egui::Ui, palette: ThemePalette, text: &str) -> egui::Response {
    redesign_btn(
        ui,
        palette,
        text,
        BtnOpts {
            small: true,
            ..Default::default()
        },
    )
}

fn render_cloud(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    available: &BTreeMap<String, String>,
) -> Option<String> {
    let mut remove = None;
    let mut add = None;
    let query = form.category_editor.input.trim().to_lowercase();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
        for (index, category) in form.kept.categories.iter().enumerate() {
            ui.push_id(index, |ui| {
                if ui
                    .add(
                        egui::Button::new(label(&format!("{category}  ×"), palette, false))
                            .fill(redesign_border_soft(palette))
                            .wrap(),
                    )
                    .on_hover_text(format!("Remove {category}"))
                    .clicked()
                {
                    remove = Some(index);
                }
            });
        }
        for (key, value) in available {
            if key.contains(&query)
                && !form
                    .kept
                    .categories
                    .iter()
                    .any(|category| category.trim().to_lowercase() == *key)
                && ui
                    .add(
                        egui::Button::new(label(value, palette, false))
                            .fill(redesign_input_bg(palette))
                            .stroke(egui::Stroke::new(1.0_f32, redesign_border_soft(palette)))
                            .wrap(),
                    )
                    .clicked()
            {
                add = Some(value.clone());
            }
        }
        if !query.is_empty() && !available.contains_key(&query) {
            let value = form.category_editor.input.trim();
            if ui
                .add(
                    egui::Button::new(label(value, palette, false))
                        .fill(redesign_input_bg(palette))
                        .stroke(egui::Stroke::new(1.0_f32, redesign_border_soft(palette)))
                        .wrap(),
                )
                .clicked()
            {
                add = Some(value.to_string());
            }
        }
    });
    if let Some(index) = remove {
        form.kept.categories.remove(index);
    }
    add
}

fn choices(form: &SourceForm, configured: &[String]) -> BTreeMap<String, String> {
    let mut choices = BTreeMap::new();
    for value in configured
        .iter()
        .chain(&form.category_editor.original)
        .chain(&form.kept.categories)
    {
        let label = value.trim();
        if !label.is_empty() {
            choices
                .entry(label.to_lowercase())
                .or_insert_with(|| label.to_string());
        }
    }
    choices
}

fn render_input(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    form: &mut SourceForm,
    available: &BTreeMap<String, String>,
) -> egui::Response {
    let width = ui.available_width();
    let response = versions_form::text_field(
        ui,
        palette,
        &mut form.category_editor.input,
        "Find or add a category…",
        false,
        width,
    );
    let enter = (response.has_focus() || response.lost_focus())
        && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if enter {
        let input = std::mem::take(&mut form.category_editor.input);
        let value = available
            .get(&input.trim().to_lowercase())
            .map_or_else(|| input.trim(), String::as_str);
        form.add_category(value);
        response.request_focus();
    }
    response
}
