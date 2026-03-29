// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

use crate::ui::compat_rules::inspect_compat_rules_inventory;

pub(super) fn write_compat_rule_inventory_json(
    run_dir: &Path,
    timestamp_unix_secs: u64,
) -> Result<PathBuf> {
    let out_path = run_dir.join("compat_rule_inventory.json");
    let inventory = inspect_compat_rules_inventory();
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "default_path": inventory.default_path,
        "user_path": inventory.user_path,
        "total_loaded_rules": inventory.total_loaded_rules,
        "files": inventory.files.iter().map(|file| json!({
            "role": file.role,
            "path": file.path,
            "exists": file.exists,
            "parse_status": file.parse_status,
            "schema_version": file.schema_version,
            "total_rules": file.total_rules,
            "enabled_rules": file.enabled_rules,
            "loaded_rules": file.loaded_rules,
            "error": file.error,
        })).collect::<Vec<_>>()
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
