// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use crate::app::mod_downloads::{self, ModDownloadSource, ModDownloadsLoad};
use crate::app::state::{Step2ModState, Step2Selection, WizardState, update_selection_signature};

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

    let mut preview = UpdatePreviewWork::default();
    collect_selected_update_preview(
        state,
        sources,
        &selected_source_ids,
        exact_log_mode,
        include_all_tabs,
        &mut preview,
    );
    extend_log_pending_update_requests(state, sources, &selected_source_ids, &mut preview);
    finalize_selected_update_preview(state, step2_update_check_rx, exact_log_mode, preview);
}

#[derive(Default)]
struct UpdatePreviewWork {
    known: Vec<String>,
    manual: Vec<String>,
    unknown: Vec<String>,
    locked: Vec<String>,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
    queued_tp2: HashSet<String>,
}

fn collect_selected_update_preview(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    exact_log_mode: bool,
    include_all_tabs: bool,
    preview: &mut UpdatePreviewWork,
) {
    for game_tab in if include_all_tabs {
        ["BGEE", "BG2EE"]
    } else {
        [state.step2.active_game_tab.as_str(), ""]
    } {
        if game_tab.is_empty() {
            continue;
        }
        let tab_mods = if game_tab == "BGEE" {
            &mut state.step2.bgee_mods
        } else {
            &mut state.step2.bg2ee_mods
        };
        for mod_state in tab_mods.iter_mut() {
            collect_selected_mod_preview(
                game_tab,
                mod_state,
                sources,
                selected_source_ids,
                exact_log_mode,
                preview,
            );
        }
    }
}

fn collect_selected_mod_preview(
    game_tab: &str,
    mod_state: &mut Step2ModState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    exact_log_mode: bool,
    preview: &mut UpdatePreviewWork,
) {
    mod_state.package_marker = None;
    if exact_log_mode || !mod_has_checked_selection(mod_state) {
        return;
    }
    let label = mod_preview_label(mod_state);
    if mod_state.update_locked {
        preview.locked.push(label);
        return;
    }
    add_mod_source_preview(
        game_tab,
        mod_state,
        &label,
        None,
        sources,
        selected_source_ids,
        preview,
    );
}

fn mod_has_checked_selection(mod_state: &Step2ModState) -> bool {
    mod_state.checked
        || mod_state
            .components
            .iter()
            .any(|component| component.checked)
}

fn mod_preview_label(mod_state: &Step2ModState) -> String {
    if mod_state.name.trim().is_empty() {
        mod_state.tp_file.clone()
    } else {
        mod_state.name.clone()
    }
}

fn add_mod_source_preview(
    game_tab: &str,
    mod_state: &mut Step2ModState,
    label: &str,
    requested_version: Option<&str>,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut UpdatePreviewWork,
) {
    let source = resolve_selected_source(sources, selected_source_ids, &mod_state.tp_file);
    if let Some(source) = source {
        if mod_downloads::source_is_auto_resolvable(&source) {
            let tp2_key = mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
            if preview.queued_tp2.insert(tp2_key) {
                queue_source_request(
                    game_tab,
                    &mod_state.tp_file,
                    label,
                    requested_version,
                    &source,
                    &mut preview.update_requests,
                );
            }
            preview.known.push(label.to_string());
        } else {
            mod_state.package_marker = Some('!');
            preview.manual.push(label.to_string());
        }
    } else {
        mod_state.package_marker = Some('!');
        preview.unknown.push(label.to_string());
    }
}

fn finalize_selected_update_preview(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    exact_log_mode: bool,
    preview: UpdatePreviewWork,
) {
    let known_count = preview.known.len();
    let manual_count = preview.manual.len();
    let unknown_count = preview.unknown.len();
    let locked_count = preview.locked.len();
    let missing_count = known_count + manual_count + unknown_count;
    state.step2.update_selected_known_sources = preview.known;
    state.step2.update_selected_manual_sources = preview.manual;
    state.step2.update_selected_unknown_sources = preview.unknown;
    reset_update_preview_state(state, preview.update_requests.len());
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
    start_preview_update_check(
        state,
        step2_update_check_rx,
        preview.update_requests,
        exact_log_mode,
    );
}

fn reset_update_preview_state(state: &mut WizardState, request_count: usize) {
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
    state.step2.update_selected_check_total_count = request_count;
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
}

fn start_preview_update_check(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
    exact_log_mode: bool,
) {
    if update_requests.is_empty() {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    } else {
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
    }
}

pub(crate) fn preview_update_selected_mod(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    sources: &ModDownloadsLoad,
) {
    let Some((game_tab, tp_file)) = selected_update_target(state) else {
        return;
    };
    state.step2.update_selected_target_game_tab = Some(game_tab.clone());
    state.step2.update_selected_target_tp_file = Some(tp_file.clone());
    let selected_source_ids = state.step2.selected_source_ids.clone();
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };

    let mut preview = UpdatePreviewWork::default();
    let mut target_label = collect_selected_mod_target_preview(
        &game_tab,
        &tp_file,
        mods,
        sources,
        &selected_source_ids,
        &mut preview,
    );
    if target_label.is_none() {
        target_label = collect_pending_target_preview(
            state,
            sources,
            &selected_source_ids,
            &game_tab,
            &tp_file,
            &mut preview,
        );
    }

    let update_requests = replace_single_mod_preview_results(
        state,
        &game_tab,
        &tp_file,
        target_label.as_deref(),
        preview,
    );
    finalize_single_update_preview(state, step2_update_check_rx, update_requests);
}

fn selected_update_target(state: &WizardState) -> Option<(String, String)> {
    if let (Some(game_tab), Some(tp_file)) = (
        state.step2.update_selected_target_game_tab.clone(),
        state.step2.update_selected_target_tp_file.clone(),
    ) {
        Some((game_tab, tp_file))
    } else {
        let Step2Selection::Mod { game_tab, tp_file } = state.step2.selected.clone()? else {
            return None;
        };
        Some((game_tab, tp_file))
    }
}

fn collect_selected_mod_target_preview(
    game_tab: &str,
    tp_file: &str,
    mods: &mut [Step2ModState],
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut UpdatePreviewWork,
) -> Option<String> {
    for mod_state in mods {
        if mod_state.tp_file != tp_file {
            continue;
        }
        mod_state.package_marker = None;
        let label = mod_preview_label(mod_state);
        if !mod_state.update_locked {
            add_mod_source_preview(
                game_tab,
                mod_state,
                &label,
                None,
                sources,
                selected_source_ids,
                preview,
            );
        }
        return Some(label);
    }
    None
}

fn collect_pending_target_preview(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    game_tab: &str,
    tp_file: &str,
    preview: &mut UpdatePreviewWork,
) -> Option<String> {
    let tp2_key = mod_downloads::normalize_mod_download_tp2(tp_file);
    for pending in &state.step2.log_pending_downloads {
        if pending.game_tab != game_tab
            || mod_downloads::normalize_mod_download_tp2(&pending.tp_file) != tp2_key
        {
            continue;
        }
        add_pending_source_preview(
            pending,
            state.step1.installs_exactly_from_weidu_logs(),
            sources,
            selected_source_ids,
            preview,
        );
        return Some(pending.label.clone());
    }
    None
}

fn add_pending_source_preview(
    pending: &crate::app::state::Step2LogPendingDownload,
    exact_log_mode: bool,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut UpdatePreviewWork,
) {
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
                &mut preview.update_requests,
            );
            preview.known.push(pending.label.clone());
        } else {
            preview.manual.push(pending.label.clone());
        }
    } else {
        preview.unknown.push(pending.label.clone());
    }
}

fn replace_single_mod_preview_results(
    state: &mut WizardState,
    game_tab: &str,
    tp_file: &str,
    target_label: Option<&str>,
    preview: UpdatePreviewWork,
) -> Vec<super::app_step2_update_check::Step2UpdateCheckRequest> {
    let UpdatePreviewWork {
        known,
        manual,
        unknown,
        update_requests,
        ..
    } = preview;
    state.step2.update_selected_confirm_latest_fallback_open = false;
    state.step2.update_selected_merge_latest_fallback = false;
    state.step2.update_selected_check_done_count = 0;
    state.step2.update_selected_check_total_count = update_requests.len();
    state.step2.update_selected_popup_open = true;
    state.step2.update_selected_has_run = true;
    state.step2.update_selected_last_selection_signature = None;
    state.step2.update_selected_last_was_full_selection = false;
    let Some(label) = target_label else {
        return update_requests;
    };
    replace_label_entries(&mut state.step2.update_selected_known_sources, label, known);
    replace_label_entries(
        &mut state.step2.update_selected_manual_sources,
        label,
        manual,
    );
    replace_label_entries(
        &mut state.step2.update_selected_unknown_sources,
        label,
        unknown,
    );
    super::app_step2_update_check::clear_update_check_result_for_mod(
        state, game_tab, tp_file, label,
    );
    update_requests
}

fn finalize_single_update_preview(
    state: &mut WizardState,
    step2_update_check_rx: &mut Option<
        Receiver<super::app_step2_update_check_worker::Step2UpdateCheckEvent>,
    >,
    update_requests: Vec<super::app_step2_update_check::Step2UpdateCheckRequest>,
) {
    if update_requests.is_empty() {
        state.step2.update_selected_check_running = false;
        *step2_update_check_rx = None;
    } else {
        state.step2.scan_status = "Checking update source: 1".to_string();
        super::app_step2_update_check::start_step2_update_check(
            state,
            step2_update_check_rx,
            update_requests,
        );
    }
}

fn replace_label_entries(target: &mut Vec<String>, label: &str, replacement: Vec<String>) {
    target.retain(|entry| entry != label);
    target.extend(replacement);
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
            commit: source.commit.clone(),
            branch: source.branch.clone(),
            asset: if source.commit.is_none() && source.tag.is_none() && source.branch.is_none() {
                source.asset.clone()
            } else {
                None
            },
            pkg: if source.commit.is_none() && source.tag.is_none() && source.branch.is_none() {
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
            commit: None,
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
            commit: None,
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
            commit: None,
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

fn extend_log_pending_update_requests(
    state: &WizardState,
    sources: &ModDownloadsLoad,
    selected_source_ids: &BTreeMap<String, String>,
    preview: &mut UpdatePreviewWork,
) {
    let include_all_tabs = state.step1.game_install == "EET";
    let exact_log_mode = state.step1.installs_exactly_from_weidu_logs();
    let game_tab = state.step2.active_game_tab.as_str();
    for pending in &state.step2.log_pending_downloads {
        if !include_all_tabs && pending.game_tab != game_tab {
            continue;
        }
        let tp2_key = mod_downloads::normalize_mod_download_tp2(&pending.tp_file);
        if tp2_key.is_empty() || !preview.queued_tp2.insert(tp2_key) {
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
                    &mut preview.update_requests,
                );
                preview.known.push(pending.label.clone());
            } else {
                preview.manual.push(pending.label.clone());
            }
        } else {
            preview.unknown.push(pending.label.clone());
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
