// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

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
