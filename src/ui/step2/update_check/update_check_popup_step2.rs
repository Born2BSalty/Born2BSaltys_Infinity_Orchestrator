// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::{WizardState, exact_log_ready_to_install, update_selection_signature};
use crate::ui::orchestrator::widgets::apply_primary_button_visuals;
use crate::ui::shared::redesign_tokens::ThemePalette;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::review_edit_any_log_applied;
use crate::ui::step2::update_check_popup_lists_step2::{
    SourceChoiceRow, SourceEditRow, collect_source_choices, collect_source_edit_rows,
    pending_log_labels, render_list, render_section_header, render_source_choices,
    single_mod_popup_target,
};
use crate::ui::step2::update_check_popup_report_step2::build_popup_report;
use crate::ui::step2::update_check_popup_source_editor_step2::render_source_editor_popup;

#[derive(Clone, Copy)]
struct PopupModes<Flag = bool> {
    exact_log: Flag,
    review_edit: Flag,
    scanned_mods: Flag,
    pending_missing: Flag,
    hybrid_missing: Flag,
    hybrid_source_pending: Flag,
    selection_stale: Flag,
    good_to_go: Flag,
    retry_latest: Flag,
    busy: Flag,
}

struct PopupResources<'a> {
    single_mod_target: Option<&'a (String, String)>,
    source_choices: &'a [SourceChoiceRow],
    source_edit_rows: &'a [SourceEditRow],
}

pub fn render(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
    if !state.step2.update_selected_popup_open {
        return;
    }

    let single_mod_popup_target = single_mod_popup_target(state);
    let modes = popup_modes(state, single_mod_popup_target.is_some());
    let source_load = mod_downloads::load_mod_download_sources();
    let source_choices = collect_source_choices(state, &source_load);
    let source_edit_rows = collect_source_edit_rows(state);
    let resources = PopupResources {
        single_mod_target: single_mod_popup_target.as_ref(),
        source_choices: &source_choices,
        source_edit_rows: &source_edit_rows,
    };
    let mut open = state.step2.update_selected_popup_open;
    render_main_popup(ctx, state, action, modes, &resources, &mut open, palette);
    state.step2.update_selected_popup_open = open && state.step2.update_selected_popup_open;

    render_latest_fallback_confirm(ctx, state, action, palette);
    render_source_editor_popup(ctx, state, action, palette);
    render_forks_popup(ctx, state, action);
}

fn popup_modes(state: &WizardState, has_single_mod_target: bool) -> PopupModes {
    let exact_log = state.step1.installs_exactly_from_weidu_logs();
    let review_edit = state.step1.bootstraps_from_weidu_logs();
    let scanned_mods = !state.step1.uses_source_weidu_logs();
    let pending_missing = review_edit && !state.step2.log_pending_downloads.is_empty();
    let hybrid_missing = review_edit && pending_missing;
    let current_selection_signature =
        scanned_mods.then(|| update_selection_signature(&state.step2));
    let selection_stale = scanned_mods
        && !has_single_mod_target
        && state.step2.update_selected_has_run
        && (!state.step2.update_selected_last_was_full_selection
            || state
                .step2
                .update_selected_last_selection_signature
                .as_deref()
                != current_selection_signature.as_deref());
    PopupModes {
        exact_log,
        review_edit,
        scanned_mods,
        pending_missing,
        hybrid_missing,
        hybrid_source_pending: hybrid_missing && !state.step2.update_selected_has_run,
        selection_stale,
        good_to_go: exact_log_ready_to_install(state),
        retry_latest: can_retry_latest(state, exact_log),
        busy: popup_busy(state),
    }
}

const fn can_retry_latest(state: &WizardState, exact_log: bool) -> bool {
    exact_log
        && !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
        && !update_pipeline_busy(state)
}

const fn popup_busy(state: &WizardState) -> bool {
    state.step2.is_scanning || update_pipeline_busy(state)
}

const fn update_pipeline_busy(state: &WizardState) -> bool {
    state.step2.update_selected_check_running
        || state.step2.update_selected_download_running
        || state.step2.update_selected_extract_running
}

fn render_main_popup(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    open: &mut bool,
    palette: ThemePalette,
) {
    egui::Window::new(popup_title(modes))
        .open(open)
        .collapsible(true)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(560.0, 320.0))
        .min_width(320.0)
        .min_height(180.0)
        .show(ctx, |ui| {
            render_popup_body(ui, state, action, modes, resources, palette);
        });
}

const fn popup_title(modes: PopupModes) -> &'static str {
    if modes.exact_log {
        "Check Mod List"
    } else {
        "Check Updates"
    }
}

fn render_popup_body(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    palette: ThemePalette,
) {
    let content_height = (ui.available_height() - 40.0).max(80.0);
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_width(ui.available_width())
        .max_height(content_height)
        .show(ui, |ui| {
            render_popup_scroll(ui, state, action, modes, resources);
        });
    ui.add_space(8.0);
    render_footer(ui, state, action, modes, resources, palette);
}

fn render_popup_scroll(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
) {
    render_status_lines(ui, state, modes);
    render_summary(ui, state, modes);
    render_selection_stale_message(ui, modes);
    let source_choice_prefix_width = render_source_choice_area(ui, modes, resources, action);
    render_primary_list(
        ui,
        state,
        action,
        modes,
        resources,
        source_choice_prefix_width,
    );
    render_mode_lists(
        ui,
        state,
        action,
        modes,
        resources,
        source_choice_prefix_width,
    );
    render_result_lists(
        ui,
        state,
        action,
        modes,
        resources,
        source_choice_prefix_width,
    );
}

fn render_status_lines(ui: &mut egui::Ui, state: &WizardState, modes: PopupModes) {
    if state.step2.update_selected_check_running {
        ui.label(if modes.exact_log {
            format!(
                "Checking missing mod sources {}/{}",
                state.step2.update_selected_check_done_count,
                state.step2.update_selected_check_total_count
            )
        } else if modes.hybrid_missing {
            format!(
                "Checking updates / missing mod sources {}/{}",
                state.step2.update_selected_check_done_count,
                state.step2.update_selected_check_total_count
            )
        } else {
            format!(
                "Checking {}/{}",
                state.step2.update_selected_check_done_count,
                state.step2.update_selected_check_total_count
            )
        });
    }
    if state.step2.update_selected_download_running {
        ui.label(download_status_label(state, modes));
    }
    if state.step2.update_selected_extract_running {
        ui.label(extract_status_label(state, modes));
    }
}

const fn download_status_label(state: &WizardState, modes: PopupModes) -> &'static str {
    if modes.exact_log {
        "Downloading missing mod archives..."
    } else if modes.hybrid_missing && !state.step2.update_selected_update_sources.is_empty() {
        "Downloading missing mod / update archives..."
    } else if modes.hybrid_missing {
        "Downloading missing mod archives..."
    } else {
        "Downloading update archives..."
    }
}

const fn extract_status_label(state: &WizardState, modes: PopupModes) -> &'static str {
    if modes.exact_log {
        "Extracting downloaded missing mods..."
    } else if modes.hybrid_missing && !state.step2.update_selected_update_sources.is_empty() {
        "Extracting downloaded missing mods / updates..."
    } else if modes.hybrid_missing {
        "Extracting downloaded missing mods..."
    } else {
        "Extracting downloaded updates..."
    }
}

fn render_summary(ui: &mut egui::Ui, state: &WizardState, modes: PopupModes) {
    if modes.scanned_mods && !state.step2.update_selected_has_run && !update_pipeline_busy(state) {
        ui.add_space(4.0);
        ui.label("No update check run yet.");
    } else if modes.good_to_go {
        ui.add_space(4.0);
        ui.label("No missing mods found. Exact-log install is good to go.");
    } else if modes.exact_log {
        render_exact_log_summary(ui, state);
    } else if modes.hybrid_source_pending {
        render_hybrid_pending_summary(ui, state);
    } else if modes.hybrid_missing {
        render_hybrid_summary(ui, state);
    } else {
        render_update_summary(ui, state);
    }
}

fn render_exact_log_summary(ui: &mut egui::Ui, state: &WizardState) {
    let missing_count = state.step2.update_selected_known_sources.len()
        + state.step2.update_selected_manual_sources.len()
        + state.step2.update_selected_unknown_sources.len();
    ui.label(format!("Missing mods: {missing_count}"));
    ui.label(format!(
        "Downloadable missing mods: {}",
        state.step2.update_selected_missing_sources.len()
    ));
    ui.label(format!(
        "Auto sources: {}",
        state.step2.update_selected_known_sources.len()
    ));
    ui.label(format!(
        "Manual sources: {}",
        state.step2.update_selected_manual_sources.len()
    ));
    ui.label(format!(
        "No source entries: {}",
        state.step2.update_selected_unknown_sources.len()
    ));
    ui.label(format!(
        "Exact version not available: {}",
        state
            .step2
            .update_selected_exact_version_failed_sources
            .len()
    ));
}

fn render_hybrid_pending_summary(ui: &mut egui::Ui, state: &WizardState) {
    ui.label(format!(
        "Missing mods from applied log: {}",
        state.step2.log_pending_downloads.len()
    ));
    ui.label("No source check run yet.");
}

fn render_hybrid_summary(ui: &mut egui::Ui, state: &WizardState) {
    ui.label(format!(
        "Updates found: {}",
        state.step2.update_selected_update_sources.len()
    ));
    ui.label(format!(
        "Missing mods: {}",
        state.step2.log_pending_downloads.len()
    ));
    ui.label(format!(
        "Downloadable missing mods: {}",
        state.step2.update_selected_missing_sources.len()
    ));
    ui.label(format!(
        "Auto sources checked: {}",
        state.step2.update_selected_known_sources.len()
    ));
    ui.label(format!(
        "Manual sources: {}",
        state.step2.update_selected_manual_sources.len()
    ));
    ui.label(format!(
        "No source entries: {}",
        state.step2.update_selected_unknown_sources.len()
    ));
    ui.label(format!(
        "Exact version not available: {}",
        state
            .step2
            .update_selected_exact_version_failed_sources
            .len()
    ));
}

fn render_update_summary(ui: &mut egui::Ui, state: &WizardState) {
    ui.label(format!(
        "Updates found: {}",
        state.step2.update_selected_update_sources.len()
    ));
    ui.label(format!(
        "Auto sources checked: {}",
        state.step2.update_selected_known_sources.len()
    ));
    ui.label(format!(
        "Manual sources: {}",
        state.step2.update_selected_manual_sources.len()
    ));
    ui.label(format!(
        "Missing sources: {}",
        state.step2.update_selected_unknown_sources.len()
    ));
}

fn render_selection_stale_message(ui: &mut egui::Ui, modes: PopupModes) {
    if modes.selection_stale && !modes.busy {
        ui.add_space(4.0);
        ui.label("Current selection differs from the last check. Run Check Updates again.");
    }
}

fn render_source_choice_area(
    ui: &mut egui::Ui,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    action: &mut Option<Step2Action>,
) -> Option<f32> {
    if resources.source_choices.is_empty() {
        return None;
    }
    ui.add_space(8.0);
    Some(
        render_source_choices(ui, resources.source_choices, modes.busy, action).list_prefix_width(),
    )
}

fn render_primary_list(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    let pending_labels = modes
        .hybrid_source_pending
        .then(|| pending_log_labels(state));
    let (title, values) = primary_list_values(state, modes, pending_labels.as_deref());
    render_list(
        ui,
        title,
        values,
        resources.source_edit_rows,
        modes.busy,
        prefix_width,
        action,
    );
}

fn primary_list_values<'a>(
    state: &'a WizardState,
    modes: PopupModes,
    pending_labels: Option<&'a [String]>,
) -> (&'static str, &'a [String]) {
    if modes.hybrid_source_pending {
        (
            "Missing Mods From Applied Log",
            pending_labels.unwrap_or(&[]),
        )
    } else if modes.exact_log || modes.hybrid_missing {
        (
            "Downloadable Missing Mods",
            &state.step2.update_selected_missing_sources,
        )
    } else {
        ("Updates", &state.step2.update_selected_update_sources)
    }
}

fn render_mode_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    if modes.exact_log {
        render_exact_log_lists(ui, state, action, modes, resources, prefix_width);
    } else if modes.hybrid_missing {
        render_hybrid_lists(ui, state, action, modes, resources, prefix_width);
    } else {
        render_update_lists(ui, state, action, modes, resources, prefix_width);
    }
}

fn render_exact_log_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    render_spaced_list(
        ui,
        "Auto Sources",
        &state.step2.update_selected_known_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_spaced_list(
        ui,
        "Manual Sources",
        &state.step2.update_selected_manual_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_spaced_list(
        ui,
        "No Source Entries",
        &state.step2.update_selected_unknown_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
}

fn render_hybrid_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    render_non_empty_spaced_list(
        ui,
        "Updates",
        &state.step2.update_selected_update_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Manual Sources",
        &state.step2.update_selected_manual_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "No Source Entries",
        &state.step2.update_selected_unknown_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
}

fn render_update_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    render_non_empty_spaced_list(
        ui,
        "Manual",
        &state.step2.update_selected_manual_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Missing",
        &state.step2.update_selected_unknown_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
}

fn render_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    prefix_width: Option<f32>,
) {
    if modes.exact_log || modes.hybrid_missing {
        render_non_empty_spaced_list(
            ui,
            "Exact Version Not Available",
            &state.step2.update_selected_exact_version_failed_sources,
            resources,
            modes,
            prefix_width,
            action,
        );
    }
    render_non_empty_spaced_list(
        ui,
        if modes.exact_log || modes.hybrid_missing {
            "Source Check Failed"
        } else {
            "Failed"
        },
        &state.step2.update_selected_failed_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Downloaded",
        &state.step2.update_selected_downloaded_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Download Failed",
        &state.step2.update_selected_download_failed_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Extracted",
        &state.step2.update_selected_extracted_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
    render_non_empty_spaced_list(
        ui,
        "Extract Failed",
        &state.step2.update_selected_extract_failed_sources,
        resources,
        modes,
        prefix_width,
        action,
    );
}

fn render_non_empty_spaced_list(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    resources: &PopupResources<'_>,
    modes: PopupModes,
    prefix_width: Option<f32>,
    action: &mut Option<Step2Action>,
) {
    if !values.is_empty() {
        render_spaced_list(ui, title, values, resources, modes, prefix_width, action);
    }
}

fn render_spaced_list(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    resources: &PopupResources<'_>,
    modes: PopupModes,
    prefix_width: Option<f32>,
    action: &mut Option<Step2Action>,
) {
    ui.add_space(8.0);
    render_list(
        ui,
        title,
        values,
        resources.source_edit_rows,
        modes.busy,
        prefix_width,
        action,
    );
}

fn render_footer(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    palette: ThemePalette,
) {
    ui.horizontal_wrapped(|ui| {
        render_check_button(ui, state, action, modes, resources, palette);
        render_add_source_button(ui, action, resources);
        render_copy_report_button(ui, state, modes);
        render_download_button(ui, state, action, modes, palette);
        render_latest_retry_button(ui, state, modes);
        render_close_button(ui, state);
    });
}

fn render_check_button(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    resources: &PopupResources<'_>,
    palette: ThemePalette,
) {
    ui.scope(|ui| {
        apply_primary_button_visuals(ui, palette);
        if ui
            .add_enabled(
                can_check_updates(state, modes),
                egui::Button::new(popup_title(modes)),
            )
            .clicked()
        {
            *action = Some(check_action(modes, resources));
        }
    });
}

fn can_check_updates(state: &WizardState, modes: PopupModes) -> bool {
    let has_any_checked = has_any_checked(state);
    if modes.exact_log {
        !modes.busy
    } else if modes.review_edit {
        review_edit_any_log_applied(state)
            && (has_any_checked || modes.pending_missing)
            && !modes.busy
    } else {
        modes.scanned_mods && has_any_checked && !modes.busy
    }
}

fn has_any_checked(state: &WizardState) -> bool {
    state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .any(|mod_state| {
            mod_state.checked
                || mod_state
                    .components
                    .iter()
                    .any(|component| component.checked)
        })
}

const fn check_action(modes: PopupModes, resources: &PopupResources<'_>) -> Step2Action {
    if modes.exact_log {
        Step2Action::CheckExactLogModList
    } else if resources.single_mod_target.is_some() {
        Step2Action::PreviewUpdateSelectedMod
    } else {
        Step2Action::PreviewUpdateSelected
    }
}

fn render_add_source_button(
    ui: &mut egui::Ui,
    action: &mut Option<Step2Action>,
    resources: &PopupResources<'_>,
) {
    if ui.button("Add Source").clicked() && action.is_none() {
        let (tp2, label) = add_source_target(resources)
            .unwrap_or_else(|| ("newmod".to_string(), "New Mod".to_string()));
        *action = Some(Step2Action::OpenModDownloadSourceEditor {
            tp2,
            label,
            source_id: "new-source".to_string(),
            allow_source_id_change: true,
        });
    }
}

fn add_source_target(resources: &PopupResources<'_>) -> Option<(String, String)> {
    resources
        .single_mod_target
        .map(|(_, tp_file)| {
            (
                mod_downloads::normalize_mod_download_tp2(tp_file),
                tp_file.clone(),
            )
        })
        .or_else(|| {
            (resources.source_choices.len() == 1).then(|| {
                (
                    resources.source_choices[0].tp2_key.clone(),
                    resources.source_choices[0].label.clone(),
                )
            })
        })
}

fn render_copy_report_button(ui: &mut egui::Ui, state: &WizardState, modes: PopupModes) {
    let can_copy_report = !modes.scanned_mods || state.step2.update_selected_has_run;
    if ui
        .add_enabled(can_copy_report, egui::Button::new("Copy Report"))
        .clicked()
    {
        ui.ctx()
            .copy_text(build_popup_report(state, modes.exact_log, modes.good_to_go));
    }
}

fn render_download_button(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    modes: PopupModes,
    palette: ThemePalette,
) {
    ui.scope(|ui| {
        apply_primary_button_visuals(ui, palette);
        if ui
            .add_enabled(
                can_download_updates(state),
                egui::Button::new(download_button_label(state, modes)),
            )
            .clicked()
        {
            *action = Some(Step2Action::DownloadUpdates);
        }
    });
}

const fn can_download_updates(state: &WizardState) -> bool {
    !state.step2.update_selected_update_assets.is_empty() && !update_pipeline_busy(state)
}

const fn download_button_label(state: &WizardState, modes: PopupModes) -> &'static str {
    if modes.exact_log {
        "Download Missing Mods"
    } else if modes.hybrid_missing
        && !state.step2.update_selected_missing_sources.is_empty()
        && !state.step2.update_selected_update_sources.is_empty()
    {
        "Download Missing Mods / Updates"
    } else if modes.hybrid_missing && !state.step2.update_selected_missing_sources.is_empty() {
        "Download Missing Mods"
    } else {
        "Download Updates"
    }
}

fn render_latest_retry_button(ui: &mut egui::Ui, state: &mut WizardState, modes: PopupModes) {
    if modes.exact_log
        && !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
        && ui
            .add_enabled(
                modes.retry_latest,
                egui::Button::new("Use Latest For Exact-Version Misses"),
            )
            .clicked()
    {
        state.step2.update_selected_confirm_latest_fallback_open = true;
    }
}

fn render_close_button(ui: &mut egui::Ui, state: &mut WizardState) {
    if ui.button("Close").clicked() {
        state.step2.update_selected_popup_open = false;
        state.step2.update_selected_confirm_latest_fallback_open = false;
    }
}

fn render_latest_fallback_confirm(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    palette: ThemePalette,
) {
    if !state.step2.update_selected_confirm_latest_fallback_open {
        return;
    }
    let mut confirm_open = true;
    egui::Window::new("Download Latest Instead?")
        .open(&mut confirm_open)
        .collapsible(true)
        .resizable(false)
        .movable(true)
        .default_size(egui::vec2(360.0, 120.0))
        .show(ctx, |ui| {
            ui.label(format!(
                "Exact version unavailable for {} mods.",
                state
                    .step2
                    .update_selected_exact_version_retry_requests
                    .len()
            ));
            ui.label("Download latest instead for those mods only?");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.scope(|ui| {
                    apply_primary_button_visuals(ui, palette);
                    if ui.button("Yes").clicked() {
                        state.step2.update_selected_confirm_latest_fallback_open = false;
                        *action = Some(Step2Action::AcceptLatestForExactVersionMisses);
                    }
                });
                if ui.button("No").clicked() {
                    state.step2.update_selected_confirm_latest_fallback_open = false;
                }
            });
        });
    state.step2.update_selected_confirm_latest_fallback_open =
        confirm_open && state.step2.update_selected_confirm_latest_fallback_open;
}

fn render_forks_popup(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
) {
    if !state.step2.mod_download_forks_popup_open {
        return;
    }
    let mut open = state.step2.mod_download_forks_popup_open;
    egui::Window::new(state.step2.mod_download_forks_popup_title.clone())
        .open(&mut open)
        .collapsible(true)
        .resizable(true)
        .movable(true)
        .default_size(egui::vec2(620.0, 420.0))
        .show(ctx, |ui| {
            render_section_header(ui, &state.step2.mod_download_forks_popup_label);
            ui.add_space(8.0);
            if let Some(err) = state.step2.mod_download_forks_popup_error.as_ref() {
                ui.label(err);
                ui.add_space(8.0);
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("step2-discovered-forks")
                        .num_columns(5)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(crate::ui::shared::typography_global::strong("Repo"));
                            ui.label(crate::ui::shared::typography_global::strong("Branch"));
                            ui.label(crate::ui::shared::typography_global::strong("Updated"));
                            ui.label("");
                            ui.label("");
                            ui.end_row();
                            for fork in &state.step2.mod_download_forks {
                                let updated_date = fork
                                    .updated_at
                                    .split('T')
                                    .next()
                                    .unwrap_or(&fork.updated_at);
                                ui.label(&fork.full_name);
                                ui.label(&fork.default_branch);
                                ui.label(updated_date);
                                if ui.button("Open").clicked() && action.is_none() {
                                    *action =
                                        Some(Step2Action::OpenSelectedWeb(fork.html_url.clone()));
                                }
                                if ui.button("Add Source").clicked() && action.is_none() {
                                    *action = Some(Step2Action::AddDiscoveredModDownloadFork {
                                        tp2: state.step2.mod_download_forks_popup_tp2.clone(),
                                        label: state.step2.mod_download_forks_popup_label.clone(),
                                        full_name: fork.full_name.clone(),
                                        owner_login: fork.owner_login.clone(),
                                        default_branch: fork.default_branch.clone(),
                                    });
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    state.step2.mod_download_forks_popup_open = open;
}
