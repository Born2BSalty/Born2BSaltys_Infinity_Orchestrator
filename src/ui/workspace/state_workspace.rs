// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::time::Instant;

use crate::app::state::Step2Selection;
use crate::registry::model::Game;

#[derive(Debug, Clone, Default)]
pub struct WorkspaceStep2State {
    pub details_open: bool,
    pub last_selection: Option<Step2Selection>,
    pub rescan_snapshot: Option<RescanSnapshot>,
    pub was_scanning: bool,
    pub rescan_drop_warning: Option<String>,
    pub resume_pending: bool,
    pub pending_weidu_log_confirm: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescanSelection {
    pub tp2_upper: String,
    pub component_id: String,
    pub selected_order: Option<usize>,
    /// Persisted `@wlb-inputs` marker value, if any, carried through rescan.
    pub wlb_inputs: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RescanSnapshot {
    pub bgee: Vec<RescanSelection>,
    pub bg2ee: Vec<RescanSelection>,
}

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
    pub const fn index(self) -> usize {
        match self {
            Self::Step2 => 0,
            Self::Step3 => 1,
            Self::Step4 => 2,
            Self::Step5 => 3,
        }
    }

    #[must_use]
    pub const fn step_kicker(self) -> &'static str {
        match self {
            Self::Step2 => "Step 2",
            Self::Step3 => "Step 3",
            Self::Step4 => "Step 4",
            Self::Step5 => "Step 5",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Step2 => "Scan and Select",
            Self::Step3 => "Reorder and Resolve",
            Self::Step4 => "Review",
            Self::Step5 => "Install",
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Step2 => "Choose components to install.",
            Self::Step3 => {
                "Review and adjust install order. Drag to reorder; right-click for more actions."
            }
            Self::Step4 => {
                "Verify setup and install order before running. Next saves weidu.log file(s) and advances to install."
            }
            Self::Step5 => "Run the install with live console, prompts, and diagnostics.",
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

    #[must_use]
    pub const fn prev(self) -> Option<Self> {
        match self {
            Self::Step2 => None,
            Self::Step3 => Some(Self::Step2),
            Self::Step4 => Some(Self::Step3),
            Self::Step5 => Some(Self::Step4),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ForkMeta {
    pub parent_name: String,
    pub parent_author: String,
    pub mods: u32,
    pub components: u32,
    pub(crate) forked_from: Vec<crate::app::modlist_share::ForkAncestor>,
}

pub type WorkspaceFlag = bool;

#[derive(Debug, Clone)]
pub struct WorkspaceViewState {
    pub modlist_id: String,
    pub modlist_name: String,
    pub fork_meta: Option<ForkMeta>,
    pub game: Game,
    pub current_step: WorkspaceStep,
    pub completed_steps: HashSet<WorkspaceStep>,
    pub renaming: WorkspaceFlag,
    pub rename_temp: String,
    pub save_draft_flash_until: Option<Instant>,
    pub share_paste_open: WorkspaceFlag,
    pub fork_info_open: WorkspaceFlag,
    pub install_complete: WorkspaceFlag,
    pub loaded_workspace_id: Option<String>,
    pub step2: WorkspaceStep2State,
}

impl Default for WorkspaceViewState {
    fn default() -> Self {
        Self {
            modlist_id: String::new(),
            modlist_name: String::new(),
            fork_meta: None,
            game: Game::default(),
            current_step: WorkspaceStep::Step2,
            completed_steps: HashSet::new(),
            renaming: false,
            rename_temp: String::new(),
            save_draft_flash_until: None,
            share_paste_open: false,
            fork_info_open: false,
            install_complete: false,
            loaded_workspace_id: None,
            step2: WorkspaceStep2State::default(),
        }
    }
}

impl WorkspaceViewState {
    #[must_use]
    pub fn is_completed(&self, step: WorkspaceStep) -> bool {
        self.completed_steps.contains(&step)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_lands_on_step2() {
        let s = WorkspaceViewState::default();
        assert_eq!(s.current_step, WorkspaceStep::Step2);
        assert!(s.completed_steps.is_empty());
        assert_eq!(s.loaded_workspace_id, None);
    }

    #[test]
    fn step_order_and_nav() {
        assert_eq!(WorkspaceStep::Step2.index(), 0);
        assert_eq!(WorkspaceStep::Step5.index(), 3);
        assert_eq!(WorkspaceStep::Step2.prev(), None);
        assert_eq!(WorkspaceStep::Step2.next(), Some(WorkspaceStep::Step3));
        assert_eq!(WorkspaceStep::Step5.next(), None);
        assert_eq!(WorkspaceStep::Step5.prev(), Some(WorkspaceStep::Step4));
        assert_eq!(WorkspaceStep::ALL.len(), 4);
    }

    #[test]
    fn labels_match_spec_2_2() {
        assert_eq!(WorkspaceStep::Step2.label(), "Scan and Select");
        assert_eq!(WorkspaceStep::Step3.label(), "Reorder and Resolve");
        assert_eq!(WorkspaceStep::Step4.label(), "Review");
        assert_eq!(WorkspaceStep::Step5.label(), "Install");
        assert_eq!(WorkspaceStep::Step3.step_kicker(), "Step 3");
    }
}
