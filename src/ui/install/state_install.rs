// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::modlist_share::ModlistSharePreview;
use crate::ui::step5::action_step5::Step5Action;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStage {
    Paste,
    Preview,
    Installing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallPreviewTab {
    Summary,
    BgeeWeidu,
    Bg2eeWeidu,
    UserDownloads,
    InstalledRefs,
    ModConfigs,
}

#[derive(Debug, Clone)]
pub struct InstallScreenState {
    pub(crate) stage: InstallStage,
    pub(crate) destination: String,
    pub(crate) import_code: String,
    pub(crate) preview: Option<ModlistSharePreview>,
    pub(crate) preview_error: Option<String>,
    pub(crate) preview_tab: InstallPreviewTab,
    pub(crate) reinstall_modlist_id: Option<String>,
}

impl Default for InstallScreenState {
    fn default() -> Self {
        Self {
            stage: InstallStage::Paste,
            destination: String::new(),
            import_code: String::new(),
            preview: None,
            preview_error: None,
            preview_tab: InstallPreviewTab::Summary,
            reinstall_modlist_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallAction {
    Step5(Step5Action),
    BeginInstallPreviewAccepted,
}
