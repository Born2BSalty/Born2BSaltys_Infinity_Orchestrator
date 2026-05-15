// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::model::Game;
use crate::ui::create::destination_default;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateStage {
    Choose,
    ForkPaste,
    ForkPreview,
    ForkDownload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateAction {
    StartNewModlist,
    PasteShareCode,
    LoadDraftRequested,
}

#[derive(Debug, Clone)]
pub struct CreateScreenState {
    pub stage: CreateStage,
    pub modlist_name: String,
    pub game: Game,
    pub destination: String,
    pub load_draft_open: bool,
    pub fork_code: String,
    pub fork_preview_error: Option<String>,
    pub fork_preview_ready: bool,
}

impl Default for CreateScreenState {
    fn default() -> Self {
        Self {
            stage: CreateStage::Choose,
            modlist_name: String::new(),
            game: Game::EET,
            destination: destination_default::for_modlist_name("Untitled modlist"),
            load_draft_open: false,
            fork_code: String::new(),
            fork_preview_error: None,
            fork_preview_ready: false,
        }
    }
}
