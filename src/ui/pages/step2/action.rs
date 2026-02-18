// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step2Action {
    StartScan,
    CancelScan,
    SelectBgeeViaLog,
    SelectBg2eeViaLog,
    RevalidateCompat,
    ExportCompatReport,
    OpenSelectedReadme(String),
    OpenSelectedWeb(String),
    OpenSelectedTp2(String),
    OpenCompatForComponent {
        game_tab: String,
        tp_file: String,
        component_id: String,
        component_key: String,
    },
}
