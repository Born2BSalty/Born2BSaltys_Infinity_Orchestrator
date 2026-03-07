// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use crate::config::options::ScanConfig;
use crate::install::weidu_scan;
use tracing::info;
use walkdir::WalkDir;

pub fn run(config: &ScanConfig) -> Result<()> {
    if let ScanConfig::Languages {
        filter_by_selected_language,
        options,
    } = config
    {
        info!(
            "command=scan languages filter_by_selected_language={filter_by_selected_language} options={options:?}"
        );

        if options.weidu_binary.as_os_str().is_empty() {
            return Err(anyhow!("--weidu-binary is required for scan languages"));
        }

        let tp2_files = collect_tp2_files(&options.mod_directories, options.depth);
        let total_tp2 = tp2_files.len();
        let mut scanned_tp2 = 0usize;
        let mut matched_entries = 0usize;
        let needle = filter_by_selected_language.to_ascii_lowercase();
        for tp2 in tp2_files {
            scanned_tp2 += 1;
            info!("scan languages progress: {scanned_tp2}/{total_tp2} {:?}", tp2);
            let langs = weidu_scan::list_languages(&options.weidu_binary, &tp2)?;
            let filtered = langs
                .into_iter()
                .filter(|entry| entry.label.to_ascii_lowercase().contains(&needle))
                .collect::<Vec<_>>();
            if filtered.is_empty() {
                continue;
            }
            matched_entries += filtered.len();
            for entry in filtered {
                println!("{:?} lang={} label={}", tp2, entry.id, entry.label);
            }
        }
        info!(
            "scan languages summary: mods_scanned={} language_matches={}",
            scanned_tp2, matched_entries
        );
    }
    Ok(())
}

fn collect_tp2_files(mod_root: &Path, depth: usize) -> Vec<PathBuf> {
    WalkDir::new(mod_root)
        .follow_links(true)
        .max_depth(depth)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if name.ends_with(".tp2") {
                Some(entry.path().to_path_buf())
            } else {
                None
            }
        })
        .collect()
}
