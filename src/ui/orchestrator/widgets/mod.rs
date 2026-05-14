// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Shared redesign widgets used by orchestrator screens. Phase 2 ships:
//   - `screen_title`  → `ScreenTitle` primitive (22px Poppins 500 + sub)
//   - `btn`           → `redesign_btn` (sketchy border, primary / small)
//   - `r_box`         → `redesign_box` (sketchy chassis + optional corner label)
//   - `label`         → `redesign_label` + `redesign_label_hand` (two variants)
//
// Each widget mirrors its wireframe counterpart (file/line cited in each
// widget file).
//
// Later phases add `pill`, `kebab`, `chip`, `tab_strip`, `confirm_dialog`,
// `toast`, `clipboard` (per the Phase 2 file inventory comment).

pub mod btn;
pub mod label;
pub mod r_box;
pub mod screen_title;

pub use btn::{BtnOpts, redesign_btn};
pub use label::{redesign_label, redesign_label_hand};
pub use r_box::redesign_box;
pub use screen_title::render as render_screen_title;
