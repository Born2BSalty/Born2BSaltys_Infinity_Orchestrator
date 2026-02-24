// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use crate::platform_defaults::app_config_dir;

use super::AppDataCopySummary;

pub(super) fn copy_appdata_configs(run_dir: &Path) -> AppDataCopySummary {
    let mut summary = AppDataCopySummary::default();
    let appdata_out_dir = run_dir.join("appdata");
    let _ = fs::create_dir_all(&appdata_out_dir);
    let Some(bio_dir) = app_config_dir() else {
        summary
            .missing
            .push("BIO app-data directory could not be resolved".to_string());
        return summary;
    };

    copy_named_appdata_dir(
        &bio_dir,
        "bio",
        &appdata_out_dir,
        &mut summary.copied,
        &mut summary.missing,
    );

    if let Some(parent) = bio_dir.parent() {
        let mod_installer_dir = parent.join("mod_installer");
        copy_named_appdata_dir(
            &mod_installer_dir,
            "mod_installer",
            &appdata_out_dir,
            &mut summary.copied,
            &mut summary.missing,
        );
    } else {
        summary
            .missing
            .push("mod_installer app-data directory parent could not be resolved".to_string());
    }

    summary
}

fn copy_named_appdata_dir(
    source_dir: &Path,
    label: &str,
    out_dir: &Path,
    copied: &mut Vec<PathBuf>,
    missing: &mut Vec<String>,
) {
    if !source_dir.is_dir() {
        missing.push(format!("{label}: not found at {}", source_dir.display()));
        return;
    }

    let dest_dir = out_dir.join(label);
    if fs::create_dir_all(&dest_dir).is_err() {
        missing.push(format!(
            "{label}: failed to create destination {}",
            dest_dir.display()
        ));
        return;
    }

    let mut copied_any = false;
    copy_appdata_tree_filtered(source_dir, &dest_dir, copied, &mut copied_any);

    if !copied_any {
        missing.push(format!(
            "{label}: no copyable config files found in {}",
            source_dir.display()
        ));
    }
}

fn copy_appdata_tree_filtered(
    src_dir: &Path,
    dst_dir: &Path,
    copied: &mut Vec<PathBuf>,
    copied_any: &mut bool,
) {
    let Ok(entries) = fs::read_dir(src_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let src_path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            let next_dst = dst_dir.join(entry.file_name());
            let _ = fs::create_dir_all(&next_dst);
            copy_appdata_tree_filtered(&src_path, &next_dst, copied, copied_any);
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "json" | "toml" | "yaml" | "yml" | "log" | "txt") {
            continue;
        }

        let Some(name) = src_path.file_name() else {
            continue;
        };
        let dest = dst_dir.join(name);
        if fs::copy(&src_path, &dest).is_ok() {
            copied.push(dest);
            *copied_any = true;
        }
    }
}
