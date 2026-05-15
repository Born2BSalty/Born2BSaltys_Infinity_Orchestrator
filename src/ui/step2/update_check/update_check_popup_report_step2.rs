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

    append_progress_lines(&mut lines, state, exact_log_mode, hybrid_missing_mode);

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
    append_summary_lines(
        &mut lines,
        state,
        &ReportMode {
            exact_log_mode,
            hybrid_source_check_not_run,
            hybrid_missing_mode,
            missing_count,
        },
    );

    lines.push(String::new());
    append_primary_report_section(
        &mut lines,
        state,
        &pending_labels,
        exact_log_mode,
        hybrid_source_check_not_run,
        hybrid_missing_mode,
    );
    if hybrid_source_check_not_run {
        return lines.join("\n");
    }
    append_secondary_report_sections(&mut lines, state, exact_log_mode, hybrid_missing_mode);

    append_tail_report_sections(&mut lines, state, exact_log_mode, hybrid_missing_mode);

    lines.join("\n")
}

fn append_progress_lines(
    lines: &mut Vec<String>,
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) {
    if state.step2.update_selected_check_running {
        lines.push(check_progress_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
    if state.step2.update_selected_download_running {
        lines.push(download_progress_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
    if state.step2.update_selected_extract_running {
        lines.push(extract_progress_text(
            state,
            exact_log_mode,
            hybrid_missing_mode,
        ));
    }
}

fn check_progress_text(
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

fn download_progress_text(
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) -> String {
    if exact_log_mode {
        "Downloading missing mod archives...".to_string()
    } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
        "Downloading missing mod / update archives...".to_string()
    } else if hybrid_missing_mode {
        "Downloading missing mod archives...".to_string()
    } else {
        "Downloading update archives...".to_string()
    }
}

fn extract_progress_text(
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) -> String {
    if exact_log_mode {
        "Extracting downloaded missing mods...".to_string()
    } else if hybrid_missing_mode && !state.step2.update_selected_update_sources.is_empty() {
        "Extracting downloaded missing mods / updates...".to_string()
    } else if hybrid_missing_mode {
        "Extracting downloaded missing mods...".to_string()
    } else {
        "Extracting downloaded updates...".to_string()
    }
}

struct ReportMode {
    exact_log_mode: bool,
    hybrid_source_check_not_run: bool,
    hybrid_missing_mode: bool,
    missing_count: usize,
}

fn append_summary_lines(lines: &mut Vec<String>, state: &WizardState, mode: &ReportMode) {
    if mode.exact_log_mode {
        append_exact_log_summary(lines, state, mode.missing_count);
    } else if mode.hybrid_source_check_not_run {
        lines.push(format!(
            "Missing mods from applied log: {}",
            state.step2.log_pending_downloads.len()
        ));
        lines.push("No source check run yet.".to_string());
    } else if mode.hybrid_missing_mode {
        append_hybrid_summary(lines, state);
    } else {
        append_update_summary(lines, state);
    }
}

fn append_exact_log_summary(lines: &mut Vec<String>, state: &WizardState, missing_count: usize) {
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
}

fn append_hybrid_summary(lines: &mut Vec<String>, state: &WizardState) {
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
}

fn append_update_summary(lines: &mut Vec<String>, state: &WizardState) {
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

fn append_primary_report_section(
    lines: &mut Vec<String>,
    state: &WizardState,
    pending_labels: &[String],
    exact_log_mode: bool,
    hybrid_source_check_not_run: bool,
    hybrid_missing_mode: bool,
) {
    append_report_section(
        lines,
        if hybrid_source_check_not_run {
            "Missing Mods From Applied Log"
        } else if exact_log_mode || hybrid_missing_mode {
            "Downloadable Missing Mods"
        } else {
            "Updates"
        },
        if hybrid_source_check_not_run {
            pending_labels
        } else if exact_log_mode || hybrid_missing_mode {
            &state.step2.update_selected_missing_sources
        } else {
            &state.step2.update_selected_update_sources
        },
    );
}

fn append_secondary_report_sections(
    lines: &mut Vec<String>,
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) {
    if exact_log_mode {
        append_spaced_section(
            lines,
            "Auto Sources",
            &state.step2.update_selected_known_sources,
        );
        append_spaced_section(
            lines,
            "Manual Sources",
            &state.step2.update_selected_manual_sources,
        );
        append_spaced_section(
            lines,
            "No Source Entries",
            &state.step2.update_selected_unknown_sources,
        );
    } else if hybrid_missing_mode {
        append_nonempty_spaced_section(
            lines,
            "Updates",
            &state.step2.update_selected_update_sources,
        );
        append_nonempty_spaced_section(
            lines,
            "Manual Sources",
            &state.step2.update_selected_manual_sources,
        );
        append_nonempty_spaced_section(
            lines,
            "No Source Entries",
            &state.step2.update_selected_unknown_sources,
        );
    } else {
        append_nonempty_spaced_section(
            lines,
            "Manual",
            &state.step2.update_selected_manual_sources,
        );
        append_nonempty_spaced_section(
            lines,
            "Missing",
            &state.step2.update_selected_unknown_sources,
        );
    }
}

fn append_tail_report_sections(
    lines: &mut Vec<String>,
    state: &WizardState,
    exact_log_mode: bool,
    hybrid_missing_mode: bool,
) {
    if exact_log_mode || hybrid_missing_mode {
        append_nonempty_spaced_section(
            lines,
            "Exact Version Not Available",
            &state.step2.update_selected_exact_version_failed_sources,
        );
    }
    append_nonempty_spaced_section(
        lines,
        if exact_log_mode || hybrid_missing_mode {
            "Source Check Failed"
        } else {
            "Failed"
        },
        &state.step2.update_selected_failed_sources,
    );
    append_nonempty_spaced_section(
        lines,
        "Downloaded",
        &state.step2.update_selected_downloaded_sources,
    );
    append_nonempty_spaced_section(
        lines,
        "Download Failed",
        &state.step2.update_selected_download_failed_sources,
    );
    append_nonempty_spaced_section(
        lines,
        "Extracted",
        &state.step2.update_selected_extracted_sources,
    );
    append_nonempty_spaced_section(
        lines,
        "Extract Failed",
        &state.step2.update_selected_extract_failed_sources,
    );
}

fn append_spaced_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    lines.push(String::new());
    append_report_section(lines, title, values);
}

fn append_nonempty_spaced_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    if !values.is_empty() {
        append_spaced_section(lines, title, values);
    }
}

fn append_report_section(lines: &mut Vec<String>, title: &str, values: &[String]) {
    lines.push(title.to_string());
    if values.is_empty() {
        lines.push("None".to_string());
    } else {
        lines.extend(values.iter().cloned());
    }
}
