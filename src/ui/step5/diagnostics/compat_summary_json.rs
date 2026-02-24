// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::state::CompatIssueDisplay;

use super::compat_summary::build_compat_summary;

pub(super) fn write_compat_summary_json(
    run_dir: &Path,
    issues: &[CompatIssueDisplay],
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let summary = build_compat_summary(issues);

    let out_path = run_dir.join("compat_summary.json");
    let payload = json!({
        "schema_version": 1,
        "totals": {
            "issues": summary.total_issues,
            "errors": summary.total_errors,
            "warnings": summary.total_warnings
        },
        "by_code": summary.by_code,
        "top_conflict_groups": sorted_group_entries(&summary.conflict_groups),
        "top_missing_dep_groups": sorted_group_entries(&summary.missing_dep_groups),
        "top_order_warn_groups": sorted_group_entries(&summary.order_warn_groups),
        "generated_at_unix": timestamp_unix_secs
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn sorted_group_entries(groups: &[super::compat_summary::GroupCount]) -> Vec<serde_json::Value> {
    groups
        .into_iter()
        .map(|entry| json!({ "group": &entry.group, "count": entry.count }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::state::CompatIssueDisplay;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_expected_json_shape() {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let run_dir = std::env::temp_dir().join(format!("bio_diag_test_{ts}"));
        std::fs::create_dir_all(&run_dir).expect("create temp diagnostics dir");

        let issues = vec![CompatIssueDisplay {
            issue_id: "id-1".to_string(),
            code: "FORBID_HIT".to_string(),
            severity: "Error".to_string(),
            is_blocking: true,
            affected_mod: "a".to_string(),
            affected_component: Some(1),
            related_mod: "b".to_string(),
            related_component: Some(2),
            reason: "conflict".to_string(),
            source: "TP2".to_string(),
            raw_evidence: None,
        }];

        let path = write_compat_summary_json(&run_dir, &issues, 123).expect("write json");
        let raw = std::fs::read_to_string(&path).expect("read json");
        let v: serde_json::Value = serde_json::from_str(&raw).expect("parse json");

        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["totals"]["issues"], 1);
        assert!(v.get("by_code").is_some());
        assert!(v.get("top_conflict_groups").is_some());
        assert!(v.get("top_missing_dep_groups").is_some());
        assert!(v.get("top_order_warn_groups").is_some());

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(run_dir);
    }
}
