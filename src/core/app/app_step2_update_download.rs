// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
    for asset in assets {
        let file_name = safe_asset_file_name(&asset.asset_name);
        let destination = archive_dir.join(file_name);
        match download_one_asset(&agent, asset, &destination) {
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

pub(super) fn safe_asset_file_name(name: &str) -> String {
    let raw = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("downloaded-mod-archive");
    let sanitized = raw
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>();
    if sanitized.trim().is_empty() {
        "downloaded-mod-archive".to_string()
    } else {
        sanitized
    }
}
