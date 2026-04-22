// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::PathBuf;

use crate::app::state::{Step1State, WizardState};
use crate::app::terminal::EmbeddedTerminal;

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

pub(crate) fn source_log_infos(
    step1: &Step1State,
) -> Vec<crate::ui::step5::log_files::SourceLogInfo> {
    crate::ui::step5::log_files::source_log_infos(step1)
}

pub(crate) fn save_console_log(state: &WizardState, console_text: &str) -> anyhow::Result<PathBuf> {
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
