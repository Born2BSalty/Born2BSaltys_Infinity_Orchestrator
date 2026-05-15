// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::selected_details::selected_details_data;
use crate::app::state::WizardState;
use crate::ui::step2::details_meta_step2::map_selected_details;
use crate::ui::step2::state_step2::Step2Details;

#[must_use]
pub fn selected_details(state: &WizardState) -> Step2Details {
    map_selected_details(selected_details_data(state))
}
