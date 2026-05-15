// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `src/ui/home/` — the Home destination (SPEC §3).
//
// Phase 5 / Run 1 ("Home — visual + navigation") ships:
//   - `page_home`              — top-level renderer (empty vs non-empty).
//   - `state_home`             — `HomeScreenState` + filter/default logic.
//   - `filter_chip`            — the Installed/In-progress/All chips.
//   - `modlist_card`           — both card types (in-progress / installed).
//   - `add_a_modlist`          — the right-column CTAs Box.
//   - `game_installs_detected` — the detected-games lines.
//   - `first_launch_setup_card`— the empty-registry setup CTA.
//
// Run 2 adds `confirm_delete` + `toast` (P5.T7 / T16 / T18); they are NOT
// declared here yet (the modules don't exist this run).

pub mod add_a_modlist;
pub mod filter_chip;
pub mod first_launch_setup_card;
pub mod game_installs_detected;
pub mod modlist_card;
pub mod page_home;
pub mod state_home;
