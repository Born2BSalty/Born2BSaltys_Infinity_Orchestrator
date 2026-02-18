// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::Step3ItemState;

pub fn step3_item_key(item: &Step3ItemState) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        item.tp_file,
        item.mod_name,
        item.component_id,
        item.component_label,
        item.raw_line,
        item.selected_order
    )
}

pub(super) fn mod_key(item: &Step3ItemState) -> String {
    format!(
        "{}::{}",
        item.tp_file.to_ascii_uppercase(),
        item.mod_name.to_ascii_uppercase()
    )
}
