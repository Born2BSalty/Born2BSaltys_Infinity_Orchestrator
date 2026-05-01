// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::selected_details::selected_source_reference;
use crate::app::selection_refs::source_path_from_reference;
use crate::app::state::WizardState;

pub fn rule_source_open_path(state: &WizardState) -> Option<String> {
    source_path_from_reference(
        selected_source_reference(state)
            .as_deref()
            .unwrap_or_default(),
    )
}
