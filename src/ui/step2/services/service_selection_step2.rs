// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::selected_details::selected_source_reference;
use crate::app::selection_refs::source_path_from_reference;
use crate::app::state::WizardState;

#[must_use]
pub fn rule_source_open_path(state: &WizardState) -> Option<String> {
    let path = source_path_from_reference(
        selected_source_reference(state)
            .as_deref()
            .unwrap_or_default(),
    )?;
    Path::new(&path).is_file().then_some(path)
}
