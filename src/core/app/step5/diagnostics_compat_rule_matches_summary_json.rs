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
            Ok(trace) => build_summary_payload(timestamp_unix_secs, &trace),
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

fn build_summary_payload(timestamp_unix_secs: u64, trace: &serde_json::Value) -> serde_json::Value {
    let grouped = collect_rule_match_summaries(trace);

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

fn collect_rule_match_summaries(trace: &serde_json::Value) -> BTreeMap<String, RuleMatchSummary> {
    let mut grouped = BTreeMap::<String, RuleMatchSummary>::new();
    for tab in trace_array(trace, "tabs") {
        collect_tab_rule_matches(&mut grouped, &tab);
    }
    grouped
}

fn collect_tab_rule_matches(
    grouped: &mut BTreeMap<String, RuleMatchSummary>,
    tab: &serde_json::Value,
) {
    let tab_name = string_field(tab, "tab", "");
    for component in trace_array(tab, "components") {
        collect_component_rule_matches(grouped, &tab_name, &component);
    }
}

fn collect_component_rule_matches(
    grouped: &mut BTreeMap<String, RuleMatchSummary>,
    tab_name: &str,
    component: &serde_json::Value,
) {
    let mod_name = string_field(component, "mod_name", "");
    let component_id = string_field(component, "component_id", "");
    for rule_match in trace_array(component, "rule_matches") {
        if !rule_match_matched(&rule_match) {
            continue;
        }
        add_rule_match_summary(grouped, tab_name, &mod_name, &component_id, &rule_match);
    }
}

fn trace_array(value: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn string_field(value: &serde_json::Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn rule_match_matched(rule_match: &serde_json::Value) -> bool {
    rule_match
        .get("direct_match")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || rule_match
            .get("relation_match")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
}

fn add_rule_match_summary(
    grouped: &mut BTreeMap<String, RuleMatchSummary>,
    tab_name: &str,
    mod_name: &str,
    component_id: &str,
    rule_match: &serde_json::Value,
) {
    let rule_index = rule_match
        .get("rule_index")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let source_path = string_field(rule_match, "source_path", "");
    let key = format!("{rule_index}|{source_path}");
    let entry = grouped
        .entry(key)
        .or_insert_with(|| build_rule_match_summary(rule_index, source_path, rule_match));
    entry.match_count += 1;
    *entry.tabs.entry(tab_name.to_string()).or_default() += 1;
    *entry.mods.entry(mod_name.to_string()).or_default() += 1;
    *entry
        .components
        .entry(format!("{mod_name} #{component_id}"))
        .or_default() += 1;
}

fn build_rule_match_summary(
    rule_index: u64,
    source_path: String,
    rule_match: &serde_json::Value,
) -> RuleMatchSummary {
    RuleMatchSummary {
        rule_index,
        kind: string_field(rule_match, "kind", ""),
        message: string_field(rule_match, "message", ""),
        source_bucket: string_field(rule_match, "source_bucket", "unknown"),
        source_path,
        match_count: 0,
        tabs: BTreeMap::new(),
        mods: BTreeMap::new(),
        components: BTreeMap::new(),
    }
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
