// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
use std::sync::mpsc::{Receiver, TryRecvError};

use crate::app::app_step2_update_policy::{
    mark_update_available, mod_has_current_version, source_ref_is_update, source_ref_matches,
    version_is_update,
};
use crate::app::mod_downloads;
use crate::app::state::{Step2UpdateAsset, Step2UpdateRetryRequest, WizardState};

#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateCheckRequest {
    pub(crate) game_tab: String,
    pub(crate) tp_file: String,
    pub(crate) label: String,
    pub(crate) source_id: String,
    pub(crate) repo: String,
    pub(crate) exact_github: Vec<String>,
    pub(crate) source_url: String,
    pub(crate) channel: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) commit: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) pkg: Option<String>,
    pub(crate) requested_version: Option<String>,
}
#[derive(Debug, Clone)]
pub(crate) enum Step2PackageKind {
    ReleaseAsset,
    PageArchive,
    SourceSnapshot,
}
#[derive(Debug, Clone)]
pub(crate) struct Step2UpdateCheckOutcome {
    pub(crate) game_tab: String,
    pub(crate) tp_file: String,
    pub(crate) label: String,
    pub(crate) source_id: String,
    pub(crate) tag: Option<String>,
    pub(crate) source_ref: Option<String>,
    pub(crate) asset_name: Option<String>,
    pub(crate) asset_url: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) package_kind: Step2PackageKind,
}
pub(crate) fn start_step2_update_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    requests: Vec<Step2UpdateCheckRequest>,
) {
    state.step2.update_selected_check_requests = requests
        .iter()
        .map(|request| Step2UpdateRetryRequest {
            game_tab: request.game_tab.clone(),
            tp_file: request.tp_file.clone(),
            label: request.label.clone(),
            source_id: request.source_id.clone(),
            repo: request.repo.clone(),
            source_url: request.source_url.clone(),
            channel: request.channel.clone(),
            tag: request.tag.clone(),
            commit: request.commit.clone(),
            branch: request.branch.clone(),
            asset: request.asset.clone(),
            pkg: request.pkg.clone(),
        })
        .collect();
    if requests.is_empty() {
        *step2_update_check_rx = None;
        state.step2.update_selected_check_running = false;
        return;
    }
    *step2_update_check_rx =
        Some(super::app_step2_update_check_worker::spawn_update_check_worker(requests));
    state.step2.update_selected_check_running = true;
}
pub(crate) fn poll_step2_update_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
) {
    let Some(rx) = step2_update_check_rx.as_ref() else {
        return;
    };
    let event = match rx.try_recv() {
        Ok(event) => Some(event),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.step2.update_selected_check_running = false;
            state.step2.update_selected_merge_latest_fallback = false;
            state.step2.update_selected_check_requests.clear();
            state.step2.scan_status = "Check updates failed: worker disconnected".to_string();
            *step2_update_check_rx = None;
            return;
        }
    };
    let Some(event) = event else {
        return;
    };

    let merge_latest_fallback = state.step2.update_selected_merge_latest_fallback;
    if let super::app_step2_update_check_worker::Step2UpdateCheckEvent::Progress(progress) = event {
        state.step2.update_selected_check_done_count = progress.completed;
        state.step2.update_selected_check_total_count = progress.total;
        state.step2.scan_status = if merge_latest_fallback {
            format!(
                "Checking latest fallback sources: {}/{}",
                progress.completed, progress.total
            )
        } else {
            format!(
                "Checking updates: {}/{}",
                progress.completed, progress.total
            )
        };
        return;
    }
    let super::app_step2_update_check_worker::Step2UpdateCheckEvent::Finished(outcomes) = event
    else {
        return;
    };
    *step2_update_check_rx = None;
    state.step2.update_selected_check_running = false;
    let existing_actionable = state.step2.update_selected_update_sources.len()
        + state.step2.update_selected_missing_sources.len();
    if !merge_latest_fallback {
        state.step2.update_selected_update_assets.clear();
        state.step2.update_selected_update_sources.clear();
        state.step2.update_selected_missing_sources.clear();
        state
            .step2
            .update_selected_exact_version_failed_sources
            .clear();
        state.step2.update_selected_failed_sources.clear();
        state
            .step2
            .update_selected_exact_version_retry_requests
            .clear();
    }
    state.step2.update_selected_check_done_count = state.step2.update_selected_check_total_count;
    let sources = mod_downloads::load_mod_download_sources();

    for outcome in outcomes {
        if let Some(tag) = outcome.tag.clone() {
            store_latest_checked_version(state, &outcome.game_tab, &outcome.tp_file, &tag);
            let has_current_version =
                mod_has_current_version(state, &outcome.game_tab, &outcome.tp_file);
            let is_missing_download =
                exact_log_missing_download_requested(state, &outcome.game_tab, &outcome.tp_file);
            let allow_log_missing_download = is_missing_download
                && (state.step1.installs_exactly_from_weidu_logs()
                    || state.step1.bootstraps_from_weidu_logs());
            let uses_source_snapshot =
                matches!(outcome.package_kind, Step2PackageKind::SourceSnapshot);
            let source_ref = outcome.source_ref.clone().unwrap_or_else(|| tag.clone());
            if uses_source_snapshot && let Some(err) = sources.error.as_ref() {
                push_update_check_failure(
                    state,
                    &outcome.game_tab,
                    &outcome.tp_file,
                    &outcome.label,
                    err,
                    merge_latest_fallback,
                );
                continue;
            }
            let allow_source_ref_update =
                uses_source_snapshot && source_ref_is_update(&outcome.tp_file, &source_ref);
            let allow_snapshot_install = uses_source_snapshot
                && !has_current_version
                && state.step1.have_weidu_logs
                && state.step1.download_archive
                && !source_ref_matches(&outcome.tp_file, &source_ref);
            if matches!(outcome.package_kind, Step2PackageKind::SourceSnapshot)
                && !allow_source_ref_update
                && !allow_snapshot_install
                && !allow_log_missing_download
                && !has_current_version
            {
                continue;
            }
            if !allow_source_ref_update && !allow_snapshot_install && !allow_log_missing_download {
                if !has_current_version {
                    continue;
                }
                if !version_is_update(state, &outcome.game_tab, &outcome.tp_file, &tag) {
                    continue;
                }
            }
            if let (Some(asset_name), Some(asset_url)) = (outcome.asset_name, outcome.asset_url) {
                state
                    .step2
                    .update_selected_update_assets
                    .push(Step2UpdateAsset {
                        game_tab: outcome.game_tab.clone(),
                        tp_file: outcome.tp_file.clone(),
                        label: outcome.label.clone(),
                        source_id: outcome.source_id.clone(),
                        tag: tag.clone(),
                        asset_name,
                        asset_url,
                        installed_source_ref: uses_source_snapshot.then_some(source_ref),
                    });
            }
            let entry = format!("{} ({tag})", outcome.label);
            if allow_log_missing_download {
                state.step2.update_selected_missing_sources.push(entry);
            } else {
                state.step2.update_selected_update_sources.push(entry);
            }
            if allow_source_ref_update {
                mark_update_available(state, &outcome.game_tab, &outcome.tp_file);
                continue;
            }
            if has_current_version {
                mark_update_available(state, &outcome.game_tab, &outcome.tp_file);
            }
        } else {
            let error = outcome
                .error
                .unwrap_or_else(|| "no release found".to_string());
            push_update_check_failure(
                state,
                &outcome.game_tab,
                &outcome.tp_file,
                &outcome.label,
                &error,
                merge_latest_fallback,
            );
        }
    }

    state.step2.update_selected_check_requests.clear();
    state.step2.update_selected_merge_latest_fallback = false;
    let updates = state.step2.update_selected_update_sources.len();
    let missing = state.step2.update_selected_missing_sources.len();
    let failed = state
        .step2
        .update_selected_exact_version_failed_sources
        .len()
        + state.step2.update_selected_failed_sources.len();
    state.step2.scan_status = if merge_latest_fallback {
        format!(
            "Latest fallback finished: {} added, {failed} failed",
            (updates + missing).saturating_sub(existing_actionable)
        )
    } else if state.step1.installs_exactly_from_weidu_logs() {
        format!("Check mod list finished: {missing} downloadable missing, {failed} failed")
    } else if state.step1.bootstraps_from_weidu_logs()
        && !state.step2.log_pending_downloads.is_empty()
    {
        format!("Check updates finished: {updates} updates, {missing} missing, {failed} failed")
    } else {
        format!("Check updates finished: {updates} updates, {failed} failed")
    };
}

pub(super) fn check_latest_release_for_worker(
    agent: &ureq::Agent,
    request: Step2UpdateCheckRequest,
) -> Step2UpdateCheckOutcome {
    if !request.repo.trim().is_empty() {
        super::app_step2_update_github::check_github_download_page(agent, &request)
    } else if mod_downloads::source_is_weaselmods_page_url(&request.source_url) {
        super::app_step2_update_weaselmods::check_weaselmods_download_page(agent, &request)
    } else if mod_downloads::source_is_morpheus_mart_page_url(&request.source_url) {
        super::app_step2_update_morpheus_mart::check_morpheus_mart_download_page(agent, &request)
    } else {
        failed_outcome(request, "source is not auto-resolvable")
    }
}

pub(super) fn failed_outcome(
    request: Step2UpdateCheckRequest,
    error: &str,
) -> Step2UpdateCheckOutcome {
    let package_kind = if mod_downloads::source_is_page_archive_url(&request.source_url) {
        Step2PackageKind::PageArchive
    } else {
        Step2PackageKind::ReleaseAsset
    };
    Step2UpdateCheckOutcome {
        game_tab: request.game_tab,
        tp_file: request.tp_file,
        label: request.label,
        source_id: request.source_id,
        tag: None,
        source_ref: None,
        asset_name: None,
        asset_url: None,
        error: Some(error.to_string()),
        package_kind,
    }
}

fn store_latest_checked_version(state: &mut WizardState, game_tab: &str, tp_file: &str, tag: &str) {
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };
    if let Some(mod_state) = mods
        .iter_mut()
        .find(|mod_state| mod_state.tp_file == tp_file)
    {
        mod_state.latest_checked_version = Some(tag.to_string());
    }
}

fn exact_log_missing_download_requested(
    state: &WizardState,
    game_tab: &str,
    tp_file: &str,
) -> bool {
    let requested_tp2 = mod_downloads::normalize_mod_download_tp2(tp_file);
    state.step2.log_pending_downloads.iter().any(|pending| {
        pending.game_tab == game_tab
            && mod_downloads::normalize_mod_download_tp2(&pending.tp_file) == requested_tp2
    })
}

fn push_update_check_failure(
    state: &mut WizardState,
    game_tab: &str,
    tp_file: &str,
    label: &str,
    error: &str,
    merge_latest_fallback: bool,
) {
    let entry = format!("{label}: {error}");
    if error.starts_with("exact version not found:") {
        state
            .step2
            .update_selected_exact_version_failed_sources
            .push(entry);
        if !merge_latest_fallback {
            push_exact_version_retry_request(state, game_tab, tp_file);
        }
    } else {
        state.step2.update_selected_failed_sources.push(entry);
    }
}

fn push_exact_version_retry_request(state: &mut WizardState, game_tab: &str, tp_file: &str) {
    let Some(request) = state
        .step2
        .update_selected_check_requests
        .iter()
        .find(|request| request.game_tab == game_tab && request.tp_file == tp_file)
        .cloned()
    else {
        return;
    };
    if state
        .step2
        .update_selected_exact_version_retry_requests
        .iter()
        .any(|existing| {
            existing.game_tab == request.game_tab && existing.tp_file == request.tp_file
        })
    {
        return;
    }
    state
        .step2
        .update_selected_exact_version_retry_requests
        .push(request);
}
