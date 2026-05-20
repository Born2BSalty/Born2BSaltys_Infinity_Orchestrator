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
    let mut grouped = BTreeMap::<String, RuleMatchSummary>::new();
    collect_rule_matches(trace, &mut grouped);
    summary_payload(timestamp_unix_secs, grouped)
}

fn summary_payload(
    timestamp_unix_secs: u64,
    grouped: BTreeMap<String, RuleMatchSummary>,
) -> serde_json::Value {
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

fn collect_rule_matches(
    trace: &serde_json::Value,
    grouped: &mut BTreeMap<String, RuleMatchSummary>,
) {
    let Some(tabs) = trace.get("tabs").and_then(serde_json::Value::as_array) else {
        return;
    };
    for tab in tabs {
        let tab_name = json_str(tab, "tab", "").to_string();
        let Some(components) = tab.get("components").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for component in components {
            collect_component_matches(grouped, &tab_name, component);
        }
    }
}

fn collect_component_matches(
    grouped: &mut BTreeMap<String, RuleMatchSummary>,
    tab_name: &str,
    component: &serde_json::Value,
) {
    let mod_name = json_str(component, "mod_name", "").to_string();
    let component_id = json_str(component, "component_id", "").to_string();
    let Some(rule_matches) = component
        .get("rule_matches")
        .and_then(serde_json::Value::as_array)
    else {
        return;
    };
    for rule_match in rule_matches {
        if rule_match_is_matched(rule_match) {
            record_rule_match(grouped, tab_name, &mod_name, &component_id, rule_match);
        }
    }
}

fn rule_match_is_matched(rule_match: &serde_json::Value) -> bool {
    rule_match
        .get("direct_match")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || rule_match
            .get("relation_match")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
}

fn record_rule_match(
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
    let source_path = json_str(rule_match, "source_path", "").to_string();
    let key = format!("{rule_index}|{source_path}");
    let entry = grouped
        .entry(key)
        .or_insert_with(|| new_rule_match_summary(rule_index, &source_path, rule_match));
    entry.match_count += 1;
    *entry.tabs.entry(tab_name.to_string()).or_default() += 1;
    *entry.mods.entry(mod_name.to_string()).or_default() += 1;
    *entry
        .components
        .entry(format!("{mod_name} #{component_id}"))
        .or_default() += 1;
}

fn new_rule_match_summary(
    rule_index: u64,
    source_path: &str,
    rule_match: &serde_json::Value,
) -> RuleMatchSummary {
    RuleMatchSummary {
        rule_index,
        kind: json_str(rule_match, "kind", "").to_string(),
        message: json_str(rule_match, "message", "").to_string(),
        source_bucket: json_str(rule_match, "source_bucket", "unknown").to_string(),
        source_path: source_path.to_string(),
        match_count: 0,
        tabs: BTreeMap::new(),
        mods: BTreeMap::new(),
        components: BTreeMap::new(),
    }
}

fn json_str<'a>(value: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or(default)
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
