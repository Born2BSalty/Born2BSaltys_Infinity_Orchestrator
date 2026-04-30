// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::fs;
use std::path::{Path, PathBuf};

use crate::app::app_step2_update_download;
use crate::app::mod_downloads;
use crate::app::state::{Step2UpdateAsset, WizardState};

#[derive(Debug, Clone)]
pub(super) struct Step2UpdateExtractJob {
    pub(super) label: String,
    pub(super) tp_file: String,
    pub(super) aliases: Vec<String>,
    pub(super) tp2_rename: Option<mod_downloads::ModDownloadTp2Rename>,
    pub(super) subdir_require: Option<String>,
    pub(super) archive_path: PathBuf,
    pub(super) mods_root: PathBuf,
    pub(super) backup_root: PathBuf,
    pub(super) target_root: Option<PathBuf>,
    pub(super) backup_version_tag: String,
    pub(super) installed_source_ref: Option<String>,
}

pub(super) fn build_extract_jobs(
    state: &mut WizardState,
    archive_dir: &Path,
) -> Vec<Step2UpdateExtractJob> {
    let mut jobs = Vec::new();
    let mods_root = PathBuf::from(state.step1.mods_folder.trim());
    if state.step1.mods_folder.trim().is_empty() {
        state
            .step2
            .update_selected_extract_failed_sources
            .push("Mods Folder is empty".to_string());
        return jobs;
    }
    let source_load = mod_downloads::load_mod_download_sources();
    if let Some(err) = source_load.error.as_ref() {
        state
            .step2
            .update_selected_extract_failed_sources
            .push(err.clone());
    }
    for asset in &state.step2.update_selected_update_assets {
        let archive_path = archive_dir.join(app_step2_update_download::archive_file_name(asset));
        if !archive_path.exists() {
            continue;
        }
        let source = resolve_selected_source(state, &source_load, &asset.tp_file);
        let tp2_rename = source.as_ref().and_then(|source| source.tp2_rename.clone());
        let subdir_require = source.as_ref().and_then(|source| source.subdir_require.clone());
        jobs.push(Step2UpdateExtractJob {
            label: asset.label.clone(),
            tp_file: asset.tp_file.clone(),
            aliases: source
                .as_ref()
                .map(|source| source.aliases.clone())
                .unwrap_or_default(),
            tp2_rename,
            subdir_require,
            archive_path,
            mods_root: mods_root.clone(),
            backup_root: PathBuf::from(state.step1.mods_backup_folder.trim()),
            target_root: current_mod_root(state, &asset.game_tab, &asset.tp_file),
            backup_version_tag: asset.tag.clone(),
            installed_source_ref: extract_source_ref(asset),
        });
    }
    jobs
}

fn resolve_selected_source(
    state: &WizardState,
    sources: &mod_downloads::ModDownloadsLoad,
    tp_file: &str,
) -> Option<mod_downloads::ModDownloadSource> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
    sources.resolve_source(
        tp_file,
        state
            .step2
            .selected_source_ids
            .get(&tp2_key)
            .map(String::as_str),
    )
}

fn extract_source_ref(asset: &Step2UpdateAsset) -> Option<String> {
    asset.installed_source_ref.clone().or_else(|| {
        let asset_name = asset.asset_name.trim().to_ascii_lowercase();
        if asset_name.ends_with("-source.zip")
            || asset_name.ends_with("-source.tar.gz")
            || asset_name.ends_with("-source.tgz")
        {
            Some(asset.tag.clone())
        } else {
            None
        }
    })
}

fn current_mod_root(state: &WizardState, game_tab: &str, tp_file: &str) -> Option<PathBuf> {
    let mods = if game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mod_state = mods.iter().find(|mod_state| mod_state.tp_file == tp_file)?;
    let tp2_path = Path::new(mod_state.tp2_path.trim());
    let tp2_parent = tp2_path.parent()?;
    let mods_root = Path::new(state.step1.mods_folder.trim());
    Some(outer_wrapper_root(mods_root, tp2_parent))
}

fn outer_wrapper_root(mods_root: &Path, tp2_parent: &Path) -> PathBuf {
    let mut current = tp2_parent.to_path_buf();
    while let Some(parent) = current.parent() {
        if parent == mods_root || !is_single_child_wrapper(parent, &current) {
            break;
        }
        current = parent.to_path_buf();
    }
    current
}

fn is_single_child_wrapper(parent: &Path, child: &Path) -> bool {
    let Ok(entries) = fs::read_dir(parent) else {
        return false;
    };
    let mut dir_count = 0usize;
    for entry in entries {
        let Ok(entry) = entry else {
            return false;
        };
        let Ok(file_type) = entry.file_type() else {
            return false;
        };
        if file_type.is_dir() {
            dir_count += 1;
            if dir_count > 1 || entry.path() != child {
                return false;
            }
        }
    }
    dir_count == 1
}
