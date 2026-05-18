// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/install/` — the Install Modlist destination (SPEC §4: "consume
// someone else's modlist" — paste → preview → downloading → installing).
//
// Phase 5 ships (Runs 3-5):
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
//   - `stage_downloading`     — Stage 3 (SPEC §4.3): the net-new
//                               `ImportDownloadScreen` surface — overall-
//                               progress Box + 4-column per-mod grid (mod /
//                               source / status / progress) + footer (Run 5,
//                               P5.T12). Reusable for Phase 6's fork-download
//                               (`DownloadScreenCopy`); only the Install path
//                               is wired this run. **The live download/extract
//                               orchestration is escalated as a SPEC CONFLICT
//                               / PLAN GAP and is intentionally UNWIRED this
//                               run** — see `stage_downloading.rs`'s module
//                               header (BIO has no "share code → download
//                               list" surface; the only producer is BIO's
//                               complex `modlist_auto_build` pipeline, a
//                               directive complex-pipeline workflow — not
//                               reimplemented, not forked, pending the user's
//                               decision). The screen renders the §4.3 chassis
//                               with an empty grid (navigable: Cancel →
//                               Preview) and is forward-compatible.
//   - `stage_installing`      — Stage 4, the real install runtime (SPEC
//                               §4.4 `InstallProgressScreen` / §9.3).
//                               **Phase 7 P7.T15 (Run 4b)** — replaces the
//                               Phase-5/Run-4a `stage_installing_stub`
//                               (deleted, mirroring the Run-1
//                               `workspace_step5_stub` "replace + delete
//                               the stub" precedent). Its own minimal
//                               chrome (simple header + back affordance +
//                               the C3-gated post-install row) wrapping
//                               BIO's embedded `page_step5::render`
//                               (read-only) — NOT the workspace 4-step
//                               progress bar / nav bar / Share header.

pub mod destination_not_empty;
pub mod page_install;
pub mod preview_counts;
pub mod preview_tabs;
pub mod stage_downloading;
pub mod stage_installing;
pub mod stage_paste;
pub mod stage_preview;
pub mod state_install;
pub mod sub_flow_footer;
