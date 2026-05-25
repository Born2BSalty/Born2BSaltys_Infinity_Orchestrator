// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::ui::step2::state_step2::applied_weidu_log_has_pending_downloads;
use crate::ui::step2::update_check_popup_lists_step2::pending_log_labels;

#[derive(Clone, Copy)]
struct PopupReportModes<Flag = bool> {
    exact_log: Flag,
    good_to_go: Flag,
    hybrid_missing: Flag,
    hybrid_source_pending: Flag,
}

pub(super) fn build_popup_report(
    state: &WizardState,
    exact_log_mode: bool,
    exact_log_good_to_go: bool,
) -> String {
    let mut lines = Vec::<String>::new();
    let modes = popup_report_modes(state, exact_log_mode, exact_log_good_to_go);
    append_running_lines(&mut lines, state, modes);

    if modes.good_to_go {
        append_blank_if_needed(&mut lines);
        lines.push("No missing mods found. Exact-log install is good to go.".to_string());
        return lines.join("\n");
    }

    append_blank_if_needed(&mut lines);
    append_summary_lines(&mut lines, state, modes);
    lines.push(String::new());

    let pending_labels = pending_log_labels(state);
    let (primary_title, primary_values) = primary_report_section(state, modes, &pending_labels);
    append_report_section(&mut lines, primary_title, primary_values);
    if modes.hybrid_source_pending {
        return lines.join("\n");
    }

    append_mode_sections(&mut lines, state, modes);
    append_result_sections(&mut lines, state, modes);
    lines.join("\n")
}

const fn popup_report_modes(
    state: &WizardState,
    exact_log: bool,
    good_to_go: bool,
) -> PopupReportModes {
    let hybrid_missing_mode = applied_weidu_log_has_pending_downloads(state);
    PopupReportModes {
        exact_log,
        good_to_go,
        hybrid_missing: hybrid_missing_mode,
        hybrid_source_pending: hybrid_missing_mode && !state.step2.update_selected_has_run,
    }
}

fn append_running_lines(lines: &mut Vec<String>, state: &WizardState, modes: PopupReportModes) {
    if state.step2.update_selected_check_running {
        lines.push(if modes.exact_log {
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
        lines.push(if modes.exact_log {
            "Downloading missing mod archives...".to_string()
        } else if modes.hybrid_missing && !state.step2.update_selected_update_sources.is_empty() {
            "Downloading missing mod / update archives...".to_string()
        } else if modes.hybrid_missing {
            "Downloading missing mod archives...".to_string()
        } else {
            "Downloading update archives...".to_string()
        });
    }
    if state.step2.update_selected_extract_running {
        lines.push(if modes.exact_log {
            "Extracting downloaded missing mods...".to_string()
        } else if modes.hybrid_missing && !state.step2.update_selected_update_sources.is_empty() {
            "Extracting downloaded missing mods / updates...".to_string()
        } else if modes.hybrid_missing {
            "Extracting downloaded missing mods...".to_string()
        } else {
            "Extracting downloaded updates...".to_string()
        });
    }
}

fn append_blank_if_needed(lines: &mut Vec<String>) {
    if !lines.is_empty() {
        lines.push(String::new());
    }
}

fn append_summary_lines(lines: &mut Vec<String>, state: &WizardState, modes: PopupReportModes) {
    let missing_count = state.step2.update_selected_known_sources.len()
        + state.step2.update_selected_manual_sources.len()
        + state.step2.update_selected_unknown_sources.len();
    if modes.exact_log {
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
    } else if modes.hybrid_source_pending {
        lines.push(format!(
            "Missing mods from applied log: {}",
            state.step2.log_pending_downloads.len()
        ));
        lines.push("No source check run yet.".to_string());
    } else if modes.hybrid_missing {
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
}

fn primary_report_section<'a>(
    state: &'a WizardState,
    modes: PopupReportModes,
    pending_labels: &'a [String],
) -> (&'static str, &'a [String]) {
    if modes.hybrid_source_pending {
        ("Missing Mods From Applied Log", pending_labels)
    } else if modes.exact_log || modes.hybrid_missing {
        (
            "Downloadable Missing Mods",
            &state.step2.update_selected_missing_sources,
        )
    } else {
        ("Updates", &state.step2.update_selected_update_sources)
    }
}

fn append_mode_sections(lines: &mut Vec<String>, state: &WizardState, modes: PopupReportModes) {
    if modes.exact_log {
        append_spaced_report_section(
            lines,
            "Auto Sources",
            &state.step2.update_selected_known_sources,
        );
        append_spaced_report_section(
            lines,
            "Manual Sources",
            &state.step2.update_selected_manual_sources,
        );
        append_spaced_report_section(
            lines,
            "No Source Entries",
            &state.step2.update_selected_unknown_sources,
        );
    } else if modes.hybrid_missing {
        if !state.step2.update_selected_update_sources.is_empty() {
            append_spaced_report_section(
                lines,
                "Updates",
                &state.step2.update_selected_update_sources,
            );
        }
        if !state.step2.update_selected_manual_sources.is_empty() {
            append_spaced_report_section(
                lines,
                "Manual Sources",
                &state.step2.update_selected_manual_sources,
            );
        }
        if !state.step2.update_selected_unknown_sources.is_empty() {
            append_spaced_report_section(
                lines,
                "No Source Entries",
                &state.step2.update_selected_unknown_sources,
            );
        }
    } else {
        if !state.step2.update_selected_manual_sources.is_empty() {
            append_spaced_report_section(
                lines,
                "Manual",
                &state.step2.update_selected_manual_sources,
            );
        }
        if !state.step2.update_selected_unknown_sources.is_empty() {
            append_spaced_report_section(
                lines,
                "Missing",
                &state.step2.update_selected_unknown_sources,
            );
        }
    }
}

fn append_result_sections(lines: &mut Vec<String>, state: &WizardState, modes: PopupReportModes) {
    if (modes.exact_log || modes.hybrid_missing)
        && !state
            .step2
            .update_selected_exact_version_failed_sources
            .is_empty()
    {
        append_spaced_report_section(
            lines,
            "Exact Version Not Available",
            &state.step2.update_selected_exact_version_failed_sources,
        );
    }
    if !state.step2.update_selected_failed_sources.is_empty() {
        append_spaced_report_section(
            lines,
            if modes.exact_log || modes.hybrid_missing {
                "Source Check Failed"
            } else {
                "Failed"
            },
            &state.step2.update_selected_failed_sources,
        );
    }
    if !state.step2.update_selected_downloaded_sources.is_empty() {
        append_spaced_report_section(
            lines,
            "Downloaded",
            &state.step2.update_selected_downloaded_sources,
        );
    }
    if !state
        .step2
        .update_selected_download_failed_sources
        .is_empty()
    {
        append_spaced_report_section(
            lines,
            "Download Failed",
            &state.step2.update_selected_download_failed_sources,
        );
    }
    if !state.step2.update_selected_extracted_sources.is_empty() {
        append_spaced_report_section(
            lines,
            "Extracted",
            &state.step2.update_selected_extracted_sources,
        );
    }
    if !state
        .step2
        .update_selected_extract_failed_sources
        .is_empty()
    {
        append_spaced_report_section(
            lines,
            "Extract Failed",
            &state.step2.update_selected_extract_failed_sources,
        );
    }
}

fn append_spaced_report_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    lines.push(String::new());
    append_report_section(lines, title, values);
}

fn append_report_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    lines.push(title.to_string());
    if values.is_empty() {
        lines.push("None".to_string());
    } else {
        lines.extend(values.iter().cloned());
    }
}
