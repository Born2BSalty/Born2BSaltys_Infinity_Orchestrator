// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::{WizardState, exact_log_ready_to_install, update_selection_signature};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::review_edit_any_log_applied;
use crate::ui::step2::update_check_popup_lists_step2::{
    SourceChoiceRow, SourceEditRow, collect_source_choices, collect_source_edit_rows,
    pending_log_labels, render_list, render_section_header, render_source_choices,
    single_mod_popup_target,
};
use crate::ui::step2::update_check_popup_report_step2::build_popup_report;
use crate::ui::step2::update_check_popup_source_editor_step2::render_source_editor_popup;

pub fn render(ctx: &egui::Context, state: &mut WizardState, action: &mut Option<Step2Action>) {
    if !state.step2.update_selected_popup_open {
        return;
    }

    let source_load = mod_downloads::load_mod_download_sources();
    let source_choices = collect_source_choices(state, &source_load);
    let source_edit_rows = collect_source_edit_rows(state);
    let mut open = state.step2.update_selected_popup_open;
    render_update_check_window(
        ctx,
        state,
        action,
        &source_choices,
        &source_edit_rows,
        &mut open,
    );
    state.step2.update_selected_popup_open = open && state.step2.update_selected_popup_open;
    render_latest_fallback_confirm(ctx, state, action);
    render_source_editor_popup(ctx, state, action);
    render_forks_popup(ctx, state, action);
}

fn render_update_check_window(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    source_choices: &[SourceChoiceRow],
    source_edit_rows: &[SourceEditRow],
    open: &mut bool,
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    egui::Window::new(if exact_log_mode {
        "Check Mod List"
    } else {
        "Check Updates"
    })
    .open(open)
    .collapsible(false)
    .resizable(true)
    .movable(true)
    .default_size(egui::vec2(560.0, 320.0))
    .min_width(320.0)
    .min_height(180.0)
    .show(ctx, |ui| {
        let content_height = (ui.available_height() - 40.0).max(80.0);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_width(ui.available_width())
            .max_height(content_height)
            .show(ui, |ui| {
                render_update_scroll_contents(ui, state, action, source_choices, source_edit_rows);
            });
        ui.add_space(8.0);
        render_update_popup_buttons(ui, state, action, source_choices);
    });
}

fn render_update_scroll_contents(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_choices: &[SourceChoiceRow],
    source_edit_rows: &[SourceEditRow],
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let review_edit_mode = state.step1.bootstraps_from_weidu_logs();
    let hybrid_missing_mode = review_edit_mode && !state.step2.log_pending_downloads.is_empty();
    let popup_busy = update_popup_busy(state);
    render_busy_status(ui, state, exact_log_mode, hybrid_missing_mode);
    render_summary_status(ui, state, exact_log_mode, hybrid_missing_mode);
    render_selection_stale_notice(ui, state);
    let source_choice_prefix_width =
        render_source_choice_section(ui, source_choices, popup_busy, action);
    ui.add_space(8.0);
    render_update_result_lists(
        ui,
        state,
        action,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
    );
}

const fn update_popup_busy(state: &WizardState) -> bool {
    state.step2.is_scanning
        || state.step2.update_selected_check_running
        || state.step2.update_selected_download_running
        || state.step2.update_selected_extract_running
}

fn render_busy_status(
    ui: &mut egui::Ui,
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) {
    if state.step2.update_selected_check_running {
        ui.label(checking_status_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
    if state.step2.update_selected_download_running {
        ui.label(download_status_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
    if state.step2.update_selected_extract_running {
        ui.label(extract_status_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
}

fn checking_status_text(
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) -> String {
    if exact_log_mode {
        format!(
            "Checking missing mod sources {}/{}",
            state.step2.update_selected_check_done_count,
            state.step2.update_selected_check_total_count
        )
    } else if hybrid_missing_mode {
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
    }
}

const fn download_status_text(
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) -> &'static str {
    if exact_log_mode {
        "Downloading missing mod archives..."
    } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
        "Downloading missing mod / update archives..."
    } else if hybrid_missing_mode {
        "Downloading missing mod archives..."
    } else {
        "Downloading update archives..."
    }
}

const fn extract_status_text(
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) -> &'static str {
    if exact_log_mode {
        "Extracting downloaded missing mods..."
    } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
        "Extracting downloaded missing mods / updates..."
    } else if hybrid_missing_mode {
        "Extracting downloaded missing mods..."
    } else {
        "Extracting downloaded updates..."
    }
}

fn render_summary_status(
    ui: &mut egui::Ui,
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) {
    let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
    let hybrid_source_check_not_run = hybrid_missing_mode && !state.step2.update_selected_has_run;
    if build_from_scanned_mods && !state.step2.update_selected_has_run && !update_popup_busy(state)
    {
        ui.add_space(4.0);
        ui.label("No update check run yet.");
    } else if exact_log_ready_to_install(state) {
        ui.add_space(4.0);
        ui.label("No missing mods found. Exact-log install is good to go.");
    } else if exact_log_mode {
        render_exact_log_summary(ui, state);
    } else if hybrid_source_check_not_run {
        ui.label(format!(
            "Missing mods from applied log: {}",
            state.step2.log_pending_downloads.len()
        ));
        ui.label("No source check run yet.");
    } else if hybrid_missing_mode {
        render_hybrid_summary(ui, state);
    } else {
        render_standard_summary(ui, state);
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

fn render_standard_summary(ui: &mut egui::Ui, state: &WizardState) {
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

fn render_selection_stale_notice(ui: &mut egui::Ui, state: &WizardState) {
    let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
    let current_selection_signature =
        build_from_scanned_mods.then(|| update_selection_signature(&state.step2));
    let selection_stale = build_from_scanned_mods
        && single_mod_popup_target(state).is_none()
        && state.step2.update_selected_has_run
        && (!state.step2.update_selected_last_was_full_selection
            || state
                .step2
                .update_selected_last_selection_signature
                .as_deref()
                != current_selection_signature.as_deref());
    if selection_stale && !update_popup_busy(state) {
        ui.add_space(4.0);
        ui.label("Current selection differs from the last check. Run Check Updates again.");
    }
}

fn render_source_choice_section(
    ui: &mut egui::Ui,
    source_choices: &[SourceChoiceRow],
    popup_busy: bool,
    action: &mut Option<Step2Action>,
) -> Option<f32> {
    if source_choices.is_empty() {
        None
    } else {
        ui.add_space(8.0);
        Some(render_source_choices(ui, source_choices, popup_busy, action).list_prefix_width())
    }
}

fn render_update_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
    let hybrid_missing_mode =
        state.step1.bootstraps_from_weidu_logs() && !state.step2.log_pending_downloads.is_empty();
    if exact_log_ready_to_install(state)
        || (build_from_scanned_mods && !state.step2.update_selected_has_run)
    {
        return;
    }
    render_primary_result_list(
        ui,
        state,
        action,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
    );
    if exact_log_mode {
        render_exact_log_result_lists(
            ui,
            state,
            action,
            source_edit_rows,
            popup_busy,
            source_choice_prefix_width,
        );
    } else if hybrid_missing_mode {
        render_hybrid_result_lists(
            ui,
            state,
            action,
            source_edit_rows,
            popup_busy,
            source_choice_prefix_width,
        );
    } else {
        render_standard_result_lists(
            ui,
            state,
            action,
            source_edit_rows,
            popup_busy,
            source_choice_prefix_width,
        );
    }
    render_common_result_lists(
        ui,
        state,
        action,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
    );
}

fn render_primary_result_list(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let hybrid_missing_mode =
        state.step1.bootstraps_from_weidu_logs() && !state.step2.log_pending_downloads.is_empty();
    let hybrid_source_check_not_run = hybrid_missing_mode && !state.step2.update_selected_has_run;
    let pending_labels = hybrid_source_check_not_run.then(|| pending_log_labels(state));
    render_list(
        ui,
        primary_result_title(
            exact_log_mode,
            hybrid_missing_mode,
            hybrid_source_check_not_run,
        ),
        if hybrid_source_check_not_run {
            pending_labels.as_deref().unwrap_or(&[])
        } else if exact_log_mode || hybrid_missing_mode {
            &state.step2.update_selected_missing_sources
        } else {
            &state.step2.update_selected_update_sources
        },
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

const fn primary_result_title(
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
    hybrid_source_check_not_run: bool,
) -> &'static str {
    if hybrid_source_check_not_run {
        "Missing Mods From Applied Log"
    } else if exact_log_mode || hybrid_missing_mode {
        "Downloadable Missing Mods"
    } else {
        "Updates"
    }
}

fn render_exact_log_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    render_spaced_list(
        ui,
        "Auto Sources",
        &state.step2.update_selected_known_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_spaced_list(
        ui,
        "Manual Sources",
        &state.step2.update_selected_manual_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_spaced_list(
        ui,
        "No Source Entries",
        &state.step2.update_selected_unknown_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

fn render_hybrid_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    render_nonempty_spaced_list(
        ui,
        "Updates",
        &state.step2.update_selected_update_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Manual Sources",
        &state.step2.update_selected_manual_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "No Source Entries",
        &state.step2.update_selected_unknown_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

fn render_standard_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    render_nonempty_spaced_list(
        ui,
        "Manual",
        &state.step2.update_selected_manual_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Missing",
        &state.step2.update_selected_unknown_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

fn render_common_result_lists(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
) {
    let exact_or_hybrid = state.step1.installs_exactly_from_weidu_logs()
        || (state.step1.bootstraps_from_weidu_logs()
            && !state.step2.log_pending_downloads.is_empty());
    if exact_or_hybrid {
        render_nonempty_spaced_list(
            ui,
            "Exact Version Not Available",
            &state.step2.update_selected_exact_version_failed_sources,
            source_edit_rows,
            popup_busy,
            source_choice_prefix_width,
            action,
        );
    }
    render_nonempty_spaced_list(
        ui,
        if exact_or_hybrid {
            "Source Check Failed"
        } else {
            "Failed"
        },
        &state.step2.update_selected_failed_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Downloaded",
        &state.step2.update_selected_downloaded_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Download Failed",
        &state.step2.update_selected_download_failed_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Extracted",
        &state.step2.update_selected_extracted_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
    render_nonempty_spaced_list(
        ui,
        "Extract Failed",
        &state.step2.update_selected_extract_failed_sources,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

fn render_nonempty_spaced_list(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
    action: &mut Option<Step2Action>,
) {
    if !values.is_empty() {
        render_spaced_list(
            ui,
            title,
            values,
            source_edit_rows,
            popup_busy,
            source_choice_prefix_width,
            action,
        );
    }
}

fn render_spaced_list(
    ui: &mut egui::Ui,
    title: &str,
    values: &[String],
    source_edit_rows: &[SourceEditRow],
    popup_busy: bool,
    source_choice_prefix_width: Option<f32>,
    action: &mut Option<Step2Action>,
) {
    ui.add_space(8.0);
    render_list(
        ui,
        title,
        values,
        source_edit_rows,
        popup_busy,
        source_choice_prefix_width,
        action,
    );
}

fn render_update_popup_buttons(
    ui: &mut egui::Ui,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
    source_choices: &[SourceChoiceRow],
) {
    ui.horizontal_wrapped(|ui| {
        let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
        let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
        let can_check_updates = can_check_updates(state);
        let can_download = !state.step2.update_selected_update_assets.is_empty()
            && !state.step2.update_selected_check_running
            && !state.step2.update_selected_download_running
            && !state.step2.update_selected_extract_running;
        if ui
            .add_enabled(
                can_check_updates,
                egui::Button::new(check_button_label(exact_log_mode)),
            )
            .clicked()
        {
            *action = Some(check_action(state, exact_log_mode));
        }
        render_add_source_button(ui, state, action, source_choices);
        if ui
            .add_enabled(
                !build_from_scanned_mods || state.step2.update_selected_has_run,
                egui::Button::new("Copy Report"),
            )
            .clicked()
        {
            ui.ctx().copy_text(build_popup_report(
                state,
                exact_log_mode,
                exact_log_ready_to_install(state),
            ));
        }
        if ui
            .add_enabled(
                can_download,
                egui::Button::new(download_button_label(state)),
            )
            .clicked()
        {
            *action = Some(Step2Action::DownloadUpdates);
        }
        render_latest_fallback_button(ui, state, exact_log_mode);
        if ui.button("Close").clicked() {
            state.step2.update_selected_popup_open = false;
            state.step2.update_selected_confirm_latest_fallback_open = false;
        }
    });
}

fn can_check_updates(state: &WizardState) -> bool {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let review_edit_mode = state.step1.bootstraps_from_weidu_logs();
    let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
    let has_pending_missing_mods =
        review_edit_mode && !state.step2.log_pending_downloads.is_empty();
    let has_any_checked = state
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
        });
    if exact_log_mode {
        !update_popup_busy(state)
    } else if review_edit_mode {
        review_edit_any_log_applied(state)
            && (has_any_checked || has_pending_missing_mods)
            && !update_popup_busy(state)
    } else {
        build_from_scanned_mods && has_any_checked && !update_popup_busy(state)
    }
}

const fn check_button_label(exact_log_mode: bool) -> &'static str {
    if exact_log_mode {
        "Check Mod List"
    } else {
        "Check Updates"
    }
}

fn check_action(state: &WizardState, exact_log_mode: bool) -> Step2Action {
    if exact_log_mode {
        Step2Action::CheckExactLogModList
    } else if single_mod_popup_target(state).is_some() {
        Step2Action::PreviewUpdateSelectedMod
    } else {
        Step2Action::PreviewUpdateSelected
    }
}

fn render_add_source_button(
    ui: &mut egui::Ui,
    state: &WizardState,
    action: &mut Option<Step2Action>,
    source_choices: &[SourceChoiceRow],
) {
    if ui.button("Add Source").clicked() && action.is_none() {
        let (tp2, label) = add_source_target(state, source_choices)
            .unwrap_or_else(|| ("newmod".to_string(), "New Mod".to_string()));
        *action = Some(Step2Action::OpenModDownloadSourceEditor {
            tp2,
            label,
            source_id: "new-source".to_string(),
            allow_source_id_change: true,
        });
    }
}

fn add_source_target(
    state: &WizardState,
    source_choices: &[SourceChoiceRow],
) -> Option<(String, String)> {
    single_mod_popup_target(state)
        .as_ref()
        .map(|(_, tp_file)| {
            (
                mod_downloads::normalize_mod_download_tp2(tp_file),
                tp_file.clone(),
            )
        })
        .or_else(|| {
            (source_choices.len() == 1).then(|| {
                (
                    source_choices[0].tp2_key.clone(),
                    source_choices[0].label.clone(),
                )
            })
        })
}

fn download_button_label(state: &WizardState) -> &'static str {
    let hybrid_missing_mode =
        state.step1.bootstraps_from_weidu_logs() && !state.step2.log_pending_downloads.is_empty();
    if state.step1.installs_exactly_from_weidu_logs() {
        "Download Missing Mods"
    } else if hybrid_missing_mode
        && !state.step2.update_selected_missing_sources.is_empty()
        && !state.step2.update_selected_update_sources.is_empty()
    {
        "Download Missing Mods / Updates"
    } else if hybrid_missing_mode && !state.step2.update_selected_missing_sources.is_empty() {
        "Download Missing Mods"
    } else {
        "Download Updates"
    }
}

fn render_latest_fallback_button(ui: &mut egui::Ui, state: &mut WizardState, exact_log_mode: bool) {
    let can_retry_latest = exact_log_mode
        && !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running;
    if exact_log_mode
        && !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
        && ui
            .add_enabled(
                can_retry_latest,
                egui::Button::new("Use Latest For Exact-Version Misses"),
            )
            .clicked()
    {
        state.step2.update_selected_confirm_latest_fallback_open = true;
    }
}

fn render_latest_fallback_confirm(
    ctx: &egui::Context,
    state: &mut WizardState,
    action: &mut Option<Step2Action>,
) {
    if !state.step2.update_selected_confirm_latest_fallback_open {
        return;
    }
    let mut confirm_open = true;
    egui::Window::new("Download Latest Instead?")
        .open(&mut confirm_open)
        .collapsible(false)
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
                if ui.button("Yes").clicked() {
                    state.step2.update_selected_confirm_latest_fallback_open = false;
                    *action = Some(Step2Action::AcceptLatestForExactVersionMisses);
                }
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
        .collapsible(false)
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
