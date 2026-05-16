// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `WorkspaceViewState` — per-modlist workspace UI state held by the
// orchestrator while a modlist workspace is open.
//
// Per Phase 6 P6.T1 + the phase-06 file inventory. The struct shape is fixed
// in Run 1 (the workspace spine) so later runs only fill behavior:
//   - `current_step` / `completed_steps`  → driven by the nav bar (P6.T4).
//   - `loaded_workspace_id`               → tracks which modlist's data is
//     currently sitting in the orchestrator's shared `WizardState` so the
//     loader (P6.T1) knows when a swap is needed.
//   - `renaming` / `rename_temp`          → inline rename (P6.T5 — Run 2;
//     declared now, inert this run, mirroring the Phase-5 staged-field
//     pattern).
//   - `save_draft_flash_until`            → the `✓ saved!` flash (P6.T6 —
//     Run 2; declared now, inert this run).
//   - `share_paste_open`                  → the Share import code dialog
//     (Phase 7; declared now, inert this run).
//   - `install_complete`                  → flipped post-install (Phase 7).
//   - `fork_meta`                         → fork badge + sub-line + ForkInfo
//     popup feed (P6.T5/T8 — Run 2/3; the holder type is fixed now so the
//     model is stable, populated later).
//
// SPEC: §2.2 (workspace shell), §13.1 (per-modlist workspace state).

// rationale: a small UI-state struct with independent flags — the multi-bool
// shape is intentional (each flag drives a distinct workspace affordance),
// and `#[must_use]` on the trivial constructor / queries is churn (Cat 3).
#![allow(clippy::struct_excessive_bools, clippy::must_use_candidate)]

use std::collections::HashSet;
use std::time::Instant;

use crate::registry::model::Game;

/// The four workspace steps (SPEC §2.2). Step 1 no longer exists inside the
/// workspace — setup migrated to Settings + Create.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkspaceStep {
    /// Step 2 — Scan and Select.
    Step2,
    /// Step 3 — Reorder and Resolve.
    Step3,
    /// Step 4 — Review.
    Step4,
    /// Step 5 — Install.
    Step5,
}

impl WorkspaceStep {
    /// All four steps in workspace order.
    pub const ALL: [WorkspaceStep; 4] = [
        WorkspaceStep::Step2,
        WorkspaceStep::Step3,
        WorkspaceStep::Step4,
        WorkspaceStep::Step5,
    ];

    /// 0-based index into [`WorkspaceStep::ALL`].
    pub fn index(self) -> usize {
        match self {
            WorkspaceStep::Step2 => 0,
            WorkspaceStep::Step3 => 1,
            WorkspaceStep::Step4 => 2,
            WorkspaceStep::Step5 => 3,
        }
    }

    /// The `Step N` kicker (SPEC §2.2 / wireframe `WORKSPACE_STEPS`).
    pub fn step_kicker(self) -> &'static str {
        match self {
            WorkspaceStep::Step2 => "Step 2",
            WorkspaceStep::Step3 => "Step 3",
            WorkspaceStep::Step4 => "Step 4",
            WorkspaceStep::Step5 => "Step 5",
        }
    }

    /// The step label (SPEC §2.2 / wireframe `WORKSPACE_STEPS`).
    pub fn label(self) -> &'static str {
        match self {
            WorkspaceStep::Step2 => "Scan and Select",
            WorkspaceStep::Step3 => "Reorder and Resolve",
            WorkspaceStep::Step4 => "Review",
            WorkspaceStep::Step5 => "Install",
        }
    }

    /// The one-line hint shown under the progress bar (wireframe
    /// `WORKSPACE_STEPS[*].hint`).
    pub fn hint(self) -> &'static str {
        match self {
            WorkspaceStep::Step2 => "Choose components to install.",
            WorkspaceStep::Step3 => {
                "Review and adjust install order. Drag to reorder; right-click for more actions."
            }
            WorkspaceStep::Step4 => {
                "Verify setup and install order before running. Next saves weidu.log file(s) and advances to install."
            }
            WorkspaceStep::Step5 => "Run the install with live console, prompts, and diagnostics.",
        }
    }

    /// The next step, or `None` if this is the last.
    pub fn next(self) -> Option<WorkspaceStep> {
        match self {
            WorkspaceStep::Step2 => Some(WorkspaceStep::Step3),
            WorkspaceStep::Step3 => Some(WorkspaceStep::Step4),
            WorkspaceStep::Step4 => Some(WorkspaceStep::Step5),
            WorkspaceStep::Step5 => None,
        }
    }

    /// The previous step, or `None` if this is the first.
    pub fn prev(self) -> Option<WorkspaceStep> {
        match self {
            WorkspaceStep::Step2 => None,
            WorkspaceStep::Step3 => Some(WorkspaceStep::Step2),
            WorkspaceStep::Step4 => Some(WorkspaceStep::Step3),
            WorkspaceStep::Step5 => Some(WorkspaceStep::Step4),
        }
    }
}

/// Fork provenance shown in the workspace header (badge + sub-line) and fed
/// to the `ForkInfoPopup`. The holder type is fixed in Run 1 so the model is
/// stable; population (from the parsed parent share code at fork time) lands
/// in Run 2/3 (P6.T5 / P6.T8). `forked_from` reuses BIO's `pub(crate)`
/// `ForkAncestor` (carve-out #5) — no parallel type.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ForkMeta {
    /// Immediate parent's display name.
    pub parent_name: String,
    /// Immediate parent's author handle.
    pub parent_author: String,
    /// Mods preselected from the parent (for the header sub-line).
    pub mods: u32,
    /// Components preselected from the parent (for the header sub-line).
    pub components: u32,
    /// Full lineage chain oldest → newest (the parent's chain ++ parent).
    /// `pub(crate)` (not `pub`): the element type `ForkAncestor` is BIO's
    /// `pub(crate)` carve-out-#5 struct; same-crate visibility avoids a
    /// `private_interfaces` warning and is sufficient (no external library
    /// consumer — the orchestrator binary is same-crate).
    pub(crate) forked_from: Vec<crate::app::modlist_share::ForkAncestor>,
}

/// Per-modlist workspace view state.
#[derive(Debug, Clone)]
pub struct WorkspaceViewState {
    /// Registry id of the modlist currently being edited.
    pub modlist_id: String,
    /// Display name (registry `ModlistEntry.name`); the header shows
    /// `Editing <name>`.
    pub modlist_name: String,
    /// Fork provenance, if this modlist was forked (Run 2/3 populates).
    pub fork_meta: Option<ForkMeta>,
    /// The modlist's chosen game family (immutable once the workspace opens —
    /// SPEC §5.1). Drives single- vs dual-game tabs in Steps 2-4.
    pub game: Game,
    /// The active workspace step.
    pub current_step: WorkspaceStep,
    /// Steps the user has advanced past (drives the progress-bar checkmarks).
    pub completed_steps: HashSet<WorkspaceStep>,
    /// Inline-rename active (P6.T5 — Run 2; inert this run).
    pub renaming: bool,
    /// Inline-rename scratch buffer (P6.T5 — Run 2; inert this run).
    pub rename_temp: String,
    /// `✓ saved!` flash deadline (P6.T6 — Run 2; inert this run).
    pub save_draft_flash_until: Option<Instant>,
    /// Share import code dialog open (Phase 7; inert this run).
    pub share_paste_open: bool,
    /// Flipped to `true` post-successful-install (Phase 7).
    pub install_complete: bool,
    /// Which modlist's data is currently loaded into the orchestrator's
    /// shared `WizardState`. The loader compares this to the routed id to
    /// decide when a populate/swap is needed (P6.T1 / P6.T12).
    pub loaded_workspace_id: Option<String>,
}

impl Default for WorkspaceViewState {
    fn default() -> Self {
        Self {
            modlist_id: String::new(),
            modlist_name: String::new(),
            fork_meta: None,
            game: Game::default(),
            // SPEC §3.2 / P6.T14: the workspace always opens at Step 2 for
            // v1 alpha.
            current_step: WorkspaceStep::Step2,
            completed_steps: HashSet::new(),
            renaming: false,
            rename_temp: String::new(),
            save_draft_flash_until: None,
            share_paste_open: false,
            install_complete: false,
            loaded_workspace_id: None,
        }
    }
}

impl WorkspaceViewState {
    /// True if `step` is marked completed (has a checkmark in the progress
    /// bar).
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
