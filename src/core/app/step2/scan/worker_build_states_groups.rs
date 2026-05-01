// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;

use crate::app::scan::ScannedComponent;

use super::tp2_blocks::Tp2ComponentBlock;

#[path = "worker_build_states_groups_collapsible.rs"]
mod collapsible;
#[path = "worker_build_states_groups_weidu.rs"]
mod weidu;

#[derive(Debug, Clone)]
pub(super) struct DerivedCollapsibleGroup {
    pub header: String,
    pub is_umbrella: bool,
}

pub(super) fn detect_derived_collapsible_groups(
    tp_file: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, DerivedCollapsibleGroup> {
    collapsible::detect_derived_collapsible_groups(tp_file, tp2_text, components)
}

pub(super) fn detect_weidu_groups(
    tp2_path: &str,
    tp2_text: &str,
    components: &[ScannedComponent],
) -> HashMap<String, String> {
    weidu::detect_weidu_groups(tp2_path, tp2_text, components)
}

fn block_is_deprecated_placeholder(block: &Tp2ComponentBlock) -> bool {
    let mut saw_deprecated = false;
    for line in &block.body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();
        if upper.contains("DEPRECATED") {
            saw_deprecated = true;
        }
        if upper.starts_with("BEGIN ")
            || upper.starts_with("GROUP ")
            || upper.starts_with("LABEL ")
            || upper.starts_with("SUBCOMPONENT ")
            || upper.starts_with("FORCED_SUBCOMPONENT ")
            || upper.starts_with("REQUIRE_")
            || upper.starts_with("DESIGNATED ")
        {
            continue;
        }
        return false;
    }
    saw_deprecated
}

#[cfg(test)]
#[path = "worker_build_states_groups_tests.rs"]
mod tests;
