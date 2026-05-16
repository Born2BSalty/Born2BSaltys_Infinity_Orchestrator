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

use crate::app::state::Step2Selection;
use crate::registry::model::Game;

/// Orchestrator-owned Step-2 chrome state (P6.T2c). The Step-2 C4 wrapper
/// owns the **Details-pane visibility** because the wireframe's Step 2 hides
/// the Details panel by default (SPEC §6: "the Details panel is hidden by
/// default") whereas BIO's `frame_step2` always renders it in the split.
/// This is net-new chrome state — BIO's `state_step2` is untouched.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceStep2State {
    /// Details pane visible. **Default `false`** (SPEC §6 — hidden by
    /// default; opened via the Kebab "Show Details panel" toggle or by
    /// selecting a tree row, mirroring the wireframe `ComponentTree`
    /// `onSelect → setDetailsOpen(true)`).
    pub details_open: bool,
    /// Snapshot of `wizard_state.step2.selected` from the previous frame.
    /// When the live selection transitions to a *new* value (a row / `[?]`
    /// click in BIO's reused tree), the wrapper auto-opens the Details
    /// panel — the egui equivalent of the wireframe `ComponentTree`'s
    /// `onSelect`/`onOpenDetails` callbacks (BIO's tree has no separate
    /// detail-open signal; a row click sets `state.step2.selected`).
    pub last_selection: Option<Step2Selection>,
    /// **Rescan-reconcile snapshot** (SPEC §6.3, the #2 fix). Captured at
    /// scan-trigger time — the current selection as
    /// `(tp2.to_ascii_uppercase(), component_id, selected_order)` over every
    /// checked component on both tabs — and re-applied onto the freshly
    /// scanned mod set when the (async) scan **completes** (the fresh set
    /// has landed via `OrchestratorApp::poll_step2_channels`). `None` when
    /// no rescan is pending. Orchestrator-owned: BIO's `state_step2` is
    /// untouched and BIO's scan is non-preserving by design.
    pub rescan_snapshot: Option<RescanSnapshot>,
    /// Previous-frame `wizard_state.step2.is_scanning`, so the reconcile can
    /// fire exactly on the scan-completion edge (`true → false`) — the
    /// moment BIO's `Step2ScanEvent::Finished` handler has replaced the mod
    /// vectors. Drained-before-render in `OrchestratorApp::update`.
    pub was_scanning: bool,
    /// Post-reconcile drop warning for the scan-status footer (SPEC §6.3:
    /// _"N component(s) dropped — M mod(s) no longer present"_). `Some` only
    /// when a completed rescan dropped at least one selected component;
    /// cleared on the next scan trigger.
    pub rescan_drop_warning: Option<String>,
    /// **Pending Select-via-WeiDU-Log destructive confirm** (SPEC §6.10 +
    /// wireframe `askWeiduImport`, `screens.jsx:2778-2784`). Select-via-Log
    /// replaces *every* component selection on the tab, so the tab-row
    /// button does **not** dispatch the picker directly — it arms this with
    /// the target tab (`Some(true)` = BGEE, `Some(false)` = BG2EE) and
    /// `workspace_step2::render` shows the danger `ConfirmDialog`. Only on
    /// **Confirm** does it dispatch `Step2Action::Select{Bgee,Bg2ee}ViaLog`
    /// (the unchanged `step2_log_glue` picker+apply path); **Cancel/dismiss**
    /// just clears this — nothing changes. `None` = no confirm in flight.
    /// Orchestrator-owned (BIO's `state_step2` is untouched).
    pub pending_weidu_log_confirm: Option<bool>,
}

/// One captured selection entry for the rescan-reconcile (SPEC §6.3, the #2
/// fix): the component's `tp2` (upper-cased — BIO matches `tp_file`
/// case-insensitively, the precedent being
/// `workspace_state_loader::apply_order_to_mods`), its `component_id`, and
/// its `selected_order` so the install order is preserved across the
/// rescan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescanSelection {
    pub tp2_upper: String,
    pub component_id: String,
    pub selected_order: Option<usize>,
}

/// The full pre-scan selection snapshot — both game tabs (BIO buckets
/// single-game modlists, incl. IWDEE, into `bgee_mods`; EET uses both).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RescanSnapshot {
    pub bgee: Vec<RescanSelection>,
    pub bg2ee: Vec<RescanSelection>,
}

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
    /// Net-new Step-2 chrome state (Details-pane visibility — hidden by
    /// default per SPEC §6 — + the selection snapshot driving auto-open).
    /// Reset with the rest of the view state on a modlist swap so a fresh
    /// workspace starts with the Details panel hidden (P6.T2c).
    pub step2: WorkspaceStep2State,
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
            step2: WorkspaceStep2State::default(),
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
