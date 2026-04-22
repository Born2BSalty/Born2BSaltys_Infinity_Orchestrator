// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

pub(super) fn write_compat_rule_matches_summary_json(
    run_dir: &Path,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("compat_rule_matches_summary.json");
    let trace_path = run_dir.join("compat_rule_trace.json");
    let payload = match fs::read_to_string(&trace_path) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(trace) => build_summary_payload(timestamp_unix_secs, trace),
            Err(err) => json!({
                "schema_version": 1,
                "generated_at_unix": timestamp_unix_secs,
                "status": "invalid_trace",
                "trace_path": trace_path.display().to_string(),
                "error": err.to_string(),
            }),
        },
        Err(err) => json!({
            "schema_version": 1,
            "generated_at_unix": timestamp_unix_secs,
            "status": "missing_trace",
            "trace_path": trace_path.display().to_string(),
            "error": err.to_string(),
        }),
    };
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}

fn build_summary_payload(timestamp_unix_secs: u64, trace: serde_json::Value) -> serde_json::Value {
    let mut grouped = BTreeMap::<String, RuleMatchSummary>::new();
    let tabs = trace
        .get("tabs")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();

    for tab in tabs {
        let tab_name = tab
            .get("tab")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let components = tab
            .get("components")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for component in components {
            let mod_name = component
                .get("mod_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let component_id = component
                .get("component_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let rule_matches = component
                .get("rule_matches")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            for rule_match in rule_matches {
                let matched = rule_match
                    .get("direct_match")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                    || rule_match
                        .get("relation_match")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                if !matched {
                    continue;
                }
                let rule_index = rule_match
                    .get("rule_index")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let source_path = rule_match
                    .get("source_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let key = format!("{rule_index}|{source_path}");
                let entry = grouped.entry(key).or_insert_with(|| RuleMatchSummary {
                    rule_index,
                    kind: rule_match
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    message: rule_match
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    source_bucket: rule_match
                        .get("source_bucket")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    source_path,
                    match_count: 0,
                    tabs: BTreeMap::new(),
                    mods: BTreeMap::new(),
                    components: BTreeMap::new(),
                });
                entry.match_count += 1;
                *entry.tabs.entry(tab_name.clone()).or_default() += 1;
                *entry.mods.entry(mod_name.clone()).or_default() += 1;
                *entry
                    .components
                    .entry(format!("{mod_name} #{component_id}"))
                    .or_default() += 1;
            }
        }
    }

    json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "status": "ok",
        "matched_rule_count": grouped.len(),
        "matched_rules": grouped.into_values().map(|row| json!({
            "rule_index": row.rule_index,
            "kind": row.kind,
            "message": row.message,
            "source_bucket": row.source_bucket,
            "source_path": row.source_path,
            "match_count": row.match_count,
            "tabs": row.tabs,
            "mods": row.mods,
            "components": row.components,
        })).collect::<Vec<_>>()
    })
}

#[derive(Default)]
struct RuleMatchSummary {
    rule_index: u64,
    kind: String,
    message: String,
    source_bucket: String,
    source_path: String,
    match_count: usize,
    tabs: BTreeMap<String, usize>,
    mods: BTreeMap<String, usize>,
    components: BTreeMap<String, usize>,
}
