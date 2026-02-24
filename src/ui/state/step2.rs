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
    pub left_pane_ratio: f32,
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
            left_pane_ratio: 0.74,
        }
    }
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
    pub checked: bool,
    pub components: Vec<Step2ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step2ComponentState {
    pub component_id: String,
    pub label: String,
    pub raw_line: String,
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
