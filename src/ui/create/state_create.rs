// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::Game;
use crate::ui::install::state_install::DestChoice;

#[derive(Debug, Clone, Default)]
pub struct CreateScreenState {
    pub modlist_name: String,
    pub game: Game,
    pub destination: String,
    pub destination_choice: Option<DestChoice>,
    pub load_draft_open: bool,

    pub resumed_build_id: Option<String>,

    pub load_draft_copied_name: Option<String>,
    pub load_draft_copied_until: Option<std::time::Instant>,

    pub load_draft_delete_target: Option<String>,
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
        assert!(s.modlist_name.is_empty());
        assert!(s.destination.is_empty());
        assert_eq!(s.destination_choice, None);
        assert!(!s.load_draft_open);
        assert_eq!(s.resumed_build_id, None);
        assert_eq!(s.load_draft_delete_target, None);
    }

    #[test]
    fn derive_default_is_bgee_so_new_is_required_for_eet() {
        assert_eq!(CreateScreenState::default().game, Game::BGEE);
        assert_eq!(CreateScreenState::new().game, Game::EET);
    }
}
