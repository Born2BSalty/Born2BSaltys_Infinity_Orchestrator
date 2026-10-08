// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
use std::sync::mpsc::{Receiver, TryRecvError};

use chrono::Local;

use crate::app::app_step2_update_policy::{
    mark_update_available, mod_has_current_version, source_ref_is_update, source_ref_matches,
    version_is_update,
};
use crate::app::app_step2_update_source_refs::{
    InstalledRefLookup, RemoteFileFacts, same_remote_file,
};
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_downloads;
use crate::app::state::{
    ManualDownloadReason, ManualDownloadRequest, Step2UpdateAsset, Step2UpdateRetryRequest,
    WizardState, push_manual_download_request,
};

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
    pub(crate) release: Option<String>,
    pub(crate) asset: Option<String>,
    pub(crate) pkg: Option<String>,
    pub(crate) requested_version: Option<String>,
}
#[derive(Debug, Clone, Copy)]
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
    pub(crate) source_url: String,
    pub(crate) tag: Option<String>,
    pub(crate) source_ref: Option<String>,
    pub(crate) asset_name: Option<String>,
    pub(crate) asset_url: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) package_kind: Step2PackageKind,
    pub(crate) version_pin_overridden: Option<String>,
    pub(crate) remote_file: Option<RemoteFileFacts>,
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
    let Some(event) = next_update_check_event(state, step2_update_check_rx) else {
        return;
    };
    let merge_latest_fallback = state.step2.update_selected_merge_latest_fallback;
    match event {
        super::app_step2_update_check_worker::Step2UpdateCheckEvent::Progress(progress) => {
            update_check_progress(state, progress, merge_latest_fallback);
        }
        super::app_step2_update_check_worker::Step2UpdateCheckEvent::Finished(outcomes) => {
            finish_update_check(
                state,
                step2_update_check_rx,
                &outcomes,
                merge_latest_fallback,
            );
        }
    }
}

fn next_update_check_event(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
) -> Option<super::app_step2_update_check_worker::Step2UpdateCheckEvent> {
    let rx = step2_update_check_rx.as_ref()?;
    match rx.try_recv() {
        Ok(event) => Some(event),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            state.step2.update_selected_check_running = false;
            state.step2.update_selected_merge_latest_fallback = false;
            state.step2.update_selected_check_requests.clear();
            state.step2.scan_status = "Compare Versions failed: worker disconnected".to_string();
            *step2_update_check_rx = None;
            None
        }
    }
}

fn update_check_progress(
    state: &mut WizardState,
    progress: super::app_step2_update_check_worker::Step2UpdateCheckProgress,
    merge_latest_fallback: bool,
) {
    state.step2.update_selected_check_done_count = progress.completed;
    state.step2.update_selected_check_total_count = progress.total;
    state.step2.scan_status = if merge_latest_fallback {
        format!(
            "Checking latest fallback sources: {}/{}",
            progress.completed, progress.total
        )
    } else {
        format!(
            "Checking versions: {}/{}",
            progress.completed, progress.total
        )
    };
}

fn finish_update_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    outcomes: &[Step2UpdateCheckOutcome],
    merge_latest_fallback: bool,
) {
    *step2_update_check_rx = None;
    state.step2.update_selected_check_running = false;
    let existing_actionable = state.step2.update_selected_update_sources.len()
        + state.step2.update_selected_missing_sources.len();
    clear_previous_update_check_results(state, outcomes, merge_latest_fallback);
    state.step2.update_selected_refresh_target_game_tab = None;
    state.step2.update_selected_refresh_target_tp_file = None;
    state.step2.update_selected_check_done_count = state.step2.update_selected_check_total_count;
    let sources = mod_downloads::load_mod_download_sources();
    let lookup = InstalledRefLookup::load(state.step1.mods_folder.trim());

    for outcome in outcomes {
        apply_update_check_outcome(state, outcome, &sources, &lookup, merge_latest_fallback);
    }

    state.step2.update_selected_check_requests.clear();
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.update_selected_last_checked_at = Some(Local::now().format("%H:%M").to_string());
    state.step2.scan_status =
        update_check_finished_status(state, merge_latest_fallback, existing_actionable);
}

fn clear_previous_update_check_results(
    state: &mut WizardState,
    outcomes: &[Step2UpdateCheckOutcome],
    merge_latest_fallback: bool,
) {
    if merge_latest_fallback {
        return;
    }
    if state.step2.update_selected_refresh_target_tp_file.is_some() {
        clear_targeted_update_check_results(state, outcomes);
    } else {
        state.step2.update_selected_update_assets.clear();
        state.step2.update_selected_in_sync_assets.clear();
        state.step2.update_selected_update_sources.clear();
        state.step2.update_selected_missing_sources.clear();
        state
            .step2
            .update_selected_exact_version_failed_sources
            .clear();
        state.step2.update_selected_failed_sources.clear();
        state.step2.update_selected_remote_file_facts.clear();
        state.step2.update_selected_unverified_sources.clear();
        state
            .step2
            .update_selected_exact_version_retry_requests
            .clear();
        state
            .step2
            .update_selected_version_override_warnings
            .clear();
    }
}

fn apply_update_check_outcome(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    sources: &mod_downloads::ModDownloadsLoad,
    lookup: &InstalledRefLookup,
    merge_latest_fallback: bool,
) {
    if let Some(tag) = outcome.tag.as_deref() {
        apply_successful_update_check_outcome(
            state,
            outcome,
            tag,
            sources,
            lookup,
            merge_latest_fallback,
        );
    } else {
        let error = outcome.error.as_deref().unwrap_or("no release found");
        push_update_check_failure(
            state,
            FailedCheckContext {
                game_tab: &outcome.game_tab,
                tp_file: &outcome.tp_file,
                label: &outcome.label,
                source_url: &outcome.source_url,
            },
            error,
            merge_latest_fallback,
            sources,
        );
    }
}

pub(super) const fn reproduce_exact_gate(state: &WizardState) -> bool {
    state.modlist_auto_build_active && state.reproduce_exact
}

fn apply_successful_update_check_outcome(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    tag: &str,
    sources: &mod_downloads::ModDownloadsLoad,
    lookup: &InstalledRefLookup,
    merge_latest_fallback: bool,
) {
    store_latest_checked_version(state, &outcome.game_tab, &outcome.tp_file, tag);
    if let Some(wanted) = outcome.version_pin_overridden.as_deref() {
        state
            .step2
            .update_selected_version_override_warnings
            .push(format!("{} ({wanted} -> {tag})", outcome.label));
    }
    if let Some(remote) = outcome.remote_file.as_ref() {
        decide_direct_link(state, outcome, tag, remote, lookup);
        return;
    }
    let has_current_version = mod_has_current_version(state, &outcome.game_tab, &outcome.tp_file);
    let allow_log_missing_download =
        exact_log_missing_download_requested(state, &outcome.game_tab, &outcome.tp_file)
            && log_missing_downloads_enabled(state);
    let uses_source_snapshot = matches!(outcome.package_kind, Step2PackageKind::SourceSnapshot);
    let source_ref = outcome.source_ref.as_deref().unwrap_or(tag);
    if source_ref_matches(lookup, &outcome.tp_file, &outcome.source_id, source_ref) {
        if reproduce_exact_gate(state) {
            push_update_asset_if_available(state, outcome, tag, source_ref, uses_source_snapshot);
            state
                .step2
                .update_selected_update_sources
                .push(format!("{} ({tag})", outcome.label));
        } else {
            keep_in_sync_asset(state, outcome, tag, source_ref, uses_source_snapshot);
        }
        return;
    }
    if uses_source_snapshot && let Some(err) = sources.error.as_ref() {
        push_update_check_failure(
            state,
            FailedCheckContext {
                game_tab: &outcome.game_tab,
                tp_file: &outcome.tp_file,
                label: &outcome.label,
                source_url: &outcome.source_url,
            },
            err,
            merge_latest_fallback,
            sources,
        );
        return;
    }
    let allow_source_ref_update =
        source_ref_is_update(lookup, &outcome.tp_file, &outcome.source_id, source_ref);
    let allow_snapshot_install = uses_source_snapshot
        && !has_current_version
        && state.step1.have_weidu_logs
        && state.step1.download_archive;
    if matches!(outcome.package_kind, Step2PackageKind::SourceSnapshot)
        && !allow_source_ref_update
        && !allow_snapshot_install
        && !allow_log_missing_download
        && !has_current_version
    {
        return;
    }
    let should_apply_update_outcome = allow_source_ref_update
        || allow_snapshot_install
        || allow_log_missing_download
        || (has_current_version
            && version_is_update(state, &outcome.game_tab, &outcome.tp_file, tag));
    if !should_apply_update_outcome {
        keep_in_sync_asset(state, outcome, tag, source_ref, uses_source_snapshot);
        return;
    }
    push_update_asset_if_available(state, outcome, tag, source_ref, uses_source_snapshot);
    let entry = format!("{} ({tag})", outcome.label);
    if allow_log_missing_download {
        state.step2.update_selected_missing_sources.push(entry);
    } else {
        state.step2.update_selected_update_sources.push(entry);
    }
    if allow_source_ref_update || has_current_version {
        mark_update_available(state, &outcome.game_tab, &outcome.tp_file);
    }
}

fn decide_direct_link(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    tag: &str,
    remote: &RemoteFileFacts,
    lookup: &InstalledRefLookup,
) {
    state.step2.update_selected_remote_file_facts.insert(
        mod_downloads::normalize_mod_download_tp2(&outcome.tp_file),
        remote.clone(),
    );
    let source_ref = outcome.source_ref.as_deref().unwrap_or(tag);
    match lookup
        .archive(&outcome.tp_file)
        .map(|record| same_remote_file(record, remote))
    {
        Some(Some(true)) => {
            if reproduce_exact_gate(state) {
                push_update_asset_if_available(state, outcome, tag, source_ref, false);
                state
                    .step2
                    .update_selected_update_sources
                    .push(format!("{} ({tag})", outcome.label));
            } else {
                keep_in_sync_asset(state, outcome, tag, source_ref, false);
            }
        }
        Some(Some(false)) => fetch_direct_link(state, outcome, tag, source_ref),
        None | Some(None) => {
            fetch_direct_link(state, outcome, tag, source_ref);
            state
                .step2
                .update_selected_unverified_sources
                .push(format!("{}: not verified", outcome.label));
        }
    }
}

fn fetch_direct_link(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    tag: &str,
    source_ref: &str,
) {
    push_update_asset_if_available(state, outcome, tag, source_ref, false);
    let entry = format!("{} ({tag})", outcome.label);
    if exact_log_missing_download_requested(state, &outcome.game_tab, &outcome.tp_file)
        && log_missing_downloads_enabled(state)
    {
        state.step2.update_selected_missing_sources.push(entry);
    } else {
        state.step2.update_selected_update_sources.push(entry);
    }
    mark_update_available(state, &outcome.game_tab, &outcome.tp_file);
}

fn push_update_asset_if_available(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    tag: &str,
    source_ref: &str,
    uses_source_snapshot: bool,
) {
    let (Some(asset_name), Some(asset_url)) = (&outcome.asset_name, &outcome.asset_url) else {
        return;
    };
    state
        .step2
        .update_selected_update_assets
        .push(Step2UpdateAsset {
            game_tab: outcome.game_tab.clone(),
            tp_file: outcome.tp_file.clone(),
            label: outcome.label.clone(),
            source_id: outcome.source_id.clone(),
            tag: tag.to_string(),
            asset_name: asset_name.clone(),
            asset_url: asset_url.clone(),
            installed_source_ref: uses_source_snapshot.then(|| source_ref.to_string()),
        });
}

fn keep_in_sync_asset(
    state: &mut WizardState,
    outcome: &Step2UpdateCheckOutcome,
    tag: &str,
    source_ref: &str,
    uses_source_snapshot: bool,
) {
    let (Some(asset_name), Some(asset_url)) = (&outcome.asset_name, &outcome.asset_url) else {
        return;
    };
    let tp2_key = mod_downloads::normalize_mod_download_tp2(&outcome.tp_file);
    let in_sync = &mut state.step2.update_selected_in_sync_assets;
    in_sync.retain(|asset| {
        asset.game_tab != outcome.game_tab
            || mod_downloads::normalize_mod_download_tp2(&asset.tp_file) != tp2_key
    });
    in_sync.push(Step2UpdateAsset {
        game_tab: outcome.game_tab.clone(),
        tp_file: outcome.tp_file.clone(),
        label: outcome.label.clone(),
        source_id: outcome.source_id.clone(),
        tag: tag.to_string(),
        asset_name: asset_name.clone(),
        asset_url: asset_url.clone(),
        installed_source_ref: uses_source_snapshot.then(|| source_ref.to_string()),
    });
}

fn update_check_finished_status(
    state: &WizardState,
    merge_latest_fallback: bool,
    existing_actionable: usize,
) -> String {
    let updates = state.step2.update_selected_update_sources.len();
    let missing = state.step2.update_selected_missing_sources.len();
    let failed = state
        .step2
        .update_selected_exact_version_failed_sources
        .len()
        + state.step2.update_selected_failed_sources.len();
    if merge_latest_fallback {
        format!(
            "Latest fallback finished: {} added, {failed} failed",
            (updates + missing).saturating_sub(existing_actionable)
        )
    } else if state.step1.installs_exactly_from_weidu_logs() {
        format!("Check mod list finished: {missing} downloadable missing, {failed} failed")
    } else if log_missing_downloads_enabled(state) && !state.step2.log_pending_downloads.is_empty()
    {
        format!(
            "Compare Versions finished: {updates} version changes, {missing} missing, {failed} failed"
        )
    } else {
        format!("Compare Versions finished: {updates} version changes, {failed} failed")
    }
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
    } else if mod_downloads::is_direct_archive_url(&request.source_url) {
        super::app_step2_update_direct_archive::check_direct_archive(&request, &|url| {
            super::app_step2_update_direct_archive::probe_remote_file(agent, url)
        })
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
        source_url: request.source_url,
        tag: None,
        source_ref: None,
        asset_name: None,
        asset_url: None,
        error: Some(error.to_string()),
        package_kind,
        version_pin_overridden: None,
        remote_file: None,
    }
}

fn clear_targeted_update_check_results(
    state: &mut WizardState,
    outcomes: &[Step2UpdateCheckOutcome],
) {
    for outcome in outcomes {
        clear_update_check_result_for_mod(
            state,
            &outcome.game_tab,
            &outcome.tp_file,
            &outcome.label,
        );
    }
}

pub(crate) fn clear_update_check_result_for_mod(
    state: &mut WizardState,
    game_tab: &str,
    tp_file: &str,
    label: &str,
) {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
    state.step2.update_selected_update_assets.retain(|asset| {
        asset.game_tab != game_tab
            || mod_downloads::normalize_mod_download_tp2(&asset.tp_file) != tp2_key
    });
    state.step2.update_selected_in_sync_assets.retain(|asset| {
        asset.game_tab != game_tab
            || mod_downloads::normalize_mod_download_tp2(&asset.tp_file) != tp2_key
    });
    state
        .step2
        .update_selected_update_sources
        .retain(|entry| !entry.starts_with(&format!("{label} (")));
    state
        .step2
        .update_selected_missing_sources
        .retain(|entry| !entry.starts_with(&format!("{label} (")));
    state
        .step2
        .update_selected_exact_version_failed_sources
        .retain(|entry| !entry.starts_with(&format!("{label}:")));
    state
        .step2
        .update_selected_failed_sources
        .retain(|entry| !entry.starts_with(&format!("{label}:")));
    state
        .step2
        .update_selected_unverified_sources
        .retain(|entry| !entry.starts_with(&format!("{label}:")));
    state
        .step2
        .update_selected_remote_file_facts
        .remove(&tp2_key);
    state
        .step2
        .update_selected_exact_version_retry_requests
        .retain(|request| {
            request.game_tab != game_tab
                || mod_downloads::normalize_mod_download_tp2(&request.tp_file) != tp2_key
        });
    state
        .step2
        .update_selected_downloaded_sources
        .retain(|entry| !entry.starts_with(label));
    state
        .step2
        .update_selected_download_failed_sources
        .retain(|entry| !entry.starts_with(&format!("{label}:")));
    state
        .step2
        .update_selected_extracted_sources
        .retain(|entry| !entry.starts_with(label));
    state
        .step2
        .update_selected_extract_failed_sources
        .retain(|entry| !entry.starts_with(&format!("{label}:")));
    state
        .step2
        .update_selected_version_override_warnings
        .retain(|entry| !entry.starts_with(&format!("{label} (")));
}

fn store_latest_checked_version(state: &mut WizardState, game_tab: &str, tp_file: &str, tag: &str) {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
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

fn log_missing_downloads_enabled(state: &WizardState) -> bool {
    state.step2.whole_folder_check_active
        || state.step1.installs_exactly_from_weidu_logs()
        || state.step1.bootstraps_from_weidu_logs()
        || ((state.step2.log_apply.review_edit_bgee_applied
            || state.step2.log_apply.review_edit_bg2ee_applied)
            && !state.step2.log_pending_downloads.is_empty())
}

#[derive(Clone, Copy)]
struct FailedCheckContext<'a> {
    game_tab: &'a str,
    tp_file: &'a str,
    label: &'a str,
    source_url: &'a str,
}

fn push_update_check_failure(
    state: &mut WizardState,
    ctx: FailedCheckContext<'_>,
    error: &str,
    merge_latest_fallback: bool,
    sources: &mod_downloads::ModDownloadsLoad,
) {
    let FailedCheckContext {
        game_tab,
        tp_file,
        label,
        source_url,
    } = ctx;
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
        let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
        let aliases = sources
            .resolve_source(
                tp_file,
                state
                    .step2
                    .selected_source_ids
                    .get(&tp2_key)
                    .map(String::as_str),
            )
            .map(|source| source.declared_tp2_names())
            .unwrap_or_default();
        push_manual_download_request(
            &mut state.step2.update_selected_manual_downloads,
            ManualDownloadRequest {
                game_tab: game_tab.to_string(),
                tp_file: tp_file.to_string(),
                label: label.to_string(),
                source_id: String::new(),
                page_url: source_url.to_string(),
                reason: ManualDownloadReason::SourceCheckFailed(error.to_string()),
                aliases,
                display_name: String::new(),
            },
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::app_step2_update_source_refs::InstalledArchiveRecord;

    #[test]
    fn reproduce_exact_gate_fires_only_in_reproduce_mode() {
        let reproduce = WizardState::<bool> {
            modlist_auto_build_active: true,
            reproduce_exact: true,
            ..Default::default()
        };
        assert!(
            reproduce_exact_gate(&reproduce),
            "both flags set: gate active, reproduce-exact path pushes the asset"
        );

        let legacy = WizardState::<bool> {
            modlist_auto_build_active: true,
            reproduce_exact: false,
            ..Default::default()
        };
        assert!(
            !reproduce_exact_gate(&legacy),
            "legacy import (reproduce_exact false): gate inactive, drop behavior unchanged"
        );

        let normal = WizardState::<bool> {
            modlist_auto_build_active: false,
            reproduce_exact: true,
            ..Default::default()
        };
        assert!(
            !reproduce_exact_gate(&normal),
            "no auto-build (modlist_auto_build_active false): gate inactive, behavior unchanged"
        );

        assert!(!reproduce_exact_gate(&WizardState::<bool>::default()));
    }

    #[test]
    fn override_outcome_lands_in_assets_and_warnings() {
        use crate::app::mod_downloads::ModDownloadsLoad;
        use crate::app::state::{Step2ComponentState, Step2ModState};

        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient = AmbientRestore(crate::app::mod_downloads::active_modlist_dir());
        crate::app::mod_downloads::set_active_modlist_dir(None);
        let _config_guard = ConfigDirGuard::new("override_outcome");
        let mut state = WizardState::<bool>::default();
        state.step2.bgee_mods = vec![Step2ModState {
            name: "ISNF".to_string(),
            tp_file: "ISNF.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: true,
            hidden_components: Vec::new(),
            components: vec![Step2ComponentState {
                component_id: "100".to_string(),
                label: "ISNF".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: "~ISNF.tp2~ #0 #100 // 6.5.5".to_string(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                is_meta_mode_component: false,
                disabled: false,
                compat_kind: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_graph: None,
                compat_evidence: None,
                disabled_reason: None,
                checked: true,
                selected_order: Some(1),
            }],
        }];

        let outcome = Step2UpdateCheckOutcome {
            game_tab: "BGEE".to_string(),
            tp_file: "ISNF.tp2".to_string(),
            label: "ISNF".to_string(),
            source_id: "weaselmods".to_string(),
            source_url: String::new(),
            tag: Some("6.5.6".to_string()),
            source_ref: None,
            asset_name: Some("isnf-6.5.6.zip".to_string()),
            asset_url: Some("https://example.com/isnf-6.5.6.zip".to_string()),
            error: None,
            package_kind: Step2PackageKind::PageArchive,
            version_pin_overridden: Some("6.5.5".to_string()),
            remote_file: None,
        };

        let sources = ModDownloadsLoad::default();
        let lookup = InstalledRefLookup::load(state.step1.mods_folder.trim());
        apply_update_check_outcome(&mut state, &outcome, &sources, &lookup, false);

        assert!(
            !state.step2.update_selected_update_assets.is_empty(),
            "override outcome must push asset to update_selected_update_assets"
        );
        assert_eq!(
            state.step2.update_selected_update_assets[0].tag, "6.5.6",
            "asset must carry the current (served) version tag"
        );
        assert_eq!(
            state.step2.update_selected_version_override_warnings,
            vec!["ISNF (6.5.5 -> 6.5.6)"],
            "override warning must use compact format: label (pinned -> served)"
        );
    }

    struct ConfigDirGuard(std::path::PathBuf);

    impl ConfigDirGuard {
        fn new(label: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "bio_update_check_{}_{}_{label}",
                std::process::id(),
                id
            ));
            std::fs::create_dir_all(&path).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(path.clone()));
            Self(path)
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finished_check_records_the_time() {
        let _config_guard = ConfigDirGuard::new("finished_check_records_the_time");
        let mut state = WizardState::<bool>::default();
        assert!(state.step2.update_selected_last_checked_at.is_none());
        let mut rx: Option<
            Receiver<super::super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
        > = None;

        finish_update_check(&mut state, &mut rx, &[], false);

        let recorded = state
            .step2
            .update_selected_last_checked_at
            .expect("timestamp recorded after a finished check");
        assert_eq!(recorded.len(), 5);
        assert_eq!(recorded.as_bytes()[2], b':');
    }

    struct AmbientRestore(Option<std::path::PathBuf>);

    impl Drop for AmbientRestore {
        fn drop(&mut self) {
            crate::app::mod_downloads::set_active_modlist_dir(self.0.take());
        }
    }

    fn multikits_state() -> WizardState<bool> {
        use crate::app::state::{Step2ComponentState, Step2ModState};

        let mut state = WizardState::<bool>::default();
        state.step2.bgee_mods = vec![Step2ModState {
            name: "A7-MultiKits".to_string(),
            tp_file: "A7-MultiKits.tp2".to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: true,
            hidden_components: Vec::new(),
            components: vec![Step2ComponentState {
                component_id: "0".to_string(),
                label: "A7-MultiKits".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: "~A7-MultiKits.tp2~ #0 #0 // Multi-kits: VERSION ~1.1~".to_string(),
                prompt_summary: None,
                prompt_events: Vec::new(),
                is_meta_mode_component: false,
                disabled: false,
                compat_kind: None,
                compat_source: None,
                compat_related_mod: None,
                compat_related_component: None,
                compat_graph: None,
                compat_evidence: None,
                disabled_reason: None,
                checked: true,
                selected_order: Some(1),
            }],
        }];
        state
    }

    fn multikits_release_outcome() -> Step2UpdateCheckOutcome {
        Step2UpdateCheckOutcome {
            game_tab: "BGEE".to_string(),
            tp_file: "A7-MultiKits.tp2".to_string(),
            label: "A7-MultiKits".to_string(),
            source_id: "argent77".to_string(),
            source_url: String::new(),
            tag: Some("v1.1".to_string()),
            source_ref: None,
            asset_name: Some("win-A7-MultiKits-v1.1.zip".to_string()),
            asset_url: Some(
                "https://example.com/argent77/A7-MultiKits/win-A7-MultiKits-v1.1.zip".to_string(),
            ),
            error: None,
            package_kind: Step2PackageKind::ReleaseAsset,
            version_pin_overridden: None,
            remote_file: None,
        }
    }

    fn check_multikits_release(label: &str, refs_file: Option<&str>) -> WizardState<bool> {
        check_multikits_release_times(label, refs_file, 1)
    }

    fn check_multikits_release_times(
        label: &str,
        refs_file: Option<&str>,
        times: usize,
    ) -> WizardState<bool> {
        check_multikits_outcome(label, refs_file, &multikits_release_outcome(), times)
    }

    fn check_multikits_outcome(
        label: &str,
        refs_file: Option<&str>,
        outcome: &Step2UpdateCheckOutcome,
        times: usize,
    ) -> WizardState<bool> {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient = AmbientRestore(crate::app::mod_downloads::active_modlist_dir());
        crate::app::mod_downloads::set_active_modlist_dir(None);
        let config_guard = ConfigDirGuard::new(label);
        let refs_path = crate::app::app_step2_update_source_refs::installed_source_refs_path();
        assert!(refs_path.starts_with(&config_guard.0));
        if let Some(content) = refs_file {
            std::fs::write(&refs_path, content).unwrap();
        }
        let mut state = multikits_state();
        assert!(mod_has_current_version(&state, "BGEE", "A7-MultiKits.tp2"));
        assert!(!version_is_update(
            &state,
            "BGEE",
            "A7-MultiKits.tp2",
            "v1.1"
        ));
        let sources = crate::app::mod_downloads::ModDownloadsLoad::default();
        let lookup = InstalledRefLookup::load(state.step1.mods_folder.trim());
        for _ in 0..times {
            apply_update_check_outcome(&mut state, outcome, &sources, &lookup, false);
        }
        state
    }

    fn assert_single_in_sync_multikits_asset(state: &WizardState<bool>) {
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        let outcome = multikits_release_outcome();
        let in_sync = &state.step2.update_selected_in_sync_assets;
        assert_eq!(in_sync.len(), 1);
        assert_eq!(Some(in_sync[0].tag.as_str()), outcome.tag.as_deref());
        assert_eq!(
            Some(in_sync[0].asset_name.as_str()),
            outcome.asset_name.as_deref()
        );
        assert_eq!(
            Some(in_sync[0].asset_url.as_str()),
            outcome.asset_url.as_deref()
        );
        assert!(in_sync[0].installed_source_ref.is_none());
    }

    #[test]
    fn in_sync_outcome_keeps_its_asset_for_a_refetch() {
        let state = check_multikits_release(
            "in_sync_keeps_asset",
            Some("[refs]\na7-multikits = \"v1.1\"\n\n[sources]\na7-multikits = \"argent77\"\n"),
        );
        assert_single_in_sync_multikits_asset(&state);
    }

    #[test]
    fn version_match_outcome_keeps_its_asset_for_a_refetch() {
        let state = check_multikits_release("version_match_keeps_asset", None);
        assert_single_in_sync_multikits_asset(&state);
    }

    #[test]
    fn a_repeat_check_replaces_the_in_sync_entry() {
        let state = check_multikits_release_times("repeat_check_in_sync", None, 2);
        assert_single_in_sync_multikits_asset(&state);
    }

    #[test]
    fn in_sync_snapshot_outcome_keeps_its_commit_ref() {
        let commit_ref = "commit@abc1234abc1234abc1234abc1234abc1234abc1";
        let outcome = Step2UpdateCheckOutcome {
            tag: Some("commit-abc1234".to_string()),
            source_ref: Some(commit_ref.to_string()),
            asset_name: Some("A7-MultiKits-abc1234.zip".to_string()),
            asset_url: Some("https://example.com/argent77/A7-MultiKits/abc1234.zip".to_string()),
            package_kind: Step2PackageKind::SourceSnapshot,
            ..multikits_release_outcome()
        };
        let refs_file = format!(
            "[refs]\na7-multikits = \"{commit_ref}\"\n\n[sources]\na7-multikits = \"argent77\"\n"
        );
        let state = check_multikits_outcome("in_sync_snapshot_ref", Some(&refs_file), &outcome, 1);
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        let in_sync = &state.step2.update_selected_in_sync_assets;
        assert_eq!(in_sync.len(), 1);
        assert_eq!(in_sync[0].installed_source_ref.as_deref(), Some(commit_ref));
    }

    struct FolderRefsRoot(std::path::PathBuf);

    impl FolderRefsRoot {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let root = Self(std::env::temp_dir().join(format!(
                "bio_folderrefs_check_test_{}_{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            )));
            std::fs::create_dir_all(&root.0).unwrap();
            crate::platform_defaults::set_config_dir_override(Some(root.0.clone()));
            root
        }
    }

    impl Drop for FolderRefsRoot {
        fn drop(&mut self) {
            crate::platform_defaults::clear_config_dir_override_if(&self.0);
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const CDTWEAKS_SHA: &str = "7649ced0123456789abcdef0123456789abcdef0";

    fn cdtweaks_state(mods_folder: &str) -> WizardState<bool> {
        let mut state = multikits_state();
        let mod_state = &mut state.step2.bgee_mods[0];
        mod_state.name = "cdtweaks".to_string();
        mod_state.tp_file = "cdtweaks.tp2".to_string();
        mod_state.components[0].label = "cdtweaks".to_string();
        mod_state.components[0].raw_line = "~cdtweaks.tp2~ #0 #0 // Tweaks: v18".to_string();
        state.step1.mods_folder = mods_folder.to_string();
        state
    }

    fn cdtweaks_branch_outcome() -> Step2UpdateCheckOutcome {
        let source_ref = format!("master@{CDTWEAKS_SHA}");
        Step2UpdateCheckOutcome {
            game_tab: "BGEE".to_string(),
            tp_file: "cdtweaks.tp2".to_string(),
            label: "cdtweaks".to_string(),
            source_id: "cdtweaks-master".to_string(),
            source_url: String::new(),
            tag: Some(source_ref.clone()),
            source_ref: Some(source_ref),
            asset_name: Some("cdtweaks-master.zip".to_string()),
            asset_url: Some("https://example.com/cdtweaks/master.zip".to_string()),
            error: None,
            package_kind: Step2PackageKind::SourceSnapshot,
            version_pin_overridden: None,
            remote_file: None,
        }
    }

    fn check_cdtweaks_with_folder_record(folder_ref: &str) -> WizardState<bool> {
        let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _ambient = AmbientRestore(crate::app::mod_downloads::active_modlist_dir());
        crate::app::mod_downloads::set_active_modlist_dir(None);
        let root = FolderRefsRoot::new();
        let mods = root.0.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        let mods = mods.to_string_lossy().into_owned();
        let list_path = crate::app::app_step2_update_source_refs::installed_source_refs_path();
        assert!(list_path.starts_with(&root.0));
        assert!(!list_path.exists());
        let folder_path =
            crate::app::app_step2_update_source_refs::mods_folder_refs_path(&mods).unwrap();
        assert!(folder_path.starts_with(&root.0));
        std::fs::create_dir_all(folder_path.parent().unwrap()).unwrap();
        let key = crate::app::mod_downloads::normalize_mod_download_tp2("cdtweaks.tp2");
        std::fs::write(
            &folder_path,
            format!("[refs]\n{key} = \"{folder_ref}\"\n\n[sources]\n{key} = \"cdtweaks-master\"\n"),
        )
        .unwrap();
        let mut state = cdtweaks_state(&mods);
        assert!(mod_has_current_version(&state, "BGEE", "cdtweaks.tp2"));
        let sources = crate::app::mod_downloads::ModDownloadsLoad::default();
        let lookup = InstalledRefLookup::load(state.step1.mods_folder.trim());
        apply_update_check_outcome(
            &mut state,
            &cdtweaks_branch_outcome(),
            &sources,
            &lookup,
            false,
        );
        state
    }

    #[test]
    fn recorded_branch_ref_makes_a_matching_release_a_fetch() {
        let state = check_multikits_release(
            "branch_ref_release_fetch",
            Some(
                "[refs]\na7-multikits = \"devel@6ae6ae0bd3d2b6275a24aa9c72abc54e2122f805\"\n\n[sources]\na7-multikits = \"argent77\"\n",
            ),
        );
        let assets = &state.step2.update_selected_update_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].tag, "v1.1");
        let sources = &state.step2.update_selected_update_sources;
        assert_eq!(sources.len(), 1);
        assert!(sources[0].ends_with("(v1.1)"));
        assert_eq!(state.step2.bgee_mods[0].package_marker, Some('+'));
    }

    #[test]
    fn release_matching_the_recorded_tag_stays_in_sync() {
        let state = check_multikits_release(
            "release_tag_in_sync",
            Some("[refs]\na7-multikits = \"v1.1\"\n\n[sources]\na7-multikits = \"argent77\"\n"),
        );
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        assert_eq!(state.step2.update_selected_update_sources.len(), 0);
    }

    #[test]
    fn no_recorded_ref_falls_back_to_the_version_comparison() {
        let state = check_multikits_release("no_recorded_ref", None);
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        assert_eq!(state.step2.update_selected_update_sources.len(), 0);
    }

    #[test]
    fn a_folder_record_makes_a_branch_source_in_sync_on_a_list_that_never_fetched_it() {
        let in_sync = check_cdtweaks_with_folder_record(&format!("master@{CDTWEAKS_SHA}"));
        assert_eq!(in_sync.step2.update_selected_update_assets.len(), 0);
        assert_eq!(in_sync.step2.update_selected_update_sources.len(), 0);

        let moved =
            check_cdtweaks_with_folder_record("master@0123456789abcdef0123456789abcdef01234567");
        let assets = &moved.step2.update_selected_update_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].tp_file, "cdtweaks.tp2");
        assert_eq!(moved.step2.bgee_mods[0].package_marker, Some('+'));
    }

    #[test]
    fn recorded_ref_from_another_source_is_ignored() {
        let state = check_multikits_release(
            "other_source_ref",
            Some(
                "[refs]\na7-multikits = \"devel@6ae6ae0bd3d2b6275a24aa9c72abc54e2122f805\"\n\n[sources]\na7-multikits = \"someone-else\"\n",
            ),
        );
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        assert_eq!(state.step2.update_selected_update_sources.len(), 0);
    }

    #[test]
    fn failed_check_outcome_carries_source_url() {
        let request = Step2UpdateCheckRequest {
            game_tab: "BGEE".to_string(),
            tp_file: "ascension/setup-ascension.tp2".to_string(),
            label: "Ascension".to_string(),
            source_id: String::new(),
            repo: String::new(),
            exact_github: vec![],
            source_url: "https://www.nexusmods.com/baldursgateenhancededition/mods/1".to_string(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            release: None,
            asset: None,
            pkg: None,
            requested_version: None,
        };
        let expected_source_url = request.source_url.clone();
        let outcome = failed_outcome(request, "x");
        assert_eq!(outcome.source_url, expected_source_url);
    }

    fn questpack_request(source_url: &str) -> Step2UpdateCheckRequest {
        Step2UpdateCheckRequest {
            game_tab: "BGEE".to_string(),
            tp_file: "d0questpack/setup-d0questpack.tp2".to_string(),
            label: "d0questpack".to_string(),
            source_id: "pocket-plane-group".to_string(),
            repo: String::new(),
            exact_github: vec![],
            source_url: source_url.to_string(),
            channel: None,
            tag: None,
            commit: None,
            branch: None,
            release: None,
            asset: None,
            pkg: None,
            requested_version: None,
        }
    }

    #[test]
    fn direct_archive_request_resolves_from_the_link_and_the_probe() {
        let request = questpack_request("https://pocketplane.net/mods/questpack-v35-win.zip");
        let probe = |_: &str| Ok(recorded_date_facts());
        let outcome =
            super::super::app_step2_update_direct_archive::check_direct_archive(&request, &probe);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.remote_file, Some(recorded_date_facts()));
        assert_eq!(outcome.tag.as_deref(), Some("questpack-v35-win"));
        assert_eq!(outcome.asset_name.as_deref(), Some("questpack-v35-win.zip"));
        assert_eq!(
            outcome.asset_url.as_deref(),
            Some("https://pocketplane.net/mods/questpack-v35-win.zip")
        );
        assert!(matches!(
            outcome.package_kind,
            Step2PackageKind::ReleaseAsset
        ));
    }

    #[test]
    fn unresolvable_request_still_fails() {
        let request =
            questpack_request("https://www.nexusmods.com/baldursgateenhancededition/mods/1");
        let outcome = check_latest_release_for_worker(&ureq::AgentBuilder::new().build(), request);
        assert_eq!(
            outcome.error.as_deref(),
            Some("source is not auto-resolvable")
        );
    }

    const QUESTPACK_TP2: &str = "d0questpack/setup-d0questpack.tp2";
    const QUESTPACK_SIZE: u64 = 21_707_615;
    const RECORDED_DATE: &str = "Thu, 10 Sep 2020 17:25:37 GMT";

    fn recorded_date_facts() -> RemoteFileFacts {
        RemoteFileFacts {
            size: Some(QUESTPACK_SIZE),
            last_modified: Some(RECORDED_DATE.to_string()),
            etag: None,
        }
    }

    fn questpack_record(last_modified: Option<&str>) -> InstalledArchiveRecord {
        InstalledArchiveRecord {
            name: "questpack-v35-win.zip".to_string(),
            size: QUESTPACK_SIZE,
            hash: "00ff00ff00ff00ff00ff00ff00ff00ff".to_string(),
            last_modified: last_modified.map(str::to_string),
            etag: None,
        }
    }

    fn questpack_state() -> WizardState<bool> {
        let mut state = multikits_state();
        let mod_state = &mut state.step2.bgee_mods[0];
        mod_state.name = "d0questpack".to_string();
        mod_state.tp_file = QUESTPACK_TP2.to_string();
        mod_state.components[0].label = "d0questpack".to_string();
        mod_state.components[0].raw_line =
            "~D0QUESTPACK/SETUP-D0QUESTPACK.TP2~ #0 #0 // Quest Pack: v3.5".to_string();
        state
    }

    fn questpack_outcome(remote: RemoteFileFacts) -> Step2UpdateCheckOutcome {
        Step2UpdateCheckOutcome {
            game_tab: "BGEE".to_string(),
            tp_file: QUESTPACK_TP2.to_string(),
            label: "d0questpack".to_string(),
            source_id: "pocket-plane-group".to_string(),
            source_url: String::new(),
            tag: Some("questpack-v35-win".to_string()),
            source_ref: None,
            asset_name: Some("questpack-v35-win.zip".to_string()),
            asset_url: Some("https://pocketplane.net/mods/questpack-v35-win.zip".to_string()),
            error: None,
            package_kind: Step2PackageKind::ReleaseAsset,
            version_pin_overridden: None,
            remote_file: Some(remote),
        }
    }

    fn check_questpack(
        record: Option<InstalledArchiveRecord>,
        remote: RemoteFileFacts,
    ) -> WizardState<bool> {
        let mut list = crate::app::app_step2_update_source_refs::ModSourceRefsFile::default();
        if let Some(record) = record {
            list.archives.insert("d0questpack".to_string(), record);
        }
        let lookup = InstalledRefLookup::from_files(None, list);
        let sources = crate::app::mod_downloads::ModDownloadsLoad::default();
        let mut state = questpack_state();
        apply_update_check_outcome(
            &mut state,
            &questpack_outcome(remote),
            &sources,
            &lookup,
            false,
        );
        state
    }

    fn assert_questpack_in_sync(state: &WizardState<bool>) {
        assert_eq!(state.step2.update_selected_update_assets.len(), 0);
        assert_eq!(state.step2.update_selected_update_sources.len(), 0);
        assert_eq!(state.step2.update_selected_unverified_sources.len(), 0);
        assert_eq!(state.step2.update_selected_in_sync_assets.len(), 1);
        assert_eq!(state.step2.bgee_mods[0].package_marker, None);
    }

    fn assert_questpack_fetch(state: &WizardState<bool>) {
        let assets = &state.step2.update_selected_update_assets;
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].tag, "questpack-v35-win");
        assert_eq!(assets[0].asset_name, "questpack-v35-win.zip");
        assert_eq!(
            state.step2.update_selected_update_sources,
            vec!["d0questpack (questpack-v35-win)".to_string()]
        );
        assert_eq!(state.step2.bgee_mods[0].package_marker, Some('+'));
    }

    #[test]
    fn direct_link_with_the_recorded_date_stays_in_sync() {
        let state = check_questpack(
            Some(questpack_record(Some(RECORDED_DATE))),
            recorded_date_facts(),
        );
        assert_questpack_in_sync(&state);
    }

    #[test]
    fn direct_link_with_a_newer_date_is_a_fetch() {
        let newer = RemoteFileFacts {
            last_modified: Some("Fri, 11 Sep 2020 17:25:37 GMT".to_string()),
            ..recorded_date_facts()
        };
        let state = check_questpack(Some(questpack_record(Some(RECORDED_DATE))), newer);
        assert_questpack_fetch(&state);
        assert_eq!(state.step2.update_selected_unverified_sources.len(), 0);
    }

    #[test]
    fn direct_link_record_without_a_date_compares_size() {
        let same_size = check_questpack(Some(questpack_record(None)), recorded_date_facts());
        assert_questpack_in_sync(&same_size);

        let resized = RemoteFileFacts {
            size: Some(QUESTPACK_SIZE + 1),
            ..recorded_date_facts()
        };
        let other_size = check_questpack(Some(questpack_record(None)), resized);
        assert_questpack_fetch(&other_size);
        assert_eq!(other_size.step2.update_selected_unverified_sources.len(), 0);
    }

    #[test]
    fn direct_link_without_a_record_is_a_fetch_marked_not_verified() {
        let state = check_questpack(None, recorded_date_facts());
        assert_questpack_fetch(&state);
        assert_eq!(
            state.step2.update_selected_unverified_sources,
            vec!["d0questpack: not verified".to_string()]
        );
    }

    #[test]
    fn a_log_pending_direct_link_without_a_record_is_counted_missing() {
        let lookup = InstalledRefLookup::from_files(
            None,
            crate::app::app_step2_update_source_refs::ModSourceRefsFile::default(),
        );
        let sources = crate::app::mod_downloads::ModDownloadsLoad::default();
        let mut state = questpack_state();
        state.step2.whole_folder_check_active = true;
        state
            .step2
            .log_pending_downloads
            .push(crate::app::state::Step2LogPendingDownload {
                game_tab: "BGEE".to_string(),
                tp_file: QUESTPACK_TP2.to_string(),
                label: "d0questpack".to_string(),
                requested_version: None,
            });

        apply_update_check_outcome(
            &mut state,
            &questpack_outcome(recorded_date_facts()),
            &sources,
            &lookup,
            false,
        );

        assert_eq!(state.step2.update_selected_update_assets.len(), 1);
        assert_eq!(
            state.step2.update_selected_missing_sources,
            vec!["d0questpack (questpack-v35-win)".to_string()]
        );
        assert_eq!(state.step2.update_selected_update_sources.len(), 0);
        assert_eq!(
            state.step2.update_selected_unverified_sources,
            vec!["d0questpack: not verified".to_string()]
        );
    }

    #[test]
    fn direct_link_facts_are_remembered_for_the_extract_job() {
        let state = check_questpack(
            Some(questpack_record(Some(RECORDED_DATE))),
            recorded_date_facts(),
        );
        assert_eq!(
            state
                .step2
                .update_selected_remote_file_facts
                .get("d0questpack"),
            Some(&recorded_date_facts())
        );
        assert_eq!(state.step2.update_selected_remote_file_facts.len(), 1);
    }

    #[test]
    fn a_new_check_forgets_the_remembered_facts() {
        let mut full = check_questpack(None, recorded_date_facts());
        assert_ne!(full.step2.update_selected_remote_file_facts.len(), 0);
        clear_previous_update_check_results(&mut full, &[], false);
        assert_eq!(full.step2.update_selected_remote_file_facts.len(), 0);
        assert_eq!(full.step2.update_selected_unverified_sources.len(), 0);

        let mut targeted = check_questpack(None, recorded_date_facts());
        targeted.step2.update_selected_refresh_target_tp_file = Some(QUESTPACK_TP2.to_string());
        clear_previous_update_check_results(
            &mut targeted,
            &[questpack_outcome(recorded_date_facts())],
            false,
        );
        assert_eq!(targeted.step2.update_selected_remote_file_facts.len(), 0);
        assert_eq!(targeted.step2.update_selected_unverified_sources.len(), 0);
    }
}
