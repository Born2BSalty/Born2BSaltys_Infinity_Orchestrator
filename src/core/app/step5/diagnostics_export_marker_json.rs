// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::json;

pub(super) fn write_export_marker_json(
    run_dir: &Path,
    timestamp_unix_secs: u64,
    written_paths: &[PathBuf],
) -> Result<PathBuf> {
    let mut files = written_paths
        .iter()
        .filter_map(|p| p.strip_prefix(run_dir).ok())
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();

    let out_path = run_dir.join("export_ok.json");
    let payload = json!({
        "schema_version": 1,
        "generated_at_unix": timestamp_unix_secs,
        "status": "ok",
        "file_count": files.len(),
        "files": files
    });
    fs::write(&out_path, serde_json::to_string_pretty(&payload)?)?;
    Ok(out_path)
}
