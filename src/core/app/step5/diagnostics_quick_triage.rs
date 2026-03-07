// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::ui::state::WizardState;

pub(super) fn write_quick_triage_txt(
    run_dir: &Path,
    state: &WizardState,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("quick_triage.txt");
    let blocking_compat = state.compat.issues.iter().filter(|i| i.is_blocking).count();
    let first_failure = detect_first_failure(state, blocking_compat);
    let mut text = String::new();
    text.push_str("BIO quick triage\n");
    text.push_str("================\n\n");
    text.push_str(&format!("generated_at_unix={timestamp_unix_secs}\n"));
    text.push_str(&format!("first_failure={first_failure}\n"));
    text.push_str(&format!(
        "step1_path_check={}\n",
        match &state.step1_path_check {
            Some((ok, _)) => {
                if *ok { "ok" } else { "failed" }
            }
            None => "not_run",
        }
    ));
    text.push_str(&format!(
        "step2_status={}\n",
        state.step2.scan_status.trim()
    ));
    text.push_str(&format!(
        "compat_blocking_issues={blocking_compat}\n"
    ));
    text.push_str(&format!(
        "install_exit_code={:?}\n",
        state.step5.last_exit_code
    ));
    text.push_str(&format!(
        "install_status={}\n\n",
        state.step5.last_status_text.trim()
    ));
    text.push_str("open_these_files_first:\n");
    text.push_str("- bio_diag.txt\n");
    text.push_str("- compat_summary.json\n");
    text.push_str("- scan_context.json\n");
    text.push_str("- undefined_summary.json\n");
    text.push_str("- compat_decisions.json\n");
    text.push_str("- source_logs/\n");
    fs::write(&out_path, text)?;
    Ok(out_path)
}

fn detect_first_failure(state: &WizardState, blocking_compat: usize) -> String {
    if let Some((ok, msg)) = &state.step1_path_check
        && !ok
    {
        return format!("step1_validation_failed: {msg}");
    }
    let step2_status = state.step2.scan_status.to_ascii_lowercase();
    if step2_status.contains("scan failed") {
        return format!("step2_scan_failed: {}", state.step2.scan_status.trim());
    }
    if blocking_compat > 0 {
        return format!("step3_compat_blocking: {blocking_compat} issue(s)");
    }
    if let Some(code) = state.step5.last_exit_code
        && code != 0
    {
        return format!("step5_install_failed: exit_code={code}");
    }
    "none".to_string()
}
