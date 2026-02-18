// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

pub(super) fn format_issue_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}
