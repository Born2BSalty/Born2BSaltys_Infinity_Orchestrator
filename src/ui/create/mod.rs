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
// Phase 6 Run 4 adds the fork sub-flow renderers (P6.T8 / SPEC §5.3),
// replacing Run 3's deferred placeholder:
//   - `stage_fork_paste`    — `ForkPasteScreen`: the import-code Box +
//                             `Back`/`Preview →` footer (the reused Phase-5
//                             `sub_flow_footer`; the textarea is
//                             wireframe-faithful net-new per the Run-3
//                             module-private precedent).
//   - `stage_fork_preview`  — `ForkPreviewScreen`: the parsed *parent*
//                             code's packed name/author + `⑂ fork info`
//                             (the **reused Phase-5 `ForkInfoPopup`**, parent
//                             lineage) + the reused `preview_tabs` chassis;
//                             `Begin Import →`, **no `allow_auto_install`
//                             gate** (SPEC §13.3 — forking is always
//                             allowed).
//   - `stage_fork_download` — drives the **reused Phase-5
//                             `stage_downloading` chassis** with the
//                             fork copy; the live fetch is Phase 7 P7.T17
//                             (SPEC §13.12a — chassis only here).
// The `operations_create::create_forked_modlist` lineage append (SPEC §13.3
// /§5.3) + the caller-anchored registry/workspace IO + the route into the
// forked Workspace live in `page_create` (the `start_scratch` precedent).
//
// SPEC: §5 (Create), §5.1 (choose mode), §5.2 (Load Draft), §5.3 (fork
//       sub-flow), §13.3 (Provenance / lineage append), §13.1, §13.14.

pub mod destination_default;
pub mod load_draft_dialog;
pub mod page_create;
pub mod stage_choose;
pub mod stage_fork_download;
pub mod stage_fork_paste;
pub mod stage_fork_preview;
pub mod state_create;
