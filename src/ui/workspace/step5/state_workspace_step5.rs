// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty
//
// `WorkspaceStep5State` — net-new local UI state for the Step-5 chrome
// wrap-around only. BIO's Step-5 panel state (console-filter selection,
// auto-scroll, prompt-answers panel open) lives in `Step5ConsoleViewState`
// (the BIO field already owned by `OrchestratorApp` as
// `step5_console_view`) — this struct never duplicates it. Per SPEC §1
// this is net-new orchestrator state; BIO's `state_step5` is untouched.
//
// **Run 1 scope.** Run 1 needs exactly one live field — the
// `install_clicked` marker that drives P7.T8's `← Previous` lock (in Run 1
// there is no real install yet, so `Step5Action::StartInstall` only flips
// this marker; Run 2 P7.T3 adds the real install-start hook). The
// remaining fields are the Run-3 chrome-state holders (the plan's File
// inventory: `share_dialog_open`, `post_install_action_pending`),
// declared now so the model is stable, **inert this run** — mirroring the
// established staged-field pattern (`WorkspaceViewState`'s Phase-7
// fields). The owner is `OrchestratorApp.workspace_step5` (P7.T1).
//
// SPEC: §9.2 (Previous lock), §9.2/§9.3 (post-install actions — Run 3),
//       §10.3 (Share import code dialog — Run 3).

use crate::registry::model::Game;

/// A pending post-install action chosen in the post-install action row.
/// Declared in Run 1 so the model is stable; the action row that produces
/// it (`post_install_actions::render`) is implemented in Run 3 (P7.T5),
/// so this is **unused this run**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostInstallAction {
    /// `Return to Home` — navigate back to the Home destination.
    ReturnToHome,
    /// `Open install folder` — reveal the destination in the OS file
    /// manager.
    OpenInstallFolder,
}

/// Per-modlist Step-5 chrome state.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceStep5State {
    /// **Run-1 install-clicked marker (drives the P7.T8 `← Previous`
    /// lock).** Set `true` the frame `Step5Action::StartInstall` is
    /// dispatched by `page_workspace_step5::render`. Per SPEC §9.2 the
    /// workspace `← Previous` is disabled "once Install has been clicked —
    /// even before the install completes, and after it completes". In Run 1
    /// there is no real install, so this is the *only* signal that Install
    /// was clicked; the P7.T8 lock condition OR-combines it with
    /// `WorkspaceViewState::install_complete` and
    /// `state.step5.install_running` (the real install signals, wired in
    /// Run 2 / Run 3). Reset with the rest of the chrome state on a modlist
    /// swap (see [`WorkspaceStep5State::reset_for_modlist`]).
    pub install_clicked: bool,

    // ---------- Run-3 chrome state (declared now, inert this run) ----------
    /// The Share import code dialog open (Run 3 / P7.T7). Declared now so
    /// the model is stable; `share_paste_code_dialog::render` (Run 3) reads
    /// it. **Unused in Run 1.**
    pub share_dialog_open: bool,
    /// A post-install action chosen this frame, applied by the orchestrator
    /// after the render borrows end (Run 3 / P7.T5 — `Return to Home` /
    /// `Open install folder`). **Unused in Run 1.**
    pub post_install_action_pending: Option<PostInstallAction>,
}

impl WorkspaceStep5State {
    /// Reset the chrome state for a freshly-loaded / swapped modlist (the
    /// documented "reset with the rest of the view state on a modlist swap"
    /// contract — the `WorkspaceStep2State::default()` precedent in
    /// `page_router::render_workspace`). Run 1 wires this from the chrome
    /// renderer's per-modlist guard so a stale install-clicked marker from
    /// a *previous* modlist cannot lock the swapped-in modlist's
    /// `← Previous`.
    pub fn reset_for_modlist(&mut self) {
        *self = Self::default();
    }
}

/// (Run-1 convenience) Whether the modlist's chosen game family is the
/// dual-game EET workflow. Not used by the Run-1 chrome scaffold itself
/// (the banner/post-install rows are empty pre-install), but kept tiny +
/// pure so Run 3's banner can resolve the per-game summary without
/// re-deriving it. Pure helper, no state — testable in isolation.
#[must_use]
pub fn is_dual_game(game: Game) -> bool {
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
    }

    #[test]
    fn reset_for_modlist_clears_install_clicked() {
        let mut s = WorkspaceStep5State {
            install_clicked: true,
            share_dialog_open: true,
            post_install_action_pending: Some(PostInstallAction::ReturnToHome),
        };
        s.reset_for_modlist();
        assert!(
            !s.install_clicked,
            "a modlist swap must clear the install-clicked marker so it \
             cannot lock the swapped-in modlist's Previous"
        );
        assert!(!s.share_dialog_open);
        assert!(s.post_install_action_pending.is_none());
    }

    #[test]
    fn is_dual_game_only_for_eet() {
        assert!(is_dual_game(Game::EET));
        assert!(!is_dual_game(Game::BGEE));
        assert!(!is_dual_game(Game::BG2EE));
    }
}
