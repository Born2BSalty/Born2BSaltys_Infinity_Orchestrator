// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `bio::ui::settings` — Phase 4 redesigned Settings screen.
//
// Per Phase 4: a 5-tab Settings destination (General, Paths, Tools, Accounts,
// Advanced) backed by:
//   - `bio_settings.json` (existing BIO `SettingsStore`) for paths / tools /
//     advanced values.
//   - `bio_redesign_settings.json` (`RedesignSettingsStore`) for user_name,
//     theme, language, diagnostic_mode (the redesign-only fields).
//
// All edits persist immediately; an orchestrator-owned debounce cycle writes
// to disk roughly once per second when the user's values stabilize.
//
// SPEC: §11.

pub mod oauth_glue;
pub mod page_settings;
pub mod state_settings;
pub mod tab_accounts;
pub mod tab_advanced;
pub mod tab_general;
pub mod tab_paths;
pub mod tab_tools;
pub mod validate_debounce;
pub mod validate_now;
pub mod widgets;

pub use state_settings::{SettingsScreenState, SettingsTab};
