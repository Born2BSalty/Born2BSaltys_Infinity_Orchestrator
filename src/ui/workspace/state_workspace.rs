// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceStep {
    Step2,
    Step3,
    Step4,
    Step5,
}

impl WorkspaceStep {
    pub const ALL: [WorkspaceStep; 4] = [
        WorkspaceStep::Step2,
        WorkspaceStep::Step3,
        WorkspaceStep::Step4,
        WorkspaceStep::Step5,
    ];

    pub fn number(self) -> usize {
        match self {
            WorkspaceStep::Step2 => 2,
            WorkspaceStep::Step3 => 3,
            WorkspaceStep::Step4 => 4,
            WorkspaceStep::Step5 => 5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WorkspaceStep::Step2 => "Scan and select",
            WorkspaceStep::Step3 => "Reorder and resolve",
            WorkspaceStep::Step4 => "Review",
            WorkspaceStep::Step5 => "Install",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            WorkspaceStep::Step2 => "scan mods, choose components, and resolve missing sources",
            WorkspaceStep::Step3 => "review install order and resolve conflicts before review",
            WorkspaceStep::Step4 => "verify setup and generated WeiDU order before install",
            WorkspaceStep::Step5 => "install runtime arrives in Phase 7",
        }
    }

    pub fn previous(self) -> Option<Self> {
        match self {
            WorkspaceStep::Step2 => None,
            WorkspaceStep::Step3 => Some(WorkspaceStep::Step2),
            WorkspaceStep::Step4 => Some(WorkspaceStep::Step3),
            WorkspaceStep::Step5 => Some(WorkspaceStep::Step4),
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            WorkspaceStep::Step2 => Some(WorkspaceStep::Step3),
            WorkspaceStep::Step3 => Some(WorkspaceStep::Step4),
            WorkspaceStep::Step4 => Some(WorkspaceStep::Step5),
            WorkspaceStep::Step5 => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceViewState {
    pub modlist_id: String,
    pub modlist_name: String,
    pub fork_meta: Option<ForkMeta>,
    pub current_step: WorkspaceStep,
    pub completed_steps: HashSet<WorkspaceStep>,
    pub renaming: bool,
    pub rename_temp: String,
    pub save_draft_flash_until: Option<Instant>,
    pub save_draft_requested: bool,
    pub share_paste_open: bool,
    pub install_complete: bool,
    pub loaded_workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkMeta {
    pub source_name: String,
}

impl WorkspaceViewState {
    pub fn new(modlist_id: impl Into<String>, modlist_name: impl Into<String>) -> Self {
        let modlist_name = modlist_name.into();
        Self {
            modlist_id: modlist_id.into(),
            rename_temp: modlist_name.clone(),
            modlist_name,
            fork_meta: None,
            current_step: WorkspaceStep::Step2,
            completed_steps: HashSet::new(),
            renaming: false,
            save_draft_flash_until: None,
            save_draft_requested: false,
            share_paste_open: false,
            install_complete: false,
            loaded_workspace_id: None,
        }
    }

    pub fn mark_current_complete(&mut self) {
        self.completed_steps.insert(self.current_step);
    }
}

impl Default for WorkspaceViewState {
    fn default() -> Self {
        Self::new("workspace-preview", "Untitled modlist")
    }
}
