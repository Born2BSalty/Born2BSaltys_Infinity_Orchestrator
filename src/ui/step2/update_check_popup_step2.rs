// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use eframe::egui;

use crate::app::state::{WizardState, exact_log_ready_to_install, update_selection_signature};
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::step2::state_step2::review_edit_any_log_applied;

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
    let current_selection_signature =
        build_from_scanned_mods.then(|| update_selection_signature(&state.step2));
    let selection_stale = build_from_scanned_mods
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
    .default_size(egui::vec2(420.0, 320.0))
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
            } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty()
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
            } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty()
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
            ui.label("Current selection differs from the last check. Run Check Updates again.");
        }
        ui.add_space(8.0);
        if !exact_log_good_to_go
            && (!build_from_scanned_mods || state.step2.update_selected_has_run)
        {
            let scroll_width = ui.available_width();
            let pending_labels = hybrid_source_check_not_run.then(|| pending_log_labels(state));
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - 36.0).max(80.0))
                .show(ui, |ui| {
                    ui.set_min_width(scroll_width);
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
                    );
                    if exact_log_mode {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Auto Sources",
                            &state.step2.update_selected_known_sources,
                        );
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Manual Sources",
                            &state.step2.update_selected_manual_sources,
                        );
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "No Source Entries",
                            &state.step2.update_selected_unknown_sources,
                        );
                    } else if hybrid_missing_mode {
                        if !state.step2.update_selected_update_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(ui, "Updates", &state.step2.update_selected_update_sources);
                        }
                        if !state.step2.update_selected_manual_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Manual Sources",
                                &state.step2.update_selected_manual_sources,
                            );
                        }
                        if !state.step2.update_selected_unknown_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "No Source Entries",
                                &state.step2.update_selected_unknown_sources,
                            );
                        }
                    } else {
                        if !state.step2.update_selected_manual_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(ui, "Manual", &state.step2.update_selected_manual_sources);
                        }
                        if !state.step2.update_selected_unknown_sources.is_empty() {
                            ui.add_space(8.0);
                            render_list(
                                ui,
                                "Missing",
                                &state.step2.update_selected_unknown_sources,
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
                        );
                    }
                    if !state.step2.update_selected_downloaded_sources.is_empty() {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Downloaded",
                            &state.step2.update_selected_downloaded_sources,
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
                        );
                    }
                    if !state.step2.update_selected_extracted_sources.is_empty() {
                        ui.add_space(8.0);
                        render_list(
                            ui,
                            "Extracted",
                            &state.step2.update_selected_extracted_sources,
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
                        );
                    }
                });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
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
                } else {
                    Step2Action::PreviewUpdateSelected
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
}

fn render_list(ui: &mut egui::Ui, title: &str, values: &[String]) {
    ui.label(crate::ui::shared::typography_global::strong(title));
    if values.is_empty() {
        ui.label("None");
    } else {
        for value in values {
            ui.label(value);
        }
    }
}

fn pending_log_labels(state: &WizardState) -> Vec<String> {
    state
        .step2
        .log_pending_downloads
        .iter()
        .map(|pending| pending.label.clone())
        .collect()
}

fn build_popup_report(
    state: &WizardState,
    exact_log_mode: bool,
    exact_log_good_to_go: bool,
) -> String {
    let mut lines = Vec::<String>::new();
    let review_edit_mode = state.step1.bootstraps_from_weidu_logs();
    let has_pending_missing_mods =
        review_edit_mode && !state.step2.log_pending_downloads.is_empty();
    let hybrid_missing_mode = review_edit_mode && has_pending_missing_mods;
    let hybrid_source_check_not_run = hybrid_missing_mode && !state.step2.update_selected_has_run;
    let pending_labels = pending_log_labels(state);

    if state.step2.update_selected_check_running {
        lines.push(if exact_log_mode {
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
        lines.push(if exact_log_mode {
            "Downloading missing mod archives...".to_string()
        } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
            "Downloading missing mod / update archives...".to_string()
        } else if hybrid_missing_mode {
            "Downloading missing mod archives...".to_string()
        } else {
            "Downloading update archives...".to_string()
        });
    }
    if state.step2.update_selected_extract_running {
        lines.push(if exact_log_mode {
            "Extracting downloaded missing mods...".to_string()
        } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
            "Extracting downloaded missing mods / updates...".to_string()
        } else if hybrid_missing_mode {
            "Extracting downloaded missing mods...".to_string()
        } else {
            "Extracting downloaded updates...".to_string()
        });
    }

    if exact_log_good_to_go {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("No missing mods found. Exact-log install is good to go.".to_string());
        return lines.join("\n");
    }

    let missing_count = state.step2.update_selected_known_sources.len()
        + state.step2.update_selected_manual_sources.len()
        + state.step2.update_selected_unknown_sources.len();
    if !lines.is_empty() {
        lines.push(String::new());
    }
    if exact_log_mode {
        lines.push(format!("Missing mods: {missing_count}"));
        lines.push(format!(
            "Downloadable missing mods: {}",
            state.step2.update_selected_missing_sources.len()
        ));
        lines.push(format!(
            "Auto sources: {}",
            state.step2.update_selected_known_sources.len()
        ));
        lines.push(format!(
            "Manual sources: {}",
            state.step2.update_selected_manual_sources.len()
        ));
        lines.push(format!(
            "No source entries: {}",
            state.step2.update_selected_unknown_sources.len()
        ));
        lines.push(format!(
            "Exact version not available: {}",
            state
                .step2
                .update_selected_exact_version_failed_sources
                .len()
        ));
    } else if hybrid_source_check_not_run {
        lines.push(format!(
            "Missing mods from applied log: {}",
            state.step2.log_pending_downloads.len()
        ));
        lines.push("No source check run yet.".to_string());
    } else if hybrid_missing_mode {
        lines.push(format!(
            "Updates found: {}",
            state.step2.update_selected_update_sources.len()
        ));
        lines.push(format!(
            "Missing mods: {}",
            state.step2.log_pending_downloads.len()
        ));
        lines.push(format!(
            "Downloadable missing mods: {}",
            state.step2.update_selected_missing_sources.len()
        ));
        lines.push(format!(
            "Auto sources checked: {}",
            state.step2.update_selected_known_sources.len()
        ));
        lines.push(format!(
            "Manual sources: {}",
            state.step2.update_selected_manual_sources.len()
        ));
        lines.push(format!(
            "No source entries: {}",
            state.step2.update_selected_unknown_sources.len()
        ));
        lines.push(format!(
            "Exact version not available: {}",
            state
                .step2
                .update_selected_exact_version_failed_sources
                .len()
        ));
    } else {
        lines.push(format!(
            "Updates found: {}",
            state.step2.update_selected_update_sources.len()
        ));
        lines.push(format!(
            "Auto sources checked: {}",
            state.step2.update_selected_known_sources.len()
        ));
        lines.push(format!(
            "Manual sources: {}",
            state.step2.update_selected_manual_sources.len()
        ));
        lines.push(format!(
            "Missing sources: {}",
            state.step2.update_selected_unknown_sources.len()
        ));
    }

    lines.push(String::new());
    append_report_section(
        &mut lines,
        if hybrid_source_check_not_run {
            "Missing Mods From Applied Log"
        } else if exact_log_mode || hybrid_missing_mode {
            "Downloadable Missing Mods"
        } else {
            "Updates"
        },
        if hybrid_source_check_not_run {
            &pending_labels
        } else if exact_log_mode || hybrid_missing_mode {
            &state.step2.update_selected_missing_sources
        } else {
            &state.step2.update_selected_update_sources
        },
    );
    if hybrid_source_check_not_run {
        return lines.join("\n");
    }
    if exact_log_mode {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Auto Sources",
            &state.step2.update_selected_known_sources,
        );
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Manual Sources",
            &state.step2.update_selected_manual_sources,
        );
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "No Source Entries",
            &state.step2.update_selected_unknown_sources,
        );
    } else if hybrid_missing_mode {
        if !state.step2.update_selected_update_sources.is_empty() {
            lines.push(String::new());
            append_report_section(
                &mut lines,
                "Updates",
                &state.step2.update_selected_update_sources,
            );
        }
        if !state.step2.update_selected_manual_sources.is_empty() {
            lines.push(String::new());
            append_report_section(
                &mut lines,
                "Manual Sources",
                &state.step2.update_selected_manual_sources,
            );
        }
        if !state.step2.update_selected_unknown_sources.is_empty() {
            lines.push(String::new());
            append_report_section(
                &mut lines,
                "No Source Entries",
                &state.step2.update_selected_unknown_sources,
            );
        }
    } else {
        if !state.step2.update_selected_manual_sources.is_empty() {
            lines.push(String::new());
            append_report_section(
                &mut lines,
                "Manual",
                &state.step2.update_selected_manual_sources,
            );
        }
        if !state.step2.update_selected_unknown_sources.is_empty() {
            lines.push(String::new());
            append_report_section(
                &mut lines,
                "Missing",
                &state.step2.update_selected_unknown_sources,
            );
        }
    }

    if (exact_log_mode || hybrid_missing_mode)
        && !state
            .step2
            .update_selected_exact_version_failed_sources
            .is_empty()
    {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Exact Version Not Available",
            &state.step2.update_selected_exact_version_failed_sources,
        );
    }
    if !state.step2.update_selected_failed_sources.is_empty() {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            if exact_log_mode || hybrid_missing_mode {
                "Source Check Failed"
            } else {
                "Failed"
            },
            &state.step2.update_selected_failed_sources,
        );
    }
    if !state.step2.update_selected_downloaded_sources.is_empty() {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Downloaded",
            &state.step2.update_selected_downloaded_sources,
        );
    }
    if !state
        .step2
        .update_selected_download_failed_sources
        .is_empty()
    {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Download Failed",
            &state.step2.update_selected_download_failed_sources,
        );
    }
    if !state.step2.update_selected_extracted_sources.is_empty() {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Extracted",
            &state.step2.update_selected_extracted_sources,
        );
    }
    if !state
        .step2
        .update_selected_extract_failed_sources
        .is_empty()
    {
        lines.push(String::new());
        append_report_section(
            &mut lines,
            "Extract Failed",
            &state.step2.update_selected_extract_failed_sources,
        );
    }

    lines.join("\n")
}

fn append_report_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    lines.push(title.to_string());
    if values.is_empty() {
        lines.push("None".to_string());
    } else {
        lines.extend(values.iter().cloned());
    }
}
