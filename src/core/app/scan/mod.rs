// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde::{Deserialize, Serialize};

use crate::parser::PromptSummaryEvent;
use crate::ui::state::{Step2ModState, Step2ScanReport};

pub mod cache;
pub mod discovery;
pub mod parse;
pub mod readme;
pub mod worker;

pub const ENABLE_TWO_PHASE_PREVIEW: bool = true;
pub const SCAN_CACHE_FILE: &str = "bio_scan_cache.json";
pub const SCAN_CACHE_VERSION: u32 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedComponent {
    pub tp_file: Option<String>,
    pub component_id: String,
    pub display: String,
    pub raw_line: String,
    #[serde(default)]
    pub prompt_summary: Option<String>,
    #[serde(default)]
    pub prompt_events: Vec<PromptSummaryEvent>,
    #[serde(default)]
    pub mod_prompt_summary: Option<String>,
    #[serde(default)]
    pub mod_prompt_events: Vec<PromptSummaryEvent>,
}

pub enum Step2ScanEvent {
    Preview {
        bgee_mods: Vec<Step2ModState>,
        bg2ee_mods: Vec<Step2ModState>,
        total: usize,
    },
    Progress {
        current: usize,
        total: usize,
        name: String,
    },
    Finished {
        bgee_mods: Vec<Step2ModState>,
        bg2ee_mods: Vec<Step2ModState>,
        report: Step2ScanReport,
    },
    Failed(String),
    Canceled,
}
