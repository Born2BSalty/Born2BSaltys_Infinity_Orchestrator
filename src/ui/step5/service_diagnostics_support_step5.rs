// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::ui::state::{ResumeTargets, Step1State, WizardState};
use crate::ui::terminal::EmbeddedTerminal;

pub(crate) fn apply_dev_defaults(state: &mut WizardState, dev_mode: bool) {
    if dev_mode {
        state.step1.bio_full_debug = true;
        state.step1.log_raw_output_dev = true;
    } else {
        state.step1.bio_full_debug = false;
        state.step1.log_raw_output_dev = false;
    }
}

pub(crate) fn export_diagnostics(
    state: &WizardState,
    terminal: Option<&EmbeddedTerminal>,
    dev_mode: bool,
    exe_fingerprint: &str,
) -> anyhow::Result<PathBuf> {
    if !dev_mode {
        anyhow::bail!("Diagnostics export is only available in -d mode");
    }
    let ctx = crate::ui::step5::diagnostics::DiagnosticsContext {
        dev_mode,
        exe_fingerprint: exe_fingerprint.to_string(),
    };
    crate::ui::step5::diagnostics::export_diagnostics(state, terminal, &ctx)
}

pub(crate) fn copy_weidu_logs_for_diagnostics(step1: &Step1State, run_id: &str) {
    if !step1.bio_full_debug && !step1.log_raw_output_dev {
        return;
    }
    let run_dir = crate::ui::step5::service_diagnostics_run_step5::run_dir_from_id(run_id);
    let source_logs_dir = run_dir.join("source_logs");
    let _ = crate::ui::step5::log_files::copy_source_weidu_logs(step1, &source_logs_dir, "original");
    let saved_logs_dir = run_dir.join("saved_logs");
    let _ = crate::ui::step5::log_files::copy_saved_weidu_logs(step1, &saved_logs_dir, "original");
}

pub(crate) fn source_log_infos(
    step1: &Step1State,
) -> Vec<crate::ui::step5::log_files::SourceLogInfo> {
    crate::ui::step5::log_files::source_log_infos(step1)
}

pub(crate) fn save_console_log(
    state: &WizardState,
    console_text: &str,
) -> anyhow::Result<PathBuf> {
    Ok(crate::ui::step5::log_files::save_console_log(
        &state.step5,
        console_text,
    )?)
}

pub(crate) fn open_console_logs_folder() -> anyhow::Result<()> {
    Ok(crate::ui::step5::log_files::open_console_logs_folder()?)
}

pub(crate) fn open_last_log_file(step1: &Step1State) -> anyhow::Result<()> {
    Ok(crate::ui::step5::log_files::open_last_log_file(step1)?)
}

pub(crate) fn prepare_target_dirs_before_install(
    step1: &Step1State,
) -> anyhow::Result<crate::ui::step5::log_files::TargetPrepResult> {
    Ok(crate::ui::step5::log_files::prepare_target_dirs_before_install(step1)?)
}

pub(crate) fn validate_resume_paths(
    step1: &Step1State,
    resume_targets: &ResumeTargets,
) -> anyhow::Result<()> {
    crate::ui::step5::log_files::validate_resume_paths(step1, resume_targets)
        .map_err(anyhow::Error::msg)
}

pub(crate) fn validate_runtime_prep_paths(step1: &Step1State) -> anyhow::Result<()> {
    crate::ui::step5::log_files::validate_runtime_prep_paths(step1)
        .map_err(anyhow::Error::msg)
}

pub(crate) fn verify_targets_prepared(step1: &Step1State) -> anyhow::Result<()> {
    crate::ui::step5::log_files::verify_targets_prepared(step1)
        .map_err(anyhow::Error::msg)
}
