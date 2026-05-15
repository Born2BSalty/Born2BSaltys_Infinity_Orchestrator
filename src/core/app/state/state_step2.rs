// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptPopupMode {
    Text,
    ToolbarIndex,
}

#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "wizard Step 2 state is a stable cross-module UI/runtime state contract"
)]
pub struct Step2State {
    pub search_query: String,
    pub scan_status: String,
    pub scan_progress_percent: u8,
    pub active_game_tab: String,
    pub selected_count: usize,
    pub total_count: usize,
    pub bgee_mods: Vec<Step2ModState>,
    pub bg2ee_mods: Vec<Step2ModState>,
    pub selected: Option<Step2Selection>,
    pub next_selection_order: usize,
    pub collapse_epoch: u64,
    pub collapse_default_open: bool,
    pub jump_to_selected_requested: bool,
    pub is_scanning: bool,
    pub compat_popup_open: bool,
    pub compat_popup_issue_override: Option<crate::app::compat_issue::CompatIssue>,
    pub compat_popup_filter: String,
    pub prompt_popup_open: bool,
    pub prompt_popup_mode: PromptPopupMode,
    pub prompt_popup_title: String,
    pub prompt_popup_text: String,
    pub update_selected_popup_open: bool,
    pub update_selected_has_run: bool,
    pub update_selected_last_selection_signature: Option<String>,
    pub update_selected_last_was_full_selection: bool,
    pub update_selected_check_running: bool,
    pub update_selected_check_done_count: usize,
    pub update_selected_check_total_count: usize,
    pub update_selected_download_running: bool,
    pub update_selected_extract_running: bool,
    pub update_selected_update_assets: Vec<Step2UpdateAsset>,
    pub update_selected_update_sources: Vec<String>,
    pub update_selected_locked_update_assets: Vec<Step2UpdateAsset>,
    pub update_selected_locked_update_sources: Vec<String>,
    pub update_selected_missing_sources: Vec<String>,
    pub update_selected_downloaded_sources: Vec<String>,
    pub update_selected_download_failed_sources: Vec<String>,
    pub update_selected_extracted_sources: Vec<String>,
    pub update_selected_extract_failed_sources: Vec<String>,
    pub update_selected_known_sources: Vec<String>,
    pub update_selected_manual_sources: Vec<String>,
    pub update_selected_unknown_sources: Vec<String>,
    pub update_selected_exact_version_failed_sources: Vec<String>,
    pub update_selected_failed_sources: Vec<String>,
    pub update_selected_check_requests: Vec<Step2UpdateRetryRequest>,
    pub update_selected_exact_version_retry_requests: Vec<Step2UpdateRetryRequest>,
    pub update_selected_confirm_latest_fallback_open: bool,
    pub update_selected_merge_latest_fallback: bool,
    pub mod_download_source_editor_open: bool,
    pub mod_download_source_editor_tp2: String,
    pub mod_download_source_editor_label: String,
    pub mod_download_source_editor_source_id: String,
    pub mod_download_source_editor_allow_source_id_change: bool,
    pub mod_download_source_editor_text: String,
    pub mod_download_source_editor_error: Option<String>,
    pub mod_download_forks_popup_open: bool,
    pub mod_download_forks_popup_title: String,
    pub mod_download_forks_popup_tp2: String,
    pub mod_download_forks_popup_label: String,
    pub mod_download_forks_popup_error: Option<String>,
    pub mod_download_forks: Vec<Step2DiscoveredFork>,
    pub selected_source_ids: BTreeMap<String, String>,
    pub update_selected_target_game_tab: Option<String>,
    pub update_selected_target_tp_file: Option<String>,
    pub update_selected_refresh_target_game_tab: Option<String>,
    pub update_selected_refresh_target_tp_file: Option<String>,
    pub log_pending_downloads: Vec<Step2LogPendingDownload>,
    pub exact_log_mod_list_checked: bool,
    pub pending_saved_log_apply: bool,
    pub pending_saved_log_update_preview: bool,
    pub pending_saved_log_download: bool,
    pub review_edit_bgee_log_applied: bool,
    pub review_edit_bg2ee_log_applied: bool,
    pub left_pane_ratio: f32,
    pub last_scan_report: Option<Step2ScanReport>,
}

impl Default for Step2State {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            scan_status: "Idle".to_string(),
            scan_progress_percent: 0,
            active_game_tab: "BGEE".to_string(),
            selected_count: 0,
            total_count: 0,
            bgee_mods: Vec::new(),
            bg2ee_mods: Vec::new(),
            selected: None,
            next_selection_order: 1,
            collapse_epoch: 0,
            collapse_default_open: false,
            jump_to_selected_requested: false,
            is_scanning: false,
            compat_popup_open: false,
            compat_popup_issue_override: None,
            compat_popup_filter: "All".to_string(),
            prompt_popup_open: false,
            prompt_popup_mode: PromptPopupMode::Text,
            prompt_popup_title: String::new(),
            prompt_popup_text: String::new(),
            update_selected_popup_open: false,
            update_selected_has_run: false,
            update_selected_last_selection_signature: None,
            update_selected_last_was_full_selection: false,
            update_selected_check_running: false,
            update_selected_check_done_count: 0,
            update_selected_check_total_count: 0,
            update_selected_download_running: false,
            update_selected_extract_running: false,
            update_selected_update_assets: Vec::new(),
            update_selected_update_sources: Vec::new(),
            update_selected_locked_update_assets: Vec::new(),
            update_selected_locked_update_sources: Vec::new(),
            update_selected_missing_sources: Vec::new(),
            update_selected_downloaded_sources: Vec::new(),
            update_selected_download_failed_sources: Vec::new(),
            update_selected_extracted_sources: Vec::new(),
            update_selected_extract_failed_sources: Vec::new(),
            update_selected_known_sources: Vec::new(),
            update_selected_manual_sources: Vec::new(),
            update_selected_unknown_sources: Vec::new(),
            update_selected_exact_version_failed_sources: Vec::new(),
            update_selected_failed_sources: Vec::new(),
            update_selected_check_requests: Vec::new(),
            update_selected_exact_version_retry_requests: Vec::new(),
            update_selected_confirm_latest_fallback_open: false,
            update_selected_merge_latest_fallback: false,
            mod_download_source_editor_open: false,
            mod_download_source_editor_tp2: String::new(),
            mod_download_source_editor_label: String::new(),
            mod_download_source_editor_source_id: String::new(),
            mod_download_source_editor_allow_source_id_change: false,
            mod_download_source_editor_text: String::new(),
            mod_download_source_editor_error: None,
            mod_download_forks_popup_open: false,
            mod_download_forks_popup_title: String::new(),
            mod_download_forks_popup_tp2: String::new(),
            mod_download_forks_popup_label: String::new(),
            mod_download_forks_popup_error: None,
            mod_download_forks: Vec::new(),
            selected_source_ids: BTreeMap::new(),
            update_selected_target_game_tab: None,
            update_selected_target_tp_file: None,
            update_selected_refresh_target_game_tab: None,
            update_selected_refresh_target_tp_file: None,
            log_pending_downloads: Vec::new(),
            exact_log_mod_list_checked: false,
            pending_saved_log_apply: false,
            pending_saved_log_update_preview: false,
            pending_saved_log_download: false,
            review_edit_bgee_log_applied: false,
            review_edit_bg2ee_log_applied: false,
            left_pane_ratio: 0.74,
            last_scan_report: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2DiscoveredFork {
    pub full_name: String,
    pub html_url: String,
    pub owner_login: String,
    pub default_branch: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2UpdateAsset {
    pub game_tab: String,
    pub tp_file: String,
    pub label: String,
    pub source_id: String,
    pub tag: String,
    pub asset_name: String,
    pub asset_url: String,
    pub installed_source_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2LogPendingDownload {
    pub game_tab: String,
    pub tp_file: String,
    pub label: String,
    pub requested_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2UpdateRetryRequest {
    pub game_tab: String,
    pub tp_file: String,
    pub label: String,
    pub source_id: String,
    pub repo: String,
    pub source_url: String,
    pub channel: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub asset: Option<String>,
    pub pkg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step2ScanReport {
    pub game_dir: String,
    pub mods_root: String,
    pub scan_depth: usize,
    pub preferred_locale: String,
    pub preferred_locale_source: String,
    pub preferred_locale_baldur_lua: Option<String>,
    pub worker_count: usize,
    pub total_groups: usize,
    pub total_tp2: usize,
    pub tp2_cache_hits: usize,
    pub tp2_cache_misses: usize,
    pub scan_cache_path: String,
    pub scan_cache_source: String,
    pub scan_cache_load_status: String,
    pub scan_cache_load_error: Option<String>,
    pub scan_cache_file_exists: bool,
    pub scan_cache_file_mtime_secs: Option<u64>,
    pub scan_cache_file_version: Option<u32>,
    pub scan_cache_writer_app_version: Option<String>,
    pub scan_cache_writer_exe_fingerprint: Option<String>,
    pub scan_cache_entry_count: usize,
    pub scan_cache_version_matches_current_schema: bool,
    pub scan_cache_fallback_path: Option<String>,
    pub scan_cache_fallback_source: Option<String>,
    pub scan_cache_fallback_load_status: Option<String>,
    pub scan_cache_fallback_load_error: Option<String>,
    pub scan_cache_save_error: Option<String>,
    pub scan_cache_writer_matches_current_app_version: Option<bool>,
    pub scan_cache_writer_matches_current_exe: Option<bool>,
    pub tp2_reports: Vec<Step2Tp2ProbeReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step2Tp2ProbeReport {
    pub group_label: String,
    pub tp2_path: String,
    pub work_dir: String,
    pub used_cache: bool,
    pub selected_from_cache: bool,
    pub language_ids_tried: Vec<String>,
    pub selected_language_id: Option<String>,
    pub parsed_count: usize,
    pub undefined_count: usize,
    pub parser_source_file: Option<String>,
    pub parser_event_count: usize,
    pub parser_warning_count: usize,
    pub parser_error_count: usize,
    pub parser_diagnostic_preview: Option<String>,
    pub parser_raw_json: Option<String>,
    pub parser_tra_language_requested: Option<String>,
    pub parser_tra_language_used: Option<String>,
    pub parser_flow_node_count: usize,
    pub parser_flow_event_ref_count: usize,
    pub parser_event_with_parent_count: usize,
    pub parser_event_with_path_count: usize,
    pub parser_option_component_binding_count: usize,
    pub parser_flow_preview: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step2Selection {
    Mod {
        game_tab: String,
        tp_file: String,
    },
    Component {
        game_tab: String,
        tp_file: String,
        component_id: String,
        component_key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2ModState {
    pub name: String,
    pub tp_file: String,
    pub tp2_path: String,
    pub readme_path: Option<String>,
    pub ini_path: Option<String>,
    pub web_url: Option<String>,
    pub package_marker: Option<char>,
    pub latest_checked_version: Option<String>,
    pub update_locked: bool,
    pub mod_prompt_summary: Option<String>,
    pub mod_prompt_events: Vec<crate::parser::PromptSummaryEvent>,
    pub checked: bool,
    pub hidden_components: Vec<Step2HiddenComponentAudit>,
    pub components: Vec<Step2ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2HiddenComponentAudit {
    pub component_id: String,
    pub label: String,
    pub raw_line: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "component state mirrors independent scan, selection, and compatibility flags"
)]
pub struct Step2ComponentState {
    pub component_id: String,
    pub label: String,
    pub weidu_group: Option<String>,
    pub subcomponent_key: Option<String>,
    pub tp2_empty_placeholder_block: bool,
    pub collapsible_group: Option<String>,
    pub collapsible_group_is_umbrella: bool,
    pub raw_line: String,
    pub prompt_summary: Option<String>,
    pub prompt_events: Vec<crate::parser::PromptSummaryEvent>,
    pub is_meta_mode_component: bool,
    pub disabled: bool,
    pub compat_kind: Option<String>,
    pub compat_source: Option<String>,
    pub compat_related_mod: Option<String>,
    pub compat_related_component: Option<String>,
    pub compat_graph: Option<String>,
    pub compat_evidence: Option<String>,
    pub disabled_reason: Option<String>,
    pub checked: bool,
    pub selected_order: Option<usize>,
}

#[must_use]
pub fn update_selection_signature(step2: &Step2State) -> String {
    let mut entries = Vec::<String>::new();
    collect_update_selection_signature("BGEE", &step2.bgee_mods, &mut entries);
    collect_update_selection_signature("BG2EE", &step2.bg2ee_mods, &mut entries);
    for (tp2, source_id) in &step2.selected_source_ids {
        entries.push(format!(
            "SOURCE|{}|{}",
            tp2.to_ascii_uppercase(),
            source_id.to_ascii_uppercase()
        ));
    }
    entries.sort_unstable();
    entries.join(";")
}

#[must_use]
pub fn exact_log_ready_to_install(state: &crate::app::state::WizardState) -> bool {
    state.step1.installs_exactly_from_weidu_logs()
        && state.step2.exact_log_mod_list_checked
        && !state.step2.is_scanning
        && !state.step2.update_selected_check_running
        && !state.step2.update_selected_download_running
        && !state.step2.update_selected_extract_running
        && state.step2.update_selected_missing_sources.is_empty()
        && state.step2.update_selected_manual_sources.is_empty()
        && state.step2.update_selected_unknown_sources.is_empty()
        && state.step2.update_selected_failed_sources.is_empty()
        && state
            .step2
            .update_selected_exact_version_retry_requests
            .is_empty()
}

fn collect_update_selection_signature(tag: &str, mods: &[Step2ModState], out: &mut Vec<String>) {
    for mod_state in mods {
        let tp_file = mod_state.tp_file.to_ascii_uppercase();
        for component in &mod_state.components {
            if component.checked {
                out.push(format!(
                    "{tag}|{tp_file}|{}|{}",
                    component.component_id,
                    component.selected_order.unwrap_or(usize::MAX)
                ));
            }
        }
    }
}
