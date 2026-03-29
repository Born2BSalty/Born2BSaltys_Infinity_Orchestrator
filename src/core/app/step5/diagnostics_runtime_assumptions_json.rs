// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::WizardState;

pub(super) fn write_runtime_assumptions_json(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("runtime_assumptions.json");
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "step1": {
            "game_install": state.step1.game_install,
            "have_weidu_logs": state.step1.have_weidu_logs,
            "skip_installed": state.step1.skip_installed,
            "check_last_installed": state.step1.check_last_installed,
            "strict_matching": state.step1.strict_matching,
            "generate_directory_enabled": state.step1.generate_directory_enabled,
            "prepare_target_dirs_before_install": state.step1.prepare_target_dirs_before_install,
            "weidu_log_mode_enabled": state.step1.weidu_log_mode_enabled,
            "bio_full_debug": state.step1.bio_full_debug,
            "rust_log_debug": state.step1.rust_log_debug,
            "rust_log_trace": state.step1.rust_log_trace,
            "log_raw_output_dev": state.step1.log_raw_output_dev,
        },
        "step2": {
            "active_game_tab": state.step2.active_game_tab,
            "scan_status": state.step2.scan_status,
            "search_query": state.step2.search_query,
            "compat_popup_filter": state.step2.compat_popup_filter,
            "selected_count": state.step2.selected_count,
            "total_count": state.step2.total_count,
        },
        "step3": {
            "active_game_tab": state.step3.active_game_tab,
            "bgee_has_conflict": state.step3.bgee_has_conflict,
            "bg2ee_has_conflict": state.step3.bg2ee_has_conflict,
        },
        "step5": {
            "last_start_mode": state.step5.last_start_mode,
            "last_cancel_mode": state.step5.last_cancel_mode,
            "last_exit_code": state.step5.last_exit_code,
            "resume_available": state.step5.resume_available,
            "install_running": state.step5.install_running,
        }
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
