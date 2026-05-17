// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/create/` — the Create destination (SPEC §5: "author a new
// modlist"). Net-new module tree; no BIO source modifications.
//
// Phase 6 Run 3 ships (the Create *entry path*):
//   - `state_create`        — `CreateScreenState` + `CreateStage`
//                             (`Choose | ForkPaste | ForkPreview |
//                             ForkDownload`; the fork stages are declared so
//                             the dispatch + back-nav are total, their
//                             renderers are Run 4).
//   - `page_create`         — top-level renderer; dispatches on `CreateStage`
//                             + applies the deferred app-level effects
//                             (create + atomic registry persist + nav).
//   - `stage_choose`        — the `choose` mode (P6.T7): setup Box (name +
//                             game ComboBox + destination FolderInput +
//                             conditional REUSED Phase-5
//                             `DestinationNotEmptyWarning`) + the two
//                             starting-point cards.
//   - `load_draft_dialog`   — the non-blocking Load Draft dialog (P6.T9 /
//                             SPEC §5.2): in-progress builds as the REUSED
//                             Phase-5 `modlist_card`; `resume` opens the
//                             workspace (NOT a file picker).
//   - `destination_default` — pure default-destination computation
//                             (`<config_dir>/modlists/installs/<slug>`).
//
// Phase 6 Run 4 adds the fork sub-flow renderers (`stage_fork_paste` /
// `stage_fork_preview` / `stage_fork_download`) + the `operations_create`
// lineage append (P6.T8) — NOT this run (the `Fork*` stages render a
// deferred placeholder until then).
//
// SPEC: §5 (Create), §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork —
//       Run 4), §13.1, §13.14.

pub mod destination_default;
pub mod load_draft_dialog;
pub mod page_create;
pub mod stage_choose;
pub mod state_create;
