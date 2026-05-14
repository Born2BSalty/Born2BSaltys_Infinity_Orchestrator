// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use crate::app::mod_downloads;
use crate::app::state::{Step2UpdateAsset, WizardState};

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateDownloadResult {
    pub(crate) downloaded: Vec<String>,
    pub(crate) failed: Vec<String>,
}

pub(crate) enum Step2UpdateDownloadEvent {
    Progress { completed: usize, total: usize },
    Finished(Step2UpdateDownloadResult),
}

pub(crate) fn start_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
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
    let (tx, rx) = mpsc::channel::<Step2UpdateDownloadEvent>();
    *step2_update_download_rx = Some(rx);
    state.step2.update_selected_download_running = true;
    state.step2.update_selected_extract_running = false;
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state.step2.update_selected_extracted_sources.clear();
    state.step2.update_selected_extract_failed_sources.clear();
    state.step2.scan_status = format!("Downloading updates: 0/{}", assets.len());

    thread::spawn(move || {
        let result = download_update_assets(&archive_dir, &assets, &tx);
        let _ = tx.send(Step2UpdateDownloadEvent::Finished(result));
    });
}

pub(crate) fn poll_step2_update_download(
    state: &mut WizardState,
    step2_update_download_rx: &mut Option<Receiver<Step2UpdateDownloadEvent>>,
    step2_update_extract_rx: &mut Option<
        Receiver<super::app_step2_update_extract::Step2UpdateExtractEvent>,
    >,
) {
    let Some(rx) = step2_update_download_rx.as_ref() else {
        return;
    };
    let event = match rx.try_recv() {
        Ok(event) => Some(event),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.step2.update_selected_download_running = false;
            state.step2.scan_status = "Download updates failed: worker disconnected".to_string();
            *step2_update_download_rx = None;
            return;
        }
    };
    let Some(event) = event else {
        return;
    };
    let Step2UpdateDownloadEvent::Finished(result) = event else {
        if let Step2UpdateDownloadEvent::Progress { completed, total } = event {
            state.step2.scan_status = format!("Downloading updates: {completed}/{total}");
        }
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
    tx: &Sender<Step2UpdateDownloadEvent>,
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
    let total = assets.len();
    for (index, asset) in assets.iter().enumerate() {
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
        let _ = tx.send(Step2UpdateDownloadEvent::Progress {
            completed: index + 1,
            total,
        });
    }
    result
}

fn download_one_asset(
    agent: &ureq::Agent,
    asset: &Step2UpdateAsset,
    destination: &Path,
) -> Result<(), String> {
    let is_sentrizeal = mod_downloads::source_is_sentrizeal_download_url(&asset.asset_url);
    let mut request = agent
        .get(&asset.asset_url)
        .set("User-Agent", "BIO-update-download");
    if is_sentrizeal {
        request = request.set("Referer", "https://www.sentrizeal.com/pst_7.htm");
    }
    let response = request.call().map_err(|err| err.to_string())?;
    if is_sentrizeal {
        let content_type = response
            .header("Content-Type")
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if content_type.starts_with("text/html") {
            return Err(
                "Sentrizeal download returned no archive bytes; missing Referer or blocked response"
                    .to_string(),
            );
        }
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        if bytes.is_empty() {
            return Err(
                "Sentrizeal download returned no archive bytes; missing Referer or blocked response"
                    .to_string(),
            );
        }
        fs::write(destination, bytes).map_err(|err| err.to_string())?;
        return Ok(());
    }
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination).map_err(|err| err.to_string())?;
    io::copy(&mut reader, &mut file).map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) fn archive_file_name(asset: &Step2UpdateAsset) -> String {
    let tp2 = safe_archive_segment(&tp2_archive_name(&asset.tp_file));
    let source = safe_archive_segment(&asset.source_id);
    let tag = safe_archive_segment(&asset.tag);
    let ext = archive_extension(&asset.asset_name);
    format!("{tp2}__{source}__{tag}{ext}")
}

fn tp2_archive_name(tp_file: &str) -> String {
    let replaced = tp_file.replace('\\', "/");
    let file = replaced.rsplit('/').next().unwrap_or(&replaced).trim();
    let lower = file.to_ascii_lowercase();
    let without_ext = lower.strip_suffix(".tp2").unwrap_or(&lower);
    without_ext.to_string()
}

fn archive_extension(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    for ext in [
        ".tar.gz", ".tar.bz2", ".tar.xz", ".zip", ".7z", ".rar", ".tgz", ".tbz2", ".txz",
    ] {
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
