// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

/// Where the source-editor saves its result: the user's global default file or
/// the active modlist's per-modlist file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModSourceEditDestination {
    /// Saves to the global `mod_downloads_user.toml` (existing behavior).
    #[default]
    GlobalDefault,
    /// Saves to the active modlist's `mod_downloads_user.toml`.
    ThisModlist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step2Action {
    StartScan,
    CancelScan,
    SelectBgeeViaLog,
    SelectBg2eeViaLog,
    OpenUpdatePopup,
    CheckExactLogModList,
    PreviewUpdateSelected,
    PreviewUpdateSelectedMod,
    DownloadUpdates,
    AcceptLatestForExactVersionMisses,
    OpenSelectedReadme(String),
    OpenSelectedWeb(String),
    OpenSelectedTp2Folder(String),
    OpenSelectedTp2(String),
    OpenSelectedIni(String),
    OpenModDownloadsUserSource,
    ReloadModDownloadSources,
    OpenModDownloadSourceEditor {
        tp2: String,
        label: String,
        source_id: String,
        allow_source_id_change: bool,
        destination: ModSourceEditDestination,
    },
    DiscoverModDownloadForks {
        tp2: String,
        label: String,
        repo: String,
    },
    AddDiscoveredModDownloadFork {
        tp2: String,
        label: String,
        full_name: String,
        owner_login: String,
        default_branch: String,
    },
    SaveModDownloadSourceEditor,
    SetModDownloadSource {
        tp2: String,
        source_id: String,
    },
    SetSelectedModUpdateLocked(bool),
    OpenCompatForComponent {
        game_tab: String,
        tp_file: String,
        component_id: String,
        component_key: String,
    },
}
