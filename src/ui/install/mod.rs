// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/install/` — the Install Modlist destination (SPEC §4: "consume
// someone else's modlist" — paste → preview → downloading → installing).
//
// Phase 5 / Run 3 ("Install — shell + paste stage + stub") ships:
//   - `state_install`         — `InstallScreenState` + `InstallStage` (the
//                               whole 4-stage machine) + `DestChoice` and the
//                               SPEC §13.12 flag mapping.
//   - `page_install`          — top-level renderer; dispatches on the stage.
//   - `stage_paste`           — Stage 1 (SPEC §4.1), fully implemented.
//   - `destination_not_empty` — the yellow `DestinationNotEmptyWarning` Box
//                               (wireframe screens.jsx:123-154, verbatim).
//   - `sub_flow_footer`       — the shared sub-flow footer
//                               (wireframe screens.jsx:3494-3510).
//   - `stage_installing_stub` — Stage 4 stub (SPEC §4.4; full runtime is
//                               Phase 7).
//
// Run 4 adds `stage_preview` + `preview_tabs` (the share-code parse + the 6
// preview tabs + the `allow_auto_install` gate). Run 5 adds
// `stage_downloading` (the per-mod download/extract grid). Until then,
// `page_install` renders Run-4 / Run-5 placeholders for those stages so the
// build is whole.

pub mod destination_not_empty;
pub mod page_install;
pub mod stage_installing_stub;
pub mod stage_paste;
pub mod state_install;
pub mod sub_flow_footer;
