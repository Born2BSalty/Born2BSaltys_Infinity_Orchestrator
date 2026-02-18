// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::model::{PromptInfo, PromptKind};

pub(crate) fn prompt_kind_name(prompt: &PromptInfo) -> &'static str {
    match prompt.kind {
        PromptKind::YesNo => "yes/no",
        PromptKind::Path => "path",
        PromptKind::Number => "number",
        PromptKind::FreeText => "text",
    }
}
