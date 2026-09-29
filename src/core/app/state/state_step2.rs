// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use crate::app::github_forks_list::ForksListState;
use crate::app::github_release_list::ReleaseListState;
use crate::app::source_form::SourceForm;
use crate::app::step2_action::ModSourceEditDestination;
use crate::app::versions_view::{KnownExtras, VersionsView};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadOrigin {
    #[default]
    Workspace,
    InstallPipeline,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeiduLogImport {
    pub first: Option<PathBuf>,
    pub second: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptPopupMode {
    Text,
    ToolbarIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManualDownloadReason {
    NotAutoResolvable,
    NoSourceEntry,
    SourceCheckFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualDownloadRequest {
    pub game_tab: String,
    pub tp_file: String,
    pub label: String,
    pub source_id: String,
    pub page_url: String,
    pub reason: ManualDownloadReason,
    pub aliases: Vec<String>,
    pub display_name: String,
}

pub fn push_manual_download_request(
    list: &mut Vec<ManualDownloadRequest>,
    request: ManualDownloadRequest,
) {
    if list.iter().any(|existing| {
        existing.game_tab == request.game_tab && existing.tp_file == request.tp_file
    }) {
        return;
    }
    list.push(request);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExtractJobProgress {
    pub total: usize,
    pub started: usize,
    pub done: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VersionsChip {
    #[default]
    All,
    Fetch,
    Attention,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionsSheet {
    EditSource,
    Forks,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionsMenu {
    Sources {
        tp2: String,
    },
    Kebab {
        tp2: String,
        bookmark_label: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VersionsDrawerUi {
    pub(crate) search: String,
    pub(crate) chip: VersionsChip,
    pub(crate) sheet: Option<VersionsSheet>,
    pub(crate) sheet_tp2: Option<String>,
    pub(crate) sheet_just_opened: bool,
    pub(crate) menu: Option<VersionsMenu>,
    pub(crate) auto_check_pending: bool,
    pub(crate) source_form: Option<SourceForm>,
    pub(crate) release_list: ReleaseListState,
    pub(crate) known: Option<KnownExtras>,
    pub(crate) pending_toast: Option<String>,
    pub(crate) sheet_error: Option<String>,
    pub(crate) focused_tp2: Option<String>,
    pub(crate) focus_scroll_pending: bool,
    pub(crate) fetch_queue: Vec<String>,
    pub(crate) fetching_tp2: Option<String>,
    pub(crate) scan_view_cache: Option<Arc<VersionsView>>,
}

impl VersionsDrawerUi {
    pub(crate) fn open_sheet(&mut self, sheet: VersionsSheet, tp2: String) {
        self.sheet = Some(sheet);
        self.sheet_tp2 = Some(tp2);
        self.sheet_just_opened = true;
        self.sheet_error = None;
    }

    pub(crate) fn focus_card(&mut self, tp2_key: String) {
        self.focused_tp2 = Some(tp2_key);
        self.focus_scroll_pending = true;
    }

    pub(crate) fn toggle_queued(&mut self, tp2: &str) {
        if let Some(index) = self.fetch_queue.iter().position(|queued| queued == tp2) {
            self.fetch_queue.remove(index);
        } else {
            self.fetch_queue.push(tp2.to_string());
        }
    }

    pub(crate) fn pop_queued(&mut self) -> Option<String> {
        if self.fetch_queue.is_empty() {
            None
        } else {
            Some(self.fetch_queue.remove(0))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step2State<Flag = bool> {
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
    pub collapse_default_open: Flag,
    pub jump_to_selected_requested: Flag,
    pub is_scanning: Flag,
    pub compat_popup_open: Flag,
    pub compat_popup_issue_override: Option<crate::app::compat_issue::CompatIssue>,
    pub compat_popup_filter: String,
    pub prompt_popup_open: Flag,
    pub prompt_popup_mode: PromptPopupMode,
    pub prompt_popup_title: String,
    pub prompt_popup_text: String,
    pub update_selected_popup_open: Flag,
    pub update_selected_has_run: Flag,
    pub update_selected_last_selection_signature: Option<String>,
    pub update_selected_last_was_full_selection: Flag,
    pub update_selected_check_running: Flag,
    pub update_selected_check_done_count: usize,
    pub update_selected_check_total_count: usize,
    pub update_selected_download_running: Flag,
    pub update_selected_extract_running: Flag,
    pub update_selected_extract_progress: Option<(usize, usize)>,
    pub update_selected_extract_jobs: BTreeMap<String, ExtractJobProgress>,
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
    pub update_selected_version_override_warnings: Vec<String>,
    pub update_selected_check_requests: Vec<Step2UpdateRetryRequest>,
    pub update_selected_exact_version_retry_requests: Vec<Step2UpdateRetryRequest>,
    pub update_selected_confirm_latest_fallback_open: Flag,
    pub update_selected_merge_latest_fallback: Flag,
    pub mod_download_source_editor_open: Flag,
    pub mod_download_source_editor_tp2: String,
    pub mod_download_source_editor_label: String,
    pub mod_download_source_editor_display_name: String,
    pub mod_download_source_editor_source_id: String,
    pub mod_download_source_editor_allow_source_id_change: Flag,
    pub mod_download_source_editor_text: String,
    pub mod_download_source_editor_error: Option<String>,
    pub mod_download_source_editor_destination: ModSourceEditDestination,
    pub mod_download_forks_popup_open: Flag,
    pub mod_download_forks_popup_title: String,
    pub mod_download_forks_popup_tp2: String,
    pub mod_download_forks_popup_label: String,
    pub mod_download_forks_popup_error: Option<String>,
    pub mod_download_forks: Vec<Step2DiscoveredFork>,
    pub(crate) forks_list: ForksListState,
    pub(crate) whole_folder_check_active: bool,
    pub selected_source_ids: BTreeMap<String, String>,
    pub update_selected_target_game_tab: Option<String>,
    pub update_selected_target_tp_file: Option<String>,
    pub update_selected_refresh_target_game_tab: Option<String>,
    pub update_selected_refresh_target_tp_file: Option<String>,
    pub log_pending_downloads: Vec<Step2LogPendingDownload>,
    pub exact_log_mod_list_checked: Flag,
    pub pending_saved_log_apply: Flag,
    pub pending_saved_log_update_preview: Flag,
    pub pending_saved_log_download: Flag,
    pub weidu_log_import: Option<WeiduLogImport>,
    pub pending_weidu_log_reapply: Flag,
    pub weidu_log_import_awaiting_check: Flag,
    pub review_edit_bgee_log_applied: Flag,
    pub review_edit_bg2ee_log_applied: Flag,
    pub left_pane_ratio: f32,
    pub last_scan_report: Option<Step2ScanReport>,
    pub update_selected_manual_downloads: Vec<ManualDownloadRequest>,
    pub skipped_manual_downloads: Vec<String>,
    pub update_selected_download_scope: Option<String>,
    pub update_selected_download_bytes: BTreeMap<usize, (u64, Option<u64>)>,
    pub update_selected_download_done: BTreeSet<usize>,
    pub update_selected_download_total: usize,
    pub update_selected_download_origin: DownloadOrigin,
    pub update_selected_download_finished: Vec<String>,
    pub(crate) update_selected_last_checked_at: Option<String>,
    pub(crate) versions_ui: VersionsDrawerUi,
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
            update_selected_extract_progress: None,
            update_selected_extract_jobs: BTreeMap::new(),
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
            update_selected_version_override_warnings: Vec::new(),
            update_selected_check_requests: Vec::new(),
            update_selected_exact_version_retry_requests: Vec::new(),
            update_selected_confirm_latest_fallback_open: false,
            update_selected_merge_latest_fallback: false,
            mod_download_source_editor_open: false,
            mod_download_source_editor_tp2: String::new(),
            mod_download_source_editor_label: String::new(),
            mod_download_source_editor_display_name: String::new(),
            mod_download_source_editor_source_id: String::new(),
            mod_download_source_editor_allow_source_id_change: false,
            mod_download_source_editor_text: String::new(),
            mod_download_source_editor_error: None,
            mod_download_source_editor_destination: ModSourceEditDestination::GlobalDefault,
            mod_download_forks_popup_open: false,
            mod_download_forks_popup_title: String::new(),
            mod_download_forks_popup_tp2: String::new(),
            mod_download_forks_popup_label: String::new(),
            mod_download_forks_popup_error: None,
            mod_download_forks: Vec::new(),
            forks_list: ForksListState::default(),
            whole_folder_check_active: false,
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
            weidu_log_import: None,
            pending_weidu_log_reapply: false,
            weidu_log_import_awaiting_check: false,
            review_edit_bgee_log_applied: false,
            review_edit_bg2ee_log_applied: false,
            left_pane_ratio: 0.74,
            last_scan_report: None,
            update_selected_manual_downloads: Vec::new(),
            skipped_manual_downloads: Vec::new(),
            update_selected_download_scope: None,
            update_selected_download_bytes: BTreeMap::new(),
            update_selected_download_done: BTreeSet::new(),
            update_selected_download_total: 0,
            update_selected_download_origin: DownloadOrigin::Workspace,
            update_selected_download_finished: Vec::new(),
            update_selected_last_checked_at: None,
            versions_ui: VersionsDrawerUi::default(),
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
pub struct Step2ComponentState<Flag = bool> {
    pub component_id: String,
    pub label: String,
    pub weidu_group: Option<String>,
    pub collapsible_group: Option<String>,
    pub collapsible_group_is_umbrella: Flag,
    pub collapsible_group_combinable: bool,
    pub raw_line: String,
    pub prompt_summary: Option<String>,
    pub prompt_events: Vec<crate::parser::PromptSummaryEvent>,
    pub is_meta_mode_component: Flag,
    pub disabled: Flag,
    pub compat_kind: Option<String>,
    pub compat_source: Option<String>,
    pub compat_related_mod: Option<String>,
    pub compat_related_component: Option<String>,
    pub compat_graph: Option<String>,
    pub compat_evidence: Option<String>,
    pub disabled_reason: Option<String>,
    pub checked: Flag,
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
pub fn update_selection_stale(step2: &Step2State) -> bool {
    step2.update_selected_has_run
        && (!step2.update_selected_last_was_full_selection
            || step2.update_selected_last_selection_signature.as_deref()
                != Some(update_selection_signature(step2).as_str()))
}

#[must_use]
pub const fn update_pipeline_busy(step2: &Step2State) -> bool {
    step2.is_scanning
        || step2.update_selected_check_running
        || step2.update_selected_download_running
        || step2.update_selected_extract_running
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
        out.push(format!("{tag}|{}", mod_state.tp_file.to_ascii_uppercase()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetch_queue_toggles_and_pops_in_order() {
        let mut ui = VersionsDrawerUi::default();
        assert_eq!(ui.pop_queued(), None);
        ui.toggle_queued("alpha");
        ui.toggle_queued("beta");
        ui.toggle_queued("gamma");
        ui.toggle_queued("beta");
        assert_eq!(
            ui.fetch_queue,
            vec!["alpha".to_string(), "gamma".to_string()]
        );
        ui.toggle_queued("beta");
        assert_eq!(ui.pop_queued().as_deref(), Some("alpha"));
        assert_eq!(ui.pop_queued().as_deref(), Some("gamma"));
        assert_eq!(ui.pop_queued().as_deref(), Some("beta"));
        assert_eq!(ui.pop_queued(), None);
    }

    fn scanned_mod(tp_file: &str) -> Step2ModState {
        Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: String::new(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: vec![Step2ComponentState {
                component_id: "0".to_string(),
                label: "0".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: String::new(),
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
                checked: false,
                selected_order: None,
            }],
        }
    }

    #[test]
    fn default_download_state_is_workspace_origin_with_empty_byte_map_and_done_set() {
        let step2 = Step2State::default();
        assert_eq!(
            step2.update_selected_download_origin,
            DownloadOrigin::Workspace
        );
        assert_eq!(DownloadOrigin::default(), DownloadOrigin::Workspace);
        assert!(step2.update_selected_download_bytes.is_empty());
        assert!(step2.update_selected_download_done.is_empty());
        assert_eq!(step2.update_selected_download_total, 0);
        assert!(step2.update_selected_download_finished.is_empty());
        assert_eq!(step2.update_selected_download_scope, None);
        assert_eq!(step2.update_selected_extract_progress, None);
        assert!(step2.update_selected_extract_jobs.is_empty());
    }

    #[test]
    fn signature_follows_folder_and_sources_not_ticks() {
        let mut step2 = Step2State::default();
        step2.bgee_mods.push(scanned_mod("alpha/setup-alpha.tp2"));
        let base = update_selection_signature(&step2);
        assert!(base.contains("BGEE|ALPHA/SETUP-ALPHA.TP2"));

        step2.bgee_mods[0].checked = true;
        step2.bgee_mods[0].components[0].checked = true;
        step2.bgee_mods[0].components[0].selected_order = Some(3);
        assert_eq!(update_selection_signature(&step2), base);

        step2.bg2ee_mods.push(scanned_mod("beta/setup-beta.tp2"));
        let with_beta = update_selection_signature(&step2);
        assert_ne!(with_beta, base);
        assert!(with_beta.contains("BG2EE|BETA/SETUP-BETA.TP2"));

        step2
            .selected_source_ids
            .insert("alpha/setup-alpha.tp2".to_string(), "primary".to_string());
        let with_source = update_selection_signature(&step2);
        assert_ne!(with_source, with_beta);
        step2
            .selected_source_ids
            .insert("alpha/setup-alpha.tp2".to_string(), "mirror".to_string());
        assert_ne!(update_selection_signature(&step2), with_source);
    }
}
