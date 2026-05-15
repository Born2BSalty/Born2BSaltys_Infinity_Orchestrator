// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/install/` — the Install Modlist destination (SPEC §4: "consume
// someone else's modlist" — paste → preview → downloading → installing).
//
// Phase 5 ships (Runs 3-4):
//   - `state_install`         — `InstallScreenState` + `InstallStage` (the
//                               whole 4-stage machine) + `DestChoice` and the
//                               SPEC §13.12 flag mapping + (Run 4) the
//                               preview state (`parsed_preview`,
//                               `active_preview_tab`, `fork_info_open`,
//                               `PreviewTab`).
//   - `page_install`          — top-level renderer; dispatches on the stage.
//   - `stage_paste`           — Stage 1 (SPEC §4.1), fully implemented.
//   - `destination_not_empty` — the yellow `DestinationNotEmptyWarning` Box
//                               (wireframe screens.jsx:123-154, verbatim).
//   - `sub_flow_footer`       — the shared sub-flow footer
//                               (wireframe screens.jsx:3494-3510); Run 4
//                               grew it with an optional secondary slot.
//   - `stage_preview`         — Stage 2 (SPEC §4.2): parsed share-code
//                               preview + Overview Box + 6-tab Content Box +
//                               `allow_auto_install` gate + provenance +
//                               `ForkInfoPopup` (Run 4).
//   - `preview_tabs`          — the 6-tab file-folder strip + per-tab body
//                               (SPEC §4.2; wireframe screens.jsx:469-529).
//   - `stage_installing_stub` — Stage 4 stub (SPEC §4.4; full runtime is
//                               Phase 7).
//
// Run 5 adds `stage_downloading` (the per-mod download/extract grid). Until
// then, `page_install` renders the Run-5 placeholder for that stage so the
// build is whole.

pub mod destination_not_empty;
pub mod page_install;
pub mod preview_tabs;
pub mod stage_installing_stub;
pub mod stage_paste;
pub mod stage_preview;
pub mod state_install;
pub mod sub_flow_footer;
