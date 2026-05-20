// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::time::Instant;

use crate::registry::model::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostInstallAction {
    ReturnToHome,
    OpenInstallFolder,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceStep5State {
    pub install_clicked: bool,

    pub share_dialog_open: bool,
    pub post_install_action_pending: Option<PostInstallAction>,
    pub copied_flash_until: Option<Instant>,
}

impl WorkspaceStep5State {
    pub fn reset_for_modlist(&mut self) {
        *self = Self::default();
    }
}

#[must_use]
pub const fn is_dual_game(game: Game) -> bool {
    matches!(game, Game::EET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_not_install_clicked() {
        let s = WorkspaceStep5State::default();
        assert!(
            !s.install_clicked,
            "a fresh workspace has not had Install clicked"
        );
        assert!(!s.share_dialog_open);
        assert!(s.post_install_action_pending.is_none());
        assert!(s.copied_flash_until.is_none());
    }

    #[test]
    fn reset_for_modlist_clears_install_clicked() {
        let mut s = WorkspaceStep5State {
            install_clicked: true,
            share_dialog_open: true,
            post_install_action_pending: Some(PostInstallAction::ReturnToHome),
            copied_flash_until: Some(Instant::now()),
        };
        s.reset_for_modlist();
        assert!(
            !s.install_clicked,
            "a modlist swap must clear the install-clicked marker so it \
             cannot lock the swapped-in modlist's Previous"
        );
        assert!(!s.share_dialog_open);
        assert!(s.post_install_action_pending.is_none());
        assert!(
            s.copied_flash_until.is_none(),
            "a modlist swap clears the copied-flash too"
        );
    }

    #[test]
    fn is_dual_game_only_for_eet() {
        assert!(is_dual_game(Game::EET));
        assert!(!is_dual_game(Game::BGEE));
        assert!(!is_dual_game(Game::BG2EE));
    }
}
