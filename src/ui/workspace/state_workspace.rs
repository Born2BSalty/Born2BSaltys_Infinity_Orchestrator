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
    pub const ALL: [Self; 4] = [Self::Step2, Self::Step3, Self::Step4, Self::Step5];

    #[must_use]
    pub const fn number(self) -> usize {
        match self {
            Self::Step2 => 2,
            Self::Step3 => 3,
            Self::Step4 => 4,
            Self::Step5 => 5,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Step2 => "Scan and select",
            Self::Step3 => "Reorder and resolve",
            Self::Step4 => "Review",
            Self::Step5 => "Install",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Step2 => "scan mods, choose components, and resolve missing sources",
            Self::Step3 => "review install order and resolve conflicts before review",
            Self::Step4 => "verify setup and generated WeiDU order before install",
            Self::Step5 => "install runtime arrives in Phase 7",
        }
    }

    #[must_use]
    pub const fn previous(self) -> Option<Self> {
        match self {
            Self::Step2 => None,
            Self::Step3 => Some(Self::Step2),
            Self::Step4 => Some(Self::Step3),
            Self::Step5 => Some(Self::Step4),
        }
    }

    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Step2 => Some(Self::Step3),
            Self::Step3 => Some(Self::Step4),
            Self::Step4 => Some(Self::Step5),
            Self::Step5 => None,
        }
    }
}

#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "workspace view state preserves independent UI workflow flags"
)]
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
