// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::widgets` — workspace-local widgets that aren't broadly
// reusable across the orchestrator (those live in
// `crate::ui::orchestrator::widgets`). Per the Phase-6 file inventory.
//
// Phase 6 Run 2 ships:
//   - `weidu_line` → the three-colour WeiDU-line renderer (SPEC §6.7) used
//     by the Step-4 review list (P6.T2b). Net-new redesign-token chrome —
//     BIO's `content_step4::render_weidu_colored_line` (which uses BIO's
//     legacy `theme_global` colours) is **not** called.
//
// SPEC: §6.7, §8.1.

pub mod weidu_line;
