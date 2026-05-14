// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::WizardState;
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_compat_info_fill, redesign_text_primary,
};
use crate::ui::step2::action_step2::Step2Action;

const UPDATE_CHECK_GRID_SPACING_X: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceChoiceLayout {
    list_prefix_width: f32,
    dropdown_width: f32,
}

impl SourceChoiceLayout {
    pub(super) fn list_prefix_width(self) -> f32 {
        self.list_prefix_width
    }
}

pub(super) fn render_source_choices(
    ui: &mut egui::Ui,
    source_choices: &[SourceChoiceRow],
    popup_busy: bool,
    action: &mut Option<Step2Action>,
) -> SourceChoiceLayout {
    render_section_header(ui, "Source Choices");
    let layout = source_choice_layout(ui, source_choices);
    egui::Grid::new("step2-update-source-choices")
        .num_columns(5)
        .spacing([UPDATE_CHECK_GRID_SPACING_X, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for choice in source_choices {
                let mut selected_source_id = choice.selected_source_id.clone();
                ui.label(&choice.label);
                ui.add_enabled_ui(!popup_busy, |ui| {
                    egui::ComboBox::from_id_salt(format!("step2-source-{}", choice.tp2_key))
                        .selected_text(&choice.selected_label)
                        .width(layout.dropdown_width)
                        .show_ui(ui, |ui| {
                            for option in &choice.options {
                                ui.selectable_value(
                                    &mut selected_source_id,
                                    option.source_id.clone(),
                                    &option.label,
                                );
                            }
                        });
                });
                if ui
                    .add_enabled(!popup_busy, egui::Button::new("Edit Source"))
                    .clicked()
                    && action.is_none()
                {
                    *action = Some(Step2Action::OpenModDownloadSourceEditor {
                        tp2: choice.tp2_key.clone(),
                        label: choice.label.clone(),
                        source_id: choice.selected_source_id.clone(),
                        allow_source_id_change: false,
                    });
                }
                if ui
                    .add_enabled(
                        !popup_busy && choice.selected_source_url.is_some(),
                        egui::Button::new("Open Source"),
                    )
                    .clicked()
                    && action.is_none()
                    && let Some(url) = choice.selected_source_url.as_ref()
                {
                    *action = Some(Step2Action::OpenSelectedWeb(url.clone()));
                }
                if ui
                    .add_enabled(
                        !popup_busy && choice.selected_source_repo.is_some(),
                        egui::Button::new("Discover Forks"),
                    )
                    .clicked()
                    && action.is_none()
                    && let Some(repo) = choice.selected_source_repo.as_ref()
                {
                    *action = Some(Step2Action::DiscoverModDownloadForks {
                        tp2: choice.tp2_key.clone(),
                        label: choice.label.clone(),
                        repo: repo.clone(),
                    });
                }
                ui.end_row();

                if selected_source_id != choice.selected_source_id && action.is_none() {
                    *action = Some(Step2Action::SetModDownloadSource {
                        tp2: choice.tp2_key.clone(),
                        source_id: selected_source_id,
                    });
                }
            }
        });
    layout
}

fn source_choice_layout(ui: &egui::Ui, source_choices: &[SourceChoiceRow]) -> SourceChoiceLayout {
    let dropdown_width = source_choice_dropdown_width(ui, source_choices);
    let label_width = widest_source_choice_label_width(ui, source_choices);
    SourceChoiceLayout {
        list_prefix_width: label_width + dropdown_width + UPDATE_CHECK_GRID_SPACING_X,
        dropdown_width,
    }
}

fn widest_source_choice_label_width(ui: &egui::Ui, source_choices: &[SourceChoiceRow]) -> f32 {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let text_color = redesign_text_primary(ThemePalette::Dark);
    source_choices
        .iter()
        .map(|choice| measured_text_width(ui, &choice.label, &font_id, text_color))
        .fold(0.0, f32::max)
}

fn source_choice_dropdown_width(ui: &egui::Ui, source_choices: &[SourceChoiceRow]) -> f32 {
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let text_color = redesign_text_primary(ThemePalette::Dark);
    source_choices
        .iter()
        .flat_map(|choice| {
            std::iter::once(choice.selected_label.as_str())
                .chain(choice.options.iter().map(|option| option.label.as_str()))
        })
        .map(|label| measured_text_width(ui, label, &font_id, text_color))
        .fold(160.0, f32::max)
        + 48.0
}

fn measured_text_width(
    ui: &egui::Ui,
    text: &str,
    font_id: &egui::FontId,
    text_color: egui::Color32,
) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_string(), font_id.clone(), text_color)
        .size()
        .x
}

pub(super) fn render_list(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    prefix_width: Option<f32>,
    action: &mut Option<Step2Action>,
) {
    render_section_header(ui, title);
    let add_mod = title == "No Source Entries" || title == "Missing";
    if values.is_empty() {
        ui.label("None");
    } else {
        egui::Grid::new(format!("step2-update-list-{title}"))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                let mut sorted_values = values.iter().collect::<Vec<_>>();
                sorted_values.sort_by_key(|value| value.to_ascii_lowercase());
                for value in sorted_values {
                    if let Some(width) = prefix_width {
                        ui.scope(|ui| {
                            ui.set_min_width(width);
                            ui.label(value);
                        });
                    } else {
                        ui.label(value);
                    }
                    if let Some(row) = source_edit_rows
                        .iter()
                        .find(|row| source_edit_row_matches_value(row, value))
                    {
                        if ui
                            .add_enabled(
                                !popup_busy,
                                egui::Button::new(if add_mod { "Add Mod" } else { "Edit Source" }),
                            )
                            .clicked()
                            && action.is_none()
                        {
                            *action = Some(Step2Action::OpenModDownloadSourceEditor {
                                tp2: row.tp2.clone(),
                                label: row.label.clone(),
                                source_id: row.source_id.clone(),
                                allow_source_id_change: add_mod,
                            });
                        }
                    } else {
                        ui.label("");
                    }
                    ui.end_row();
                }
            });
    }
}

pub(super) fn render_section_header(ui: &mut egui::Ui, title: &str) {
    let text = crate::ui::shared::typography_global::strong(title)
        .color(redesign_text_primary(ThemePalette::Dark));
    egui::Frame::group(ui.style())
        .fill(redesign_compat_info_fill(ThemePalette::Dark))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            crate::ui::shared::redesign_tokens::redesign_border_soft(ThemePalette::Dark),
        ))
        .inner_margin(egui::Margin {
            left: 8,
            right: 8,
            top: 4,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.label(text);
        });
}

#[derive(Debug, Clone)]
pub(super) struct SourceEditRow {
    tp2: String,
    label: String,
    source_id: String,
}

pub(super) fn collect_source_edit_rows(state: &WizardState) -> Vec<SourceEditRow> {
    let mut rows = Vec::<SourceEditRow>::new();
    for pending in &state.step2.log_pending_downloads {
        push_source_edit_row(state, &mut rows, &pending.tp_file, &pending.label);
    }
    for asset in &state.step2.update_selected_update_assets {
        push_source_edit_row(state, &mut rows, &asset.tp_file, &asset.label);
    }
    for mod_state in state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
    {
        let label = if mod_state.name.trim().is_empty() {
            mod_state.tp_file.as_str()
        } else {
            mod_state.name.as_str()
        };
        push_source_edit_row(state, &mut rows, &mod_state.tp_file, label);
    }
    rows
}

fn push_source_edit_row(
    state: &WizardState,
    rows: &mut Vec<SourceEditRow>,
    tp2: &str,
    label: &str,
) {
    let key = mod_downloads::normalize_mod_download_tp2(tp2);
    if key.is_empty()
        || rows
            .iter()
            .any(|row| mod_downloads::normalize_mod_download_tp2(&row.tp2) == key)
    {
        return;
    }
    rows.push(SourceEditRow {
        tp2: tp2.to_string(),
        label: label.to_string(),
        source_id: state
            .step2
            .selected_source_ids
            .get(&key)
            .cloned()
            .unwrap_or_else(|| "primary".to_string()),
    });
}

fn source_edit_row_matches_value(row: &SourceEditRow, value: &str) -> bool {
    let value = value.trim();
    let label = row.label.trim();
    value.eq_ignore_ascii_case(label)
        || value
            .to_ascii_lowercase()
            .starts_with(&format!("{} (", label.to_ascii_lowercase()))
        || value
            .to_ascii_lowercase()
            .starts_with(&format!("{}:", label.to_ascii_lowercase()))
}

pub(super) fn pending_log_labels(state: &WizardState) -> Vec<String> {
    state
        .step2
        .log_pending_downloads
        .iter()
        .map(|pending| pending.label.clone())
        .collect()
}

#[derive(Debug, Clone)]
pub(super) struct SourceChoiceRow {
    pub(super) tp2_key: String,
    pub(super) label: String,
    selected_source_id: String,
    selected_label: String,
    selected_source_url: Option<String>,
    selected_source_repo: Option<String>,
    options: Vec<SourceChoiceOption>,
}

#[derive(Debug, Clone)]
struct SourceChoiceOption {
    source_id: String,
    label: String,
}

pub(super) fn collect_source_choices(
    state: &WizardState,
    source_load: &mod_downloads::ModDownloadsLoad,
) -> Vec<SourceChoiceRow> {
    let mut targets = Vec::<(String, String)>::new();
    if let Some((_, tp_file)) = single_mod_popup_target(state) {
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&tp_file);
        if !tp2_key.is_empty() {
            targets.push((tp2_key, tp_file));
        }
    } else if state.step1.installs_exactly_from_weidu_logs() {
        for pending in &state.step2.log_pending_downloads {
            let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
            if !tp2_key.is_empty() && !targets.iter().any(|(key, _)| key == &tp2_key) {
                targets.push((tp2_key, pending.label.clone()));
            }
        }
    } else {
        if state.step1.bootstraps_from_weidu_logs() {
            for pending in &state.step2.log_pending_downloads {
                let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
                if !tp2_key.is_empty() && !targets.iter().any(|(key, _)| key == &tp2_key) {
                    targets.push((tp2_key, pending.label.clone()));
                }
            }
        }
        for mod_state in state
            .step2
            .bgee_mods
            .iter()
            .chain(state.step2.bg2ee_mods.iter())
        {
            if !mod_state.checked
                && !mod_state
                    .components
                    .iter()
                    .any(|component| component.checked)
            {
                continue;
            }
            let tp2_key = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
            if tp2_key.is_empty() || targets.iter().any(|(key, _)| key == &tp2_key) {
                continue;
            }
            targets.push((
                tp2_key,
                if mod_state.name.trim().is_empty() {
                    mod_state.tp_file.clone()
                } else {
                    mod_state.name.clone()
                },
            ));
        }
    }

    let mut rows = Vec::<SourceChoiceRow>::new();
    for (tp2_key, label) in targets {
        let sources = source_load.find_sources(&tp2_key);
        if sources.is_empty() {
            continue;
        }
        let selected_source = source_load
            .resolve_source(
                &tp2_key,
                state
                    .step2
                    .selected_source_ids
                    .get(&tp2_key)
                    .map(String::as_str),
            )
            .unwrap_or_else(|| sources[0].clone());
        let selected_source_url = source_open_url(&selected_source);
        let selected_source_repo = selected_source.github.clone();
        rows.push(SourceChoiceRow {
            tp2_key,
            label,
            selected_source_id: selected_source.source_id.clone(),
            selected_label: selected_source.source_label.clone(),
            selected_source_url,
            selected_source_repo,
            options: sources
                .into_iter()
                .map(|source| SourceChoiceOption {
                    source_id: source.source_id,
                    label: source.source_label,
                })
                .collect(),
        });
    }
    rows.sort_by_key(|row| row.label.to_ascii_lowercase());
    rows
}

fn source_open_url(source: &mod_downloads::ModDownloadSource) -> Option<String> {
    let url = source.url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        return Some(url.to_string());
    }
    let github = source.github.as_deref()?.trim();
    if github.starts_with("http://") || github.starts_with("https://") {
        return Some(github.to_string());
    }
    let repo = github.trim_matches('/');
    let mut parts = repo.split('/');
    if matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty()
    ) {
        return Some(format!("https://github.com/{repo}"));
    }
    None
}

pub(super) fn single_mod_popup_target(state: &WizardState) -> Option<(String, String)> {
    match (
        state.step2.update_selected_target_game_tab.as_ref(),
        state.step2.update_selected_target_tp_file.as_ref(),
    ) {
        (Some(game_tab), Some(tp_file))
            if !game_tab.trim().is_empty() && !tp_file.trim().is_empty() =>
        {
            Some((game_tab.clone(), tp_file.clone()))
        }
        _ => None,
    }
}
