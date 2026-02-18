// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ui::state::CompatIssueDisplay;

use super::issue_text;
use super::target::format_issue_target;

pub(super) fn export_step3_compat_report(issues: &[CompatIssueDisplay]) -> std::io::Result<PathBuf> {
    let out_dir = PathBuf::from("diagnostics");
    fs::create_dir_all(&out_dir)?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let out_path = out_dir.join(format!("compat_step3_{ts}.txt"));

    let mut text = String::new();
    text.push_str("Step 3 Compatibility Report\n");
    text.push_str("===========================\n\n");
    let error_count = issues.iter().filter(|i| i.is_blocking).count();
    let warning_count = issues.len().saturating_sub(error_count);
    text.push_str(&format!("Errors: {error_count} | Warnings: {warning_count}\n\n"));
    if issues.is_empty() {
        text.push_str("No compatibility issues.\n");
    } else {
        for issue in issues {
            let sev = if issue.is_blocking { "ERROR" } else { "WARN" };
            let affected = format_issue_target(&issue.affected_mod, issue.affected_component);
            let related = format_issue_target(&issue.related_mod, issue.related_component);
            text.push_str(&format!("- [{sev}] {} {affected} -> {related}\n", issue.code));
            if !issue.reason.trim().is_empty() {
                text.push_str(&format!("  reason: {}\n", issue.reason));
            }
            if !issue.source.trim().is_empty() {
                text.push_str(&format!("  source: {}\n", issue.source));
            }
            text.push_str(&format!("  graph: {}\n", issue_text::issue_graph(issue)));
            if let Some(raw) = issue.raw_evidence.as_deref()
                && !raw.trim().is_empty()
            {
                text.push_str(&format!("  rule_detail: {raw}\n"));
            }
        }
    }

    fs::write(&out_path, text)?;
    Ok(out_path)
}
