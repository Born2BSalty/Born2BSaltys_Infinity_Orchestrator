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
// Run 2 ("Home — actions live") adds:
//   - `confirm_delete`         — Delete + Reinstall confirm bodies (P5.T7 / T18).
//   - `toast`                  — bottom-center transient toast (P5.T16).
//
// Phase 7 Run 4b (P7.T10) adds:
//   - `reinstall_route_wire`   — wires the Reinstall confirm's Confirm
//                                button to the real Install-Modlist
//                                Reinstall route (replaces the Phase-5
//                                placeholder-toast seam; SPEC §3.1).

pub mod add_a_modlist;
pub mod confirm_delete;
pub mod filter_chip;
pub mod first_launch_setup_card;
pub mod game_installs_detected;
pub mod modlist_card;
pub mod page_home;
pub mod reinstall_route_wire;
pub mod state_home;
pub mod toast;
