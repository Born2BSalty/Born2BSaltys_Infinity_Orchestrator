// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::app::mod_downloads::{self, ModDownloadSource, ModDownloadsLoad};
use crate::app::state::{Step2Selection, WizardState, update_selection_signature};

pub(crate) fn preview_update_selected(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &ModDownloadsLoad,
) {
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let include_all_tabs = state.step1.game_install == "EET";
    let selected_source_ids = state.step2.selected_source_ids.clone();
    state.step2.update_selected_target_game_tab = None;
    state.step2.update_selected_target_tp_file = None;

    let mut known = Vec::new();
    let mut manual = Vec::new();
    let mut unknown = Vec::new();
    let mut locked = Vec::new();
    let mut update_requests = Vec::new();
    let mut queued_tp2 = HashSet::new();
    for game_tab in if include_all_tabs {
        ["BGEE", "BG2EE"]
    } else {
        [state.step2.active_game_tab.as_str(), ""]
    } {
        if game_tab.is_empty() {
            continue;
        }
        let mods = if game_tab == "BGEE" {
            &mut state.step2.bgee_mods
        } else {
            &mut state.step2.bg2ee_mods
        };
        for mod_state in mods.iter_mut() {
            mod_state.package_marker = None;
            if !exact_log_mode {
                if !mod_state.checked
                    && !mod_state
                        .components
                        .iter()
                        .any(|component| component.checked)
                {
                    continue;
                }
                let label = if mod_state.name.trim().is_empty() {
                    mod_state.tp_file.clone()
                } else {
                    mod_state.name.clone()
                };
                if mod_state.update_locked {
                    locked.push(label);
                    continue;
                }
                let source =
                    resolve_selected_source(sources, &selected_source_ids, &mod_state.tp_file);
                if let Some(source) = source {
                    if mod_downloads::source_is_auto_resolvable(&source) {
                        let tp2_key = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
                        if queued_tp2.insert(tp2_key) {
                            queue_source_request(
                                game_tab,
                                &mod_state.tp_file,
                                &label,
                                None,
                                &source,
                                &mut update_requests,
                            );
                        }
                        known.push(label);
                    } else {
                        mod_state.package_marker = Some('!');
                        manual.push(label);
                    }
                } else {
                    mod_state.package_marker = Some('!');
                    unknown.push(label);
                }
            }
        }
    }
    let mut pending_preview = PendingLogUpdatePreview {
        known: &mut known,
        manual: &mut manual,
        unknown: &mut unknown,
        update_requests: &mut update_requests,
    };
    extend_log_pending_update_requests(
        state,
        sources,
        &selected_source_ids,
        &mut queued_tp2,
        &mut pending_preview,
    );

    let known_count = known.len();
    let manual_count = manual.len();
    let unknown_count = unknown.len();
    let locked_count = locked.len();
    let missing_count = known_count + manual_count + unknown_count;
    state.step2.update_selected_known_sources = known;
    state.step2.update_selected_manual_sources = manual;
    state.step2.update_selected_unknown_sources = unknown;
    state.step2.update_selected_update_assets.clear();
    state.step2.update_selected_update_sources.clear();
    state.step2.update_selected_locked_update_assets.clear();
    state.step2.update_selected_locked_update_sources.clear();
    state.step2.update_selected_missing_sources.clear();
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state
        .step2
        .update_selected_exact_version_failed_sources
        .clear();
    state.step2.update_selected_failed_sources.clear();
    state.step2.update_selected_check_requests.clear();
    state
        .step2
        .update_selected_exact_version_retry_requests
        .clear();
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.update_selected_check_done_count = 0;
    state.step2.update_selected_check_total_count = update_requests.len();
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
    state.step2.update_selected_last_selection_signature =
        Some(update_selection_signature(&state.step2));
    state.step2.update_selected_last_was_full_selection = true;
    if exact_log_mode {
        state.step2.exact_log_mod_list_checked = true;
    }
    state.step2.scan_status = if exact_log_mode {
        format!(
            "Check mod list: {missing_count} missing, {known_count} auto, {manual_count} manual, {unknown_count} no source"
        )
    } else {
        format!(
            "Check updates: {known_count} auto, {manual_count} manual, {unknown_count} missing, {locked_count} locked"
        )
    };
    if !update_requests.is_empty() {
        state.step2.scan_status = if exact_log_mode {
            format!("Checking missing mod sources: {}", update_requests.len())
        } else {
            format!("Checking update sources: {}", update_requests.len())
        };
        super::app_step2_update_check::start_step2_update_check(
            state,
            step2_update_check_rx,
            update_requests,
        );
    } else {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    }
}

pub(crate) fn preview_update_selected_mod(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &ModDownloadsLoad,
) {
    let (game_tab, tp_file) = if let (Some(game_tab), Some(tp_file)) = (
        state.step2.update_selected_target_game_tab.clone(),
        state.step2.update_selected_target_tp_file.clone(),
    ) {
        (game_tab, tp_file)
    } else {
        let Some(Step2Selection::Mod { game_tab, tp_file }) = state.step2.selected.clone() else {
            return;
        };
        (game_tab, tp_file)
    };
    state.step2.update_selected_target_game_tab = Some(game_tab.clone());
    state.step2.update_selected_target_tp_file = Some(tp_file.clone());
    let selected_source_ids = state.step2.selected_source_ids.clone();
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };

    let mut known = Vec::new();
    let mut manual = Vec::new();
    let mut unknown = Vec::new();
    let mut update_requests = Vec::new();

    for mod_state in mods.iter_mut() {
        if mod_state.tp_file != tp_file {
            continue;
        }
        mod_state.package_marker = None;
        let label = if mod_state.name.trim().is_empty() {
            mod_state.tp_file.clone()
        } else {
            mod_state.name.clone()
        };
        if mod_state.update_locked {
            break;
        }
        let source = resolve_selected_source(sources, &selected_source_ids, &mod_state.tp_file);
        if let Some(source) = source {
            if mod_downloads::source_is_auto_resolvable(&source) {
                queue_source_request(
                    &game_tab,
                    &mod_state.tp_file,
                    &label,
                    None,
                    &source,
                    &mut update_requests,
                );
                known.push(label);
            } else {
                mod_state.package_marker = Some('!');
                manual.push(label);
            }
        } else {
            mod_state.package_marker = Some('!');
            unknown.push(label);
        }
        break;
    }

    state.step2.update_selected_known_sources = known;
    state.step2.update_selected_manual_sources = manual;
    state.step2.update_selected_unknown_sources = unknown;
    state.step2.update_selected_update_assets.clear();
    state.step2.update_selected_update_sources.clear();
    state.step2.update_selected_locked_update_assets.clear();
    state.step2.update_selected_locked_update_sources.clear();
    state.step2.update_selected_missing_sources.clear();
    state.step2.update_selected_downloaded_sources.clear();
    state.step2.update_selected_download_failed_sources.clear();
    state
        .step2
        .update_selected_exact_version_failed_sources
        .clear();
    state.step2.update_selected_failed_sources.clear();
    state.step2.update_selected_check_requests.clear();
    state
        .step2
        .update_selected_exact_version_retry_requests
        .clear();
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.update_selected_check_done_count = 0;
    state.step2.update_selected_check_total_count = update_requests.len();
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
    state.step2.update_selected_last_selection_signature = None;
    state.step2.update_selected_last_was_full_selection = false;
    if !update_requests.is_empty() {
        state.step2.scan_status = "Checking update source: 1".to_string();
        super::app_step2_update_check::start_step2_update_check(
            state,
            step2_update_check_rx,
            update_requests,
        );
    } else {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    }
}

fn queue_source_request(
    game_tab: &str,
    tp_file: &str,
    label: &str,
    requested_version: Option<&str>,
    source: &ModDownloadSource,
    update_requests: &mut Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
) {
    if let Some(repo) = source.github.as_deref() {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: repo.to_string(),
            exact_github: source.exact_github.clone(),
            source_url: String::new(),
            channel: source.channel.clone(),
            tag: source.tag.clone(),
            branch: source.branch.clone(),
            asset: if source.tag.is_none() && source.branch.is_none() {
                source.asset.clone()
            } else {
                None
            },
            pkg: if source.tag.is_none() && source.branch.is_none() {
                mod_downloads::preferred_pkg_for_current_platform(source)
            } else {
                None
            },
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::source_is_page_archive_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::source_is_sentrizeal_download_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    } else if mod_downloads::is_direct_archive_url(&source.url) {
        update_requests.push(super::app_step2_update_check::Step2UpdateCheckRequest {
            game_tab: game_tab.to_string(),
            tp_file: tp_file.to_string(),
            label: label.to_string(),
            source_id: source.source_id.clone(),
            repo: String::new(),
            exact_github: Vec::new(),
            source_url: source.url.clone(),
            channel: None,
            tag: None,
            branch: None,
            asset: None,
            pkg: None,
            requested_version: requested_version
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
        });
    }
}

struct PendingLogUpdatePreview<'a> {
    known: &'a mut Vec<String>,
    manual: &'a mut Vec<String>,
    unknown: &'a mut Vec<String>,
    update_requests: &'a mut Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
}

fn extend_log_pending_update_requests(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    queued_tp2: &mut HashSet<String>,
    pending_preview: &mut PendingLogUpdatePreview<'_>,
) {
    let include_all_tabs = state.step1.game_install == "EET";
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let game_tab = state.step2.active_game_tab.as_str();
    for pending in &state.step2.log_pending_downloads {
        if !include_all_tabs && pending.game_tab != game_tab {
            continue;
        }
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
        if tp2_key.is_empty() || !queued_tp2.insert(tp2_key) {
            continue;
        }
        let source = resolve_selected_source(sources, selected_source_ids, &pending.tp_file);
        if let Some(source) = source {
            if mod_downloads::source_is_auto_resolvable(&source) {
                queue_source_request(
                    &pending.game_tab,
                    &pending.tp_file,
                    &pending.label,
                    if exact_log_mode {
                        pending.requested_version.as_deref()
                    } else {
                        None
                    },
                    &source,
                    pending_preview.update_requests,
                );
                pending_preview.known.push(pending.label.clone());
            } else {
                pending_preview.manual.push(pending.label.clone());
            }
        } else {
            pending_preview.unknown.push(pending.label.clone());
        }
    }
}

fn resolve_selected_source(
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    tp2: &str,
) -> Option<ModDownloadSource> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp2);
    sources.resolve_source(tp2, selected_source_ids.get(&tp2_key).map(String::as_str))
}
