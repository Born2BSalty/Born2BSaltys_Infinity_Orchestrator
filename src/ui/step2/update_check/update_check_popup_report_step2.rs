// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::ui::step2::update_check_popup_lists_step2::pending_log_labels;

pub(super) fn build_popup_report(
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
