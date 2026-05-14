// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// Settings-screen widget collection.
//
// Per Phase 4 P4.T8 file inventory:
//   - `tab_strip`        — file-folder tabs at the top of the page.
//   - `path_row`         — label + mono value field + browse button + hint.
//   - `value_row`        — single-input field with placeholder + hint
//                          (absorb-the-gate pattern, SPEC §11.5).
//   - `toggle_row`       — label + toggle switch + hint.
//   - `segmented_toggle` — two-option Light/Dark style switch.
//   - `name_row`         — edit-in-place name field for the General tab.
//   - `account_card`     — service card chassis used by `tab_accounts`.

pub mod account_card;
pub mod name_row;
pub mod path_row;
pub mod segmented_toggle;
pub mod tab_strip;
pub mod toggle_row;
pub mod value_row;
