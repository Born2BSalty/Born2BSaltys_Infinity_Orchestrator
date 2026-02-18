// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use crate::config::options::ScanConfig;
use crate::install::weidu_scan;
use tracing::info;
use walkdir::WalkDir;

pub fn run(config: &ScanConfig) -> Result<()> {
    if let ScanConfig::Components {
        game_directory,
        filter_by_selected_language,
        options,
    } = config
    {
        info!(
            "command=scan components game_directory={game_directory:?} filter_by_selected_language={filter_by_selected_language} options={options:?}"
        );

        if options.weidu_binary.as_os_str().is_empty() {
            return Err(anyhow!("--weidu-binary is required for scan components"));
        }

        let tp2_files = collect_tp2_files(&options.mod_directories, options.depth);
        let total_tp2 = tp2_files.len();
        let mut scanned_tp2 = 0usize;
        let mut matched_langs = 0usize;
        let mut found_components = 0usize;
        let needle = filter_by_selected_language.to_ascii_lowercase();
        for tp2 in tp2_files {
            scanned_tp2 += 1;
            info!("scan components progress: {scanned_tp2}/{total_tp2} {:?}", tp2);

            let lang_entries = weidu_scan::list_languages(&options.weidu_binary, &tp2)?;
            let lang_ids: HashSet<String> = lang_entries
                .into_iter()
                .filter(|entry| entry.label.to_ascii_lowercase().contains(&needle))
                .map(|entry| entry.id)
                .collect();
            matched_langs += lang_ids.len();
            for lang_id in lang_ids {
                let components = weidu_scan::list_components(
                    &options.weidu_binary,
                    game_directory,
                    &tp2,
                    &lang_id,
                )?;
                found_components += components.len();
                for component in components {
                    println!("{:?}", component);
                }
            }
        }
        info!(
            "scan components summary: mods_scanned={} matched_langs={} components_found={}",
            scanned_tp2, matched_langs, found_components
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
