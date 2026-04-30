// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

#[path = "compat_rule_runtime_active.rs"]
mod active;
#[path = "compat_rule_runtime_matches.rs"]
mod matches;
#[path = "compat_rule_runtime_relations.rs"]
mod relations;

pub(crate) use active::{
    CompatActiveItem, active_item_order, collect_step2_active_items, collect_step3_active_items,
};
pub(crate) use matches::{
    clear_kind_matches, compat_component_matches, compat_mod_matches, kind_disables_selection,
    match_kind_matches, mode_matches, non_empty, normalize_kind, normalize_mod_key, tab_matches,
};
pub(crate) use relations::{
    direct_rule_applies, game_dir_for_tab, matched_related_target, relation_rule_applies,
    single_related_target,
};

#[cfg(test)]
#[path = "compat_rule_runtime_tests.rs"]
mod tests;
