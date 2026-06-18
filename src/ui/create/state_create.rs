// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::modlist_share::ModlistSharePreview;
use crate::registry::model::Game;
use crate::ui::install::stage_downloading::DownloadProgress;
use crate::ui::install::state_install::{DestChoice, PreviewTab};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartingPoint {
    #[default]
    Scratch,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateStage {
    #[default]
    Choose,
    ForkPaste,
    ForkPreview,
    ForkDownload,
}

#[derive(Debug, Clone, Default)]
pub struct CreateScreenState {
    pub stage: CreateStage,
    pub modlist_name: String,
    pub game: Game,
    pub destination: String,
    pub destination_choice: Option<DestChoice>,
    pub starting_point: StartingPoint,
    pub load_draft_open: bool,

    pub fork_code: String,
    pub(crate) fork_preview: Option<ModlistSharePreview>,
    pub fork_preview_parse_error: Option<String>,
    pub fork_active_preview_tab: PreviewTab,
    pub fork_info_open: bool,
    pub fork_download_progress: DownloadProgress,

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

    pub fn clear_fork_preview(&mut self) {
        self.fork_preview = None;
        self.fork_preview_parse_error = None;
        self.fork_info_open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_eet_and_choose_stage() {
        let s = CreateScreenState::new();
        assert_eq!(s.game, Game::EET);
        assert_eq!(s.stage, CreateStage::Choose);
        assert!(s.modlist_name.is_empty());
        assert!(s.destination.is_empty());
        assert_eq!(s.destination_choice, None);
        assert!(!s.load_draft_open);
        assert_eq!(s.resumed_build_id, None);
        assert_eq!(s.starting_point, StartingPoint::Scratch);
        assert_eq!(s.load_draft_delete_target, None);
        assert!(s.fork_code.is_empty());
        assert!(s.fork_preview.is_none());
        assert!(s.fork_preview_parse_error.is_none());
        assert_eq!(s.fork_active_preview_tab, PreviewTab::Summary);
        assert!(!s.fork_info_open);
    }

    #[test]
    fn clear_fork_preview_resets_fork_preview_state() {
        let mut s = CreateScreenState::new();
        s.fork_preview_parse_error = Some("bad code".to_string());
        s.fork_info_open = true;
        s.clear_fork_preview();
        assert!(s.fork_preview.is_none());
        assert!(s.fork_preview_parse_error.is_none());
        assert!(!s.fork_info_open);
    }

    #[test]
    fn derive_default_is_bgee_so_new_is_required_for_eet() {
        assert_eq!(CreateScreenState::default().game, Game::BGEE);
        assert_eq!(CreateScreenState::new().game, Game::EET);
    }

    #[test]
    fn stage_default_is_choose() {
        assert_eq!(CreateStage::default(), CreateStage::Choose);
    }

    #[test]
    fn starting_point_default_is_scratch() {
        assert_eq!(StartingPoint::default(), StartingPoint::Scratch);
        assert_ne!(StartingPoint::Scratch, StartingPoint::Import);
    }
}
