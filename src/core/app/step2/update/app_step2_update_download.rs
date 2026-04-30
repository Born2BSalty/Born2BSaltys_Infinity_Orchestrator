// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use crate::app::state::{Step2UpdateAsset, WizardState};

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateDownloadResult {
    pub(crate) downloaded: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) fn start_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadResult>>,
) {
    if state.step2.update_selected_download_running {
        return;
    }
    if !state.step1.download_archive {
        state.step2.scan_status = "Download Archive is disabled in Step 1".to_string();
        return;
    }
    let archive_dir = state.step1.mods_archive_folder.trim();
    if archive_dir.is_empty() {
        state.step2.scan_status = "Mods Archive folder is empty".to_string();
        return;
    }
    let assets = state.step2.update_selected_update_assets.clone();
    if assets.is_empty() {
        state.step2.scan_status = "No update archives to download".to_string();
        return;
    }

    let archive_dir = PathBuf::from(archive_dir);
    let (tx, rx) = mpsc::channel::<Step2UpdateDownloadResult>();
    *step2_update_download_rx = Some(rx);
    state.step2.update_selected_download_running = true;
    state.step2.update_selected_extract_running = false;
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.scan_status = format!("Downloading updates: {}", assets.len());

    thread::spawn(move || {
        let result = download_update_assets(&archive_dir, &assets);
        let _ = tx.send(result);
    });
}

pub(crate) fn poll_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadResult>>,
    step2_update_extract_rx: &mut Option<
        Receiver<super::app_step2_update_extract::Step2UpdateExtractResult>,
    >,
) {
    let Some(rx) = step2_update_download_rx.as_ref() else {
        return;
    };
    let result = match rx.try_recv() {
        Ok(result) => Some(result),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.step2.update_selected_download_running = false;
            state.step2.scan_status = "Download updates failed: worker disconnected".to_string();
            *step2_update_download_rx = None;
            return;
        }
    };
    let Some(result) = result else {
        return;
    };

    *step2_update_download_rx = None;
    state.step2.update_selected_download_running = false;
    state.step2.update_selected_downloaded_sources = result.downloaded;
    state.step2.update_selected_download_failed_sources = result.failed;
    let downloaded = state.step2.update_selected_downloaded_sources.len();
    let failed = state.step2.update_selected_download_failed_sources.len();
    state.step2.scan_status =
        format!("Download updates finished: {downloaded} downloaded, {failed} failed");
    super::app_step2_update_extract::start_step2_update_extract(state, step2_update_extract_rx);
}

fn download_update_assets(
    archive_dir: &Path,
    assets: &[Step2UpdateAsset],
) -> Step2UpdateDownloadResult {
    let mut result = Step2UpdateDownloadResult {
        downloaded: Vec::new(),
        failed: Vec::new(),
    };
    if let Err(err) = fs::create_dir_all(archive_dir) {
        result.failed.push(format!("Mods Archive: {err}"));
        return result;
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(Duration::from_secs(120))
        .build();
    let mut cached_results = BTreeMap::<String, Result<(), String>>::new();
    for asset in assets {
        let file_name = archive_file_name(asset);
        let destination = archive_dir.join(file_name);
        let cache_key = format!("{}|{}", destination.display(), asset.asset_url);
        let download_result = if let Some(existing) = cached_results.get(&cache_key) {
            existing.clone()
        } else {
            let result = download_one_asset(&agent, asset, &destination);
            cached_results.insert(cache_key, result.clone());
            result
        };
        match download_result {
            Ok(()) => {
                result
                    .downloaded
                    .push(format!("{} -> {}", asset.label, destination.display()))
            }
            Err(err) => result.failed.push(format!("{}: {err}", asset.label)),
        }
    }
    result
}

fn download_one_asset(
    agent: &ureq::Agent,
    asset: &Step2UpdateAsset,
    destination: &Path,
) -> Result<(), String> {
    let response = agent
        .get(&asset.asset_url)
        .set("User-Agent", "BIO-update-download")
        .call()
        .map_err(|err| err.to_string())?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination).map_err(|err| err.to_string())?;
    io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) fn archive_file_name(asset: &Step2UpdateAsset) -> String {
    let tp2 = safe_archive_segment(&tp2_archive_name(&asset.tp_file));
    let source = safe_archive_segment(&asset.source_id);
    let tag = safe_archive_segment(&asset.tag);
    let ext = archive_extension(&asset.asset_name);
    format!("{tp2}__{source}__{tag}{ext}")
}

fn tp2_archive_name(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let file = replaced.rsplit('/').next().unwrap_or(&replaced).trim();
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .or_else(|| without_ext.strip_prefix("setup_"))
        .unwrap_or(without_ext)
        .to_string()
}

fn archive_extension(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for ext in [".tar.gz", ".tar.bz2", ".tar.xz", ".zip", ".7z", ".rar", ".tgz", ".tbz2", ".txz"] {
        if lower.ends_with(ext) {
            return ext.to_string();
        }
    }
    ".zip".to_string()
}

fn safe_archive_segment(value: &str) -> String {
    let sanitized = value
        .trim()
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '@' => '-',
            _ if ch.is_control() || ch.is_whitespace() => '-',
            _ => ch,
        })
        .collect::<String>();
    let sanitized = sanitized.trim_matches([' ', '.', '-']).trim();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized.to_string()
    }
}
