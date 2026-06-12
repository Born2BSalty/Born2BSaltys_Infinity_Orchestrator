// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::WizardState;
use crate::app::step2_action::ModSourceEditDestination;
use crate::ui::orchestrator::widgets::{BtnOpts, redesign_btn, redesign_section_header};
use crate::ui::shared::redesign_tokens::{
    REDESIGN_BORDER_RADIUS_U8, REDESIGN_BORDER_WIDTH_PX, ThemePalette, redesign_border_strong,
    redesign_shell_bg,
};
use crate::ui::shared::{theme_global, typography_global};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::applied_weidu_log_has_pending_downloads;

const UPDATE_CHECK_GRID_SPACING_X: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct SourceChoiceLayout {
    list_prefix_width: f32,
    dropdown_width: f32,
}

impl SourceChoiceLayout {
    pub(super) const fn list_prefix_width(self) -> f32 {
        self.list_prefix_width
    }
}

/// Context bundle for list rendering helpers to stay within argument-count limits.
pub(super) struct ListCtx<'a> {
    pub(super) palette: ThemePalette,
    pub(super) source_edit_rows: &'a [SourceEditRow],
    pub(super) popup_busy: bool,
    pub(super) prefix_width: Option<f32>,
    pub(super) action: &'a mut Option<Step2Action>,
}

pub(super) fn render_source_choices(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    source_choices: &[SourceChoiceRow],
    popup_busy: bool,
    action: &mut Option<Step2Action>,
) -> SourceChoiceLayout {
    let count = source_choices.len();
    redesign_section_header(ui, palette, "Source Choices", Some(count));
    ui.add_space(8.0);
    let layout = source_choice_layout(ui, source_choices);
    egui::Frame::group(ui.style())
        .fill(redesign_shell_bg(palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            egui::Grid::new("step2-update-source-choices")
                .num_columns(6)
                .spacing([UPDATE_CHECK_GRID_SPACING_X, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    for choice in source_choices {
                        render_source_choice_row(ui, palette, choice, popup_busy, layout, action);
                    }
                });
        });
    ui.add_space(8.0);
    layout
}

/// Renders the destination dropdown + "Edit Source" button in one horizontal cell.
/// Returns the `OpenModDownloadSourceEditor` action if the button was clicked.
fn render_edit_source_cell(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    choice: &SourceChoiceRow,
    popup_busy: bool,
) -> Option<Step2Action> {
    let edit_enabled = !popup_busy;
    let make_action = |destination| Step2Action::OpenModDownloadSourceEditor {
        tp2: choice.tp2_key.clone(),
        label: choice.label.clone(),
        source_id: choice.selected_source_id.clone(),
        allow_source_id_change: false,
        destination,
    };

    let btn = redesign_btn(
        ui,
        palette,
        "Set Source",
        BtnOpts {
            small: true,
            disabled: !edit_enabled,
            ..Default::default()
        },
    );
    if !edit_enabled {
        return None;
    }

    // With an active modlist the button opens a popup offering both destinations
    // (choosing one opens the editor against that file); without one (legacy), it
    // opens the editor against the global file directly.
    if mod_downloads::active_modlist_downloads_path().is_some() {
        let popup_id = ui.make_persistent_id(format!("step2-set-source-{}", choice.tp2_key));
        if btn.clicked() {
            ui.memory_mut(|m| m.toggle_popup(popup_id));
        }
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &btn,
            egui::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_min_width(150.0);
                let mut chosen = None;
                if ui
                    .selectable_label(
                        false,
                        destination_label(ModSourceEditDestination::GlobalDefault),
                    )
                    .clicked()
                {
                    chosen = Some(make_action(ModSourceEditDestination::GlobalDefault));
                    ui.memory_mut(egui::Memory::close_popup);
                }
                if ui
                    .selectable_label(
                        false,
                        destination_label(ModSourceEditDestination::ThisModlist),
                    )
                    .clicked()
                {
                    chosen = Some(make_action(ModSourceEditDestination::ThisModlist));
                    ui.memory_mut(egui::Memory::close_popup);
                }
                chosen
            },
        )
        .flatten()
    } else if btn.clicked() {
        Some(make_action(ModSourceEditDestination::GlobalDefault))
    } else {
        None
    }
}

fn render_source_choice_row(
    ui: &mut egui::Ui,
    palette: ThemePalette,
    choice: &SourceChoiceRow,
    popup_busy: bool,
    layout: SourceChoiceLayout,
    action: &mut Option<Step2Action>,
) {
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
    if let Some(act) = render_edit_source_cell(ui, palette, choice, popup_busy)
        && action.is_none()
    {
        *action = Some(act);
    }
    let open_enabled = !popup_busy && choice.selected_source_url.is_some();
    if redesign_btn(
        ui,
        palette,
        "Open Source",
        BtnOpts {
            small: true,
            disabled: !open_enabled,
            ..Default::default()
        },
    )
    .clicked()
        && open_enabled
        && action.is_none()
        && let Some(url) = choice.selected_source_url.as_ref()
    {
        *action = Some(Step2Action::OpenSelectedWeb(url.clone()));
    }
    let forks_enabled = !popup_busy && choice.selected_source_repo.is_some();
    if redesign_btn(
        ui,
        palette,
        "Discover Forks",
        BtnOpts {
            small: true,
            disabled: !forks_enabled,
            ..Default::default()
        },
    )
    .clicked()
        && forks_enabled
        && action.is_none()
        && let Some(repo) = choice.selected_source_repo.as_ref()
    {
        *action = Some(Step2Action::DiscoverModDownloadForks {
            tp2: choice.tp2_key.clone(),
            label: choice.label.clone(),
            repo: repo.clone(),
        });
    }
    let lock_icon = if choice.update_locked {
        typography_global::strong("🔒").color(theme_global::warning())
    } else {
        typography_global::strong("🔓").color(theme_global::text_disabled())
    };
    let hover_text = if choice.update_locked {
        "Unlock updates"
    } else {
        "Lock updates"
    };
    if ui
        .small_button(lock_icon)
        .on_hover_text(hover_text)
        .clicked()
        && action.is_none()
    {
        *action = Some(Step2Action::SetModUpdateLocked {
            tp2: choice.tp2_key.clone(),
            locked: !choice.update_locked,
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

const fn destination_label(dest: ModSourceEditDestination) -> &'static str {
    match dest {
        ModSourceEditDestination::GlobalDefault => "My default",
        ModSourceEditDestination::ThisModlist => "For this modlist",
    }
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
    let text_color = ui.visuals().text_color();
    source_choices
        .iter()
        .map(|choice| measured_text_width(ui, &choice.label, &font_id, text_color))
        .fold(0.0, f32::max)
}

fn source_choice_dropdown_width(ui: &egui::Ui, source_choices: &[SourceChoiceRow]) -> f32 {
    let font_id = egui::TextStyle::Button.resolve(ui.style());
    let text_color = ui.visuals().text_color();
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
    ctx: &mut ListCtx<'_>,
) {
    let count = values.len();
    redesign_section_header(ui, ctx.palette, title, Some(count));
    ui.add_space(8.0);
    let add_mod = title == "No Source Entries" || title == "Missing";
    egui::Frame::group(ui.style())
        .fill(redesign_shell_bg(ctx.palette))
        .stroke(egui::Stroke::new(
            REDESIGN_BORDER_WIDTH_PX,
            redesign_border_strong(ctx.palette),
        ))
        .corner_radius(egui::CornerRadius::same(REDESIGN_BORDER_RADIUS_U8))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            if values.is_empty() {
                ui.label("None");
            } else {
                render_list_grid(ui, title, values, add_mod, ctx);
            }
        });
    ui.add_space(8.0);
}

fn render_list_grid(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    add_mod: bool,
    ctx: &mut ListCtx<'_>,
) {
    egui::Grid::new(format!("step2-update-list-{title}"))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            let mut sorted_values = values.iter().collect::<Vec<_>>();
            sorted_values.sort_by_key(|value| value.to_ascii_lowercase());
            for value in sorted_values {
                if let Some(width) = ctx.prefix_width {
                    ui.scope(|ui| {
                        ui.set_min_width(width);
                        ui.label(value);
                    });
                } else {
                    ui.label(value);
                }
                if let Some(row) = ctx
                    .source_edit_rows
                    .iter()
                    .find(|row| source_edit_row_matches_value(row, value))
                {
                    let btn_label = if add_mod { "Add Mod" } else { "Edit Source" };
                    let btn_enabled = !ctx.popup_busy;
                    if redesign_btn(
                        ui,
                        ctx.palette,
                        btn_label,
                        BtnOpts {
                            small: true,
                            disabled: !btn_enabled,
                            ..Default::default()
                        },
                    )
                    .clicked()
                        && btn_enabled
                        && ctx.action.is_none()
                    {
                        *ctx.action = Some(Step2Action::OpenModDownloadSourceEditor {
                            tp2: row.tp2.clone(),
                            label: row.label.clone(),
                            source_id: row.source_id.clone(),
                            allow_source_id_change: add_mod,
                            // "Add Mod" and list-grid "Edit Source" are always global-only
                            // (no per-modlist scoping for the add-a-new-source path).
                            destination: ModSourceEditDestination::GlobalDefault,
                        });
                    }
                } else {
                    ui.label("");
                }
                ui.end_row();
            }
        });
}

#[derive(Debug, Clone)]
pub(super) struct SourceEditRow {
    pub(super) tp2: String,
    pub(super) label: String,
    pub(super) source_id: String,
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
    /// Whether the mod's update-lock is active, as read from the scanned mod state.
    update_locked: bool,
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
        if applied_weidu_log_has_pending_downloads(state) {
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
        let update_locked = state
            .step2
            .bgee_mods
            .iter()
            .chain(state.step2.bg2ee_mods.iter())
            .find(|m| mod_downloads::normalize_mod_download_tp2(&m.tp_file) == tp2_key)
            .is_some_and(|m| m.update_locked);
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
            update_locked,
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
