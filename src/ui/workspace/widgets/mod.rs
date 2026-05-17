// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::workspace::widgets` — workspace-local widgets that aren't broadly
// reusable across the orchestrator (those live in
// `crate::ui::orchestrator::widgets`). Per the Phase-6 file inventory.
//
// Phase 6 ships:
//   - `weidu_line` → the three-colour WeiDU-line renderer (SPEC §6.7) used
//     by the Step-4 review list (P6.T2b). Net-new redesign-token chrome —
//     BIO's `content_step4::render_weidu_colored_line` (which uses BIO's
//     legacy `theme_global` colours) is **not** called.
//   - `game_tab` → the ONE shared wireframe `GameTab` (no bottom bar in any
//     state), used identically by Step 2 / Step 4 / (Step-3 C4) — replaces
//     the two former per-step duplicate painters.
//
// SPEC: §6.4, §6.7, §7, §8.1.

pub mod game_tab;
pub mod weidu_line;
