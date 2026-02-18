// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptKind {
    YesNo,
    Path,
    Number,
    FreeText,
}

pub(crate) struct PromptInfo {
    pub(crate) key: String,
    pub(crate) legacy_key: Option<String>,
    pub(crate) preview_line: String,
    pub(crate) kind: PromptKind,
    pub(crate) option_count: usize,
    pub(crate) line_count: usize,
    pub(crate) char_count: usize,
}
