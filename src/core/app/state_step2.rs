// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq)]
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
    pub compat_popup_filter: String,
    pub prompt_popup_open: bool,
    pub prompt_popup_title: String,
    pub prompt_popup_text: String,
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
            compat_popup_filter: "all".to_string(),
            prompt_popup_open: false,
            prompt_popup_title: String::new(),
            prompt_popup_text: String::new(),
            left_pane_ratio: 0.74,
            last_scan_report: None,
        }
    }
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
    pub scan_cache_file_exists: bool,
    pub scan_cache_file_mtime_secs: Option<u64>,
    pub scan_cache_file_version: Option<u32>,
    pub scan_cache_writer_app_version: Option<String>,
    pub scan_cache_writer_exe_fingerprint: Option<String>,
    pub scan_cache_entry_count: usize,
    pub scan_cache_version_matches_current_schema: bool,
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
    Mod { game_tab: String, tp_file: String },
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
    pub web_url: Option<String>,
    pub mod_prompt_summary: Option<String>,
    pub mod_prompt_events: Vec<crate::parser::PromptSummaryEvent>,
    pub checked: bool,
    pub components: Vec<Step2ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2ComponentState {
    pub component_id: String,
    pub label: String,
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
