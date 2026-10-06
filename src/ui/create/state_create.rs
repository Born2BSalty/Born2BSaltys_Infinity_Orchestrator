// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::Game;
use crate::ui::install::state_install::DestChoice;
use crate::ui::workspace::state_workspace::WeiduLogImportForm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateMode {
    #[default]
    FromLogs,
    FromScratch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogCheck {
    Valid { components: usize, mods: usize },
    NotALog,
}

#[derive(Debug, Clone, Default)]
pub struct CreateScreenState {
    pub modlist_name: String,
    pub game: Game,
    pub destination: String,
    pub destination_choice: Option<DestChoice>,
    pub load_draft_open: bool,

    pub resumed_build_id: Option<String>,

    pub load_draft_delete_target: Option<String>,

    pub mode: CreateMode,
    pub log_form: WeiduLogImportForm,
    pub first_check: Option<LogCheck>,
    pub second_check: Option<LogCheck>,
}

impl CreateScreenState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            game: Game::EET,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_eet() {
        let s = CreateScreenState::new();
        assert_eq!(s.game, Game::EET);
        assert_eq!(s.modlist_name.len(), 0);
        assert_eq!(s.destination.len(), 0);
        assert_eq!(s.destination_choice, None);
        assert!(!s.load_draft_open);
        assert_eq!(s.resumed_build_id, None);
        assert_eq!(s.load_draft_delete_target, None);
        assert_eq!(s.mode, CreateMode::FromLogs);
        assert!(s.log_form.fetch_missing);
        assert_eq!(s.log_form.first, None);
        assert_eq!(s.log_form.second, None);
        assert_eq!(s.first_check, None);
        assert_eq!(s.second_check, None);
    }

    #[test]
    fn derive_default_is_bgee_so_new_is_required_for_eet() {
        assert_eq!(CreateScreenState::default().game, Game::BGEE);
        assert_eq!(CreateScreenState::new().game, Game::EET);
    }
}
