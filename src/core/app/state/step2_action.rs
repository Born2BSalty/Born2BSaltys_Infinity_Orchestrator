// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModSourceEditDestination {
    #[default]
    GlobalDefault,
    ThisModlist,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TravelFilesState {
    pub(crate) mod_name: String,
    pub(crate) compared_against: Option<String>,
    pub(crate) rows: Vec<(String, &'static str)>,
    pub(crate) truncated: bool,
    pub(crate) missing: Vec<String>,
    pub(crate) invalid: Vec<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step2Action {
    StartScan,
    CancelScan,
    SelectBgeeViaLog,
    SelectBg2eeViaLog,
    OpenUpdatePopup,
    PreviewUpdateSelected,
    PreviewUpdateSelectedMod,
    DownloadUpdates,
    DownloadUpdateFor {
        tp2: String,
    },
    AcceptLatestForExactVersionMisses,
    OpenSelectedReadme(String),
    OpenSelectedWeb(String),
    OpenSelectedTp2Folder(String),
    OpenSelectedTp2(String),
    OpenSelectedIni(String),
    OpenModDownloadsUserSource,
    ReloadModDownloadSources,
    DiscoverModDownloadForks {
        tp2: String,
        label: String,
        repo: String,
    },
    SaveSourceForm,
    RequestReleaseList {
        repo: String,
    },
    UseKnownSource {
        tp2: String,
        card_key: String,
        block: String,
        save_to: ModSourceEditDestination,
        who: String,
    },
    SaveSourceNote {
        tp2: String,
        signature: String,
        text: String,
        who: String,
    },
    BookmarkOnDisk {
        tp2: String,
        card_key: String,
    },
    ShowTravelFiles {
        tp2: String,
    },
    SetModDownloadSource {
        tp2: String,
        source_id: String,
    },
    SetSelectedModUpdateLocked(bool),
    SetModUpdateLocked {
        tp2: String,
        locked: bool,
    },
    OpenCompatForComponent {
        game_tab: String,
        tp_file: String,
        component_id: String,
        component_key: String,
    },
}
