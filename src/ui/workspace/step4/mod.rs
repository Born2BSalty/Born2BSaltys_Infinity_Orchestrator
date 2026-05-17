// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::step4` — the **Step-4 C4 module tree** (P6.T2b). The
// direct analogue of `step2/` (P6.T2c): a net-new orchestrator-side Step-4
// renderer that **does not** call BIO's `bio::ui::step4::page_step4::render`
// (per C4 — that would double the Save button, since BIO's
// `content_step4::content_step4` already paints one). The legacy `BIO`
// binary's `WizardApp::update_loop` keeps invoking `page_step4::render`
// normally; the orchestrator simply doesn't go through it.
//
// Phase 6 Run 2 ships:
//   - `workspace_step4`         → the top-level C4 renderer (Save row + tab
//                                 strip + body branch). `pub fn render(ui,
//                                 orchestrator) -> Option<Step4Action>`.
//   - `step4_save_row`          → the top action row (`Save weidu.log['s]`
//                                 button + the count text).
//   - `step4_review_list`       → the line-numbered three-colour review
//                                 list (normal install modes).
//   - `step4_exact_log_viewer`  → the read-only source-WeiDU-log viewer +
//                                 `Check Mod List` (exact-log mode).
//
// Reuses only BIO's data + `pub(crate)` action helpers read-only
// (`auto_save_step4_weidu_logs` / `app_step4_flow::handle_step4_action` /
// `source_log_infos`); the rendering surface is net-new redesign chrome.
//
// SPEC: §8.1, §8.2, §6.7, Appendix A.7.

pub mod step4_exact_log_viewer;
pub mod step4_review_list;
pub mod step4_save_row;
pub mod workspace_step4;
