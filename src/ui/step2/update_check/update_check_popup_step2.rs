// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::mod_downloads;
use crate::app::state::{WizardState, exact_log_ready_to_install, update_selection_signature};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::review_edit_any_log_applied;
use crate::ui::step2::update_check_popup_lists_step2::{
    collect_source_choices, collect_source_edit_rows, pending_log_labels, render_list,
    render_section_header, render_source_choices, single_mod_popup_target,
};
use crate::ui::step2::update_check_popup_report_step2::build_popup_report;
use crate::ui::step2::update_check_popup_source_editor_step2::render_source_editor_popup;

pub fn render(ctx: &egui::Context, state: &mut WizardState, action: &mut Option<Step2Action>) {
    if !state.step2.update_selected_popup_open {
        return;
    }

    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let review_edit_mode = state.step1.bootstraps_from_weidu_logs();
    let build_from_scanned_mods = !state.step1.uses_source_weidu_logs();
    let has_pending_missing_mods =
        review_edit_mode && !state.step2.log_pending_downloads.is_empty();
    let hybrid_missing_mode = review_edit_mode && has_pending_missing_mods;
    let hybrid_source_check_not_run = hybrid_missing_mode && !state.step2.update_selected_has_run;
    let single_mod_popup_target = single_mod_popup_target(state);
    let current_selection_signature =
        build_from_scanned_mods.then(|| update_selection_signature(&state.step2));
    let selection_stale = build_from_scanned_mods
        && single_mod_popup_target.is_none()
        && state.step2.update_selected_has_run
        && (!state.step2.update_selected_last_was_full_selection
            || state
                .step2
                .update_selected_last_selection_signature
                .as_deref()
                != current_selection_signature.as_deref());
    let missing_count = state.step2.update_selected_known_sources.len()
        + state.step2.update_selected_manual_sources.len()
        + state.step2.update_selected_unknown_sources.len();
    let exact_log_good_to_go = exact_log_ready_to_install(state);
    let can_retry_latest = exact_log_mode
        && !state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running;
    let popup_busy = state.step2.is_scanning
        || state.step2.update_selected_check_running
        || state.step2.update_selected_download_running
        || state.step2.update_selected_extract_running;
    let source_load = mod_downloads::load_mod_download_sources();
    let source_choices = collect_source_choices(state, &source_load);
    let source_edit_rows = collect_source_edit_rows(state);
    let mut open = state.step2.update_selected_popup_open;
    egui::Window::new(if exact_log_mode {
        "Check Mod List"
    } else {
        "Check Updates"
    })
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .movable(true)
    .default_size(egui::vec2(560.0, 320.0))
    .min_width(320.0)
    .min_height(180.0)
    .show(ctx, |ui| {
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
        let content_height = (ui.available_height() - 40.0).max(80.0);
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .max_width(ui.available_width())
            .max_height(content_height)
            .show(ui, |ui| {
                if state.step2.update_selected_check_running {
                    ui.label(if exact_log_mode {
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
                    });
                }
                if state.step2.update_selected_download_running {
                    ui.label(if exact_log_mode {
                        "Downloading missing mod archives..."
                    } else if hybrid_missing_mode
                        && !state.step2.update_selected_update_sources.is_empty()
                    {
                        "Downloading missing mod / update archives..."
                    } else if hybrid_missing_mode {
                        "Downloading missing mod archives..."
                    } else {
                        "Downloading update archives..."
                    });
                }
                if state.step2.update_selected_extract_running {
                    ui.label(if exact_log_mode {
                        "Extracting downloaded missing mods..."
                    } else if hybrid_missing_mode
                        && !state.step2.update_selected_update_sources.is_empty()
                    {
                        "Extracting downloaded missing mods / updates..."
                    } else if hybrid_missing_mode {
                        "Extracting downloaded missing mods..."
                    } else {
                        "Extracting downloaded updates..."
                    });
                }
                if build_from_scanned_mods
                    && !state.step2.update_selected_has_run
                    && !state.step2.update_selected_check_running
                    && !state.step2.update_selected_download_running
                    && !state.step2.update_selected_extract_running
                {
                    ui.add_space(4.0);
                    ui.label("No update check run yet.");
                } else if exact_log_good_to_go {
                    ui.add_space(4.0);
                    ui.label("No missing mods found. Exact-log install is good to go.");
                } else if exact_log_mode {
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
                } else if hybrid_source_check_not_run {
                    ui.label(format!(
                        "Missing mods from applied log: {}",
                        state.step2.log_pending_downloads.len()
                    ));
                    ui.label("No source check run yet.");
                } else if hybrid_missing_mode {
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
                } else {
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
                if selection_stale
                    && !state.step2.update_selected_check_running
                    && !state.step2.update_selected_download_running
                    && !state.step2.update_selected_extract_running
                {
                    ui.add_space(4.0);
                    ui.label(
                        "Current selection differs from the last check. Run Check Updates again.",
                    );
                }
                let source_choice_prefix_width = if !source_choices.is_empty() {
                    ui.add_space(8.0);
                    Some(
                        render_source_choices(ui, &source_choices, popup_busy, action)
                            .list_prefix_width(),
                    )
                } else {
                    None
                };
                ui.add_space(8.0);
                if !exact_log_good_to_go
                    && (!build_from_scanned_mods || state.step2.update_selected_has_run)
                {
                    let pending_labels =
                        hybrid_source_check_not_run.then(|| pending_log_labels(state));
                    render_list(
                        ui,
                        if hybrid_source_check_not_run {
                            "Missing Mods From Applied Log"
                        } else if exact_log_mode || hybrid_missing_mode {
                            "Downloadable Missing Mods"
                        } else {
                            "Updates"
                        },
                        if hybrid_source_check_not_run {
                            pending_labels.as_deref().unwrap_or(&[])
                        } else if exact_log_mode || hybrid_missing_mode {
                            &state.step2.update_selected_missing_sources
                        } else {
                            &state.step2.update_selected_update_sources
                        },
                        &source_edit_rows,
                        popup_busy,
                        source_choice_prefix_width,
                        action,
                    );
                    if exact_log_mode {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Auto Sources",
                            &state.step2.update_selected_known_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Manual Sources",
                            &state.step2.update_selected_manual_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "No Source Entries",
                            &state.step2.update_selected_unknown_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    } else if hybrid_missing_mode {
                        if !state.step2.update_selected_update_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Updates",
                                &state.step2.update_selected_update_sources,
                                &source_edit_rows,
                                popup_busy,
                                source_choice_prefix_width,
                                action,
                            );
                        }
                        if !state.step2.update_selected_manual_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Manual Sources",
                                &state.step2.update_selected_manual_sources,
                                &source_edit_rows,
                                popup_busy,
                                source_choice_prefix_width,
                                action,
                            );
                        }
                        if !state.step2.update_selected_unknown_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "No Source Entries",
                                &state.step2.update_selected_unknown_sources,
                                &source_edit_rows,
                                popup_busy,
                                source_choice_prefix_width,
                                action,
                            );
                        }
                    } else {
                        if !state.step2.update_selected_manual_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Manual",
                                &state.step2.update_selected_manual_sources,
                                &source_edit_rows,
                                popup_busy,
                                source_choice_prefix_width,
                                action,
                            );
                        }
                        if !state.step2.update_selected_unknown_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Missing",
                                &state.step2.update_selected_unknown_sources,
                                &source_edit_rows,
                                popup_busy,
                                source_choice_prefix_width,
                                action,
                            );
                        }
                    }
                    if (exact_log_mode || hybrid_missing_mode)
                        && !state
                            .step2
                            .update_selected_exact_version_failed_sources
                            .is_empty()
                    {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Exact Version Not Available",
                            &state.step2.update_selected_exact_version_failed_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                    if !state.step2.update_selected_failed_sources.is_empty() {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            if exact_log_mode || hybrid_missing_mode {
                                "Source Check Failed"
                            } else {
                                "Failed"
                            },
                            &state.step2.update_selected_failed_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                    if !state.step2.update_selected_downloaded_sources.is_empty() {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Downloaded",
                            &state.step2.update_selected_downloaded_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                    if !state
                        .step2
                        .update_selected_download_failed_sources
                        .is_empty()
                    {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Download Failed",
                            &state.step2.update_selected_download_failed_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                    if !state.step2.update_selected_extracted_sources.is_empty() {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Extracted",
                            &state.step2.update_selected_extracted_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                    if !state
                        .step2
                        .update_selected_extract_failed_sources
                        .is_empty()
                    {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Extract Failed",
                            &state.step2.update_selected_extract_failed_sources,
                            &source_edit_rows,
                            popup_busy,
                            source_choice_prefix_width,
                            action,
                        );
                    }
                }
            });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let can_check_updates = if exact_log_mode {
                !state.step2.is_scanning
                    && !state.step2.update_selected_check_running
                    && !state.step2.update_selected_download_running
                    && !state.step2.update_selected_extract_running
            } else if review_edit_mode {
                review_edit_any_log_applied(state)
                    && (has_any_checked || has_pending_missing_mods)
                    && !state.step2.is_scanning
                    && !state.step2.update_selected_check_running
                    && !state.step2.update_selected_download_running
                    && !state.step2.update_selected_extract_running
            } else {
                build_from_scanned_mods
                    && has_any_checked
                    && !state.step2.is_scanning
                    && !state.step2.update_selected_check_running
                    && !state.step2.update_selected_download_running
                    && !state.step2.update_selected_extract_running
            };
            let can_download = !state.step2.update_selected_update_assets.is_empty()
                && !state.step2.update_selected_check_running
                && !state.step2.update_selected_download_running
                && !state.step2.update_selected_extract_running;
            let can_copy_report = !build_from_scanned_mods || state.step2.update_selected_has_run;
            let download_label = if exact_log_mode {
                "Download Missing Mods"
            } else if hybrid_missing_mode
                && !state.step2.update_selected_missing_sources.is_empty()
                && !state.step2.update_selected_update_sources.is_empty()
            {
                "Download Missing Mods / Updates"
            } else if hybrid_missing_mode && !state.step2.update_selected_missing_sources.is_empty()
            {
                "Download Missing Mods"
            } else {
                "Download Updates"
            };
            if ui
                .add_enabled(
                    can_check_updates,
                    egui::Button::new(if exact_log_mode {
                        "Check Mod List"
                    } else {
                        "Check Updates"
                    }),
                )
                .clicked()
            {
                *action = Some(if exact_log_mode {
                    Step2Action::CheckExactLogModList
                } else if single_mod_popup_target.is_some() {
                    Step2Action::PreviewUpdateSelectedMod
                } else {
                    Step2Action::PreviewUpdateSelected
                });
            }
            let add_source_target = single_mod_popup_target
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
                });
            if ui.button("Add Source").clicked() && action.is_none() {
                let (tp2, label) = add_source_target
                    .unwrap_or_else(|| ("newmod".to_string(), "New Mod".to_string()));
                *action = Some(Step2Action::OpenModDownloadSourceEditor {
                    tp2,
                    label,
                    source_id: "new-source".to_string(),
                    allow_source_id_change: true,
                });
            }
            if ui
                .add_enabled(can_copy_report, egui::Button::new("Copy Report"))
                .clicked()
            {
                ui.ctx().copy_text(build_popup_report(
                    state,
                    exact_log_mode,
                    exact_log_good_to_go,
                ));
            }
            if ui
                .add_enabled(can_download, egui::Button::new(download_label))
                .clicked()
            {
                *action = Some(Step2Action::DownloadUpdates);
            }
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
            if ui.button("Close").clicked() {
                state.step2.update_selected_popup_open = false;
                state.step2.update_selected_confirm_latest_fallback_open = false;
            }
        });
    });
    state.step2.update_selected_popup_open = open && state.step2.update_selected_popup_open;

    if state.step2.update_selected_confirm_latest_fallback_open {
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
    render_source_editor_popup(ctx, state, action);
    render_forks_popup(ctx, state, action);
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
