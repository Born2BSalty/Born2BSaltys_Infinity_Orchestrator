// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{compat_component_matches, compat_mod_matches, game_dir_for_tab};
use crate::app::state::{Step1State, Step2ComponentState};

use super::super::compat_rules::{CompatRule, StringOrMany};

#[test]
fn game_dir_for_tab_reads_the_iwdee_folder_on_the_first_slot() {
    let step1 = Step1State {
        game_install: "IWDEE".to_string(),
        iwdee_game_folder: "/games/iwdee".to_string(),
        bgee_game_folder: "/games/bgee".to_string(),
        generate_directory_enabled: false,
        ..Step1State::default()
    };
    assert_eq!(game_dir_for_tab(&step1, "IWDEE"), Some("/games/iwdee"));
    assert_eq!(game_dir_for_tab(&step1, "BGEE"), Some("/games/iwdee"));
}

#[test]
fn game_dir_for_tab_prefers_generate_directory_for_iwdee() {
    let step1 = Step1State {
        game_install: "IWDEE".to_string(),
        iwdee_game_folder: "/games/iwdee".to_string(),
        generate_directory_enabled: true,
        generate_directory: "/games/generated".to_string(),
        ..Step1State::default()
    };
    assert_eq!(game_dir_for_tab(&step1, "IWDEE"), Some("/games/generated"));
}

#[test]
fn compat_mod_matches_any_item_from_mod_list() {
    let rule = test_rule(
        StringOrMany::Many(vec!["eet_end".to_string(), "eet_gui".to_string()]),
        None,
    );

    assert!(compat_mod_matches(&rule, "setup-EET_end.tp2", "EET_end"));
    assert!(compat_mod_matches(&rule, "setup-EET_gui.tp2", "EET GUI"));
    assert!(!compat_mod_matches(&rule, "setup-EET.tp2", "EET"));
}

#[test]
fn compat_component_matches_star_component_id() {
    let rule = test_rule(
        StringOrMany::One("eet_end".to_string()),
        Some(StringOrMany::One("*".to_string())),
    );
    let component = Step2ComponentState {
        component_id: "42".to_string(),
        label: "Any component".to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: "BEGIN @42 DESIGNATED 42".to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        is_meta_mode_component: false,
        disabled: false,
        compat_kind: None,
        compat_source: None,
        compat_related_mod: None,
        compat_related_component: None,
        compat_graph: None,
        compat_evidence: None,
        disabled_reason: None,
        checked: false,
        selected_order: None,
    };

    assert!(compat_component_matches(
        &rule,
        &component.component_id,
        &component.label,
        &component.raw_line,
    ));
}

fn test_rule(r#mod: StringOrMany, component_id: Option<StringOrMany>) -> CompatRule {
    CompatRule {
        enabled: true,
        r#mod,
        component: None,
        component_id,
        mode: None,
        tab: None,
        kind: "mismatch".to_string(),
        match_kind: None,
        clear_kinds: None,
        position: None,
        path_field: None,
        path_check: None,
        game_file: None,
        game_file_check: None,
        message: String::new(),
        source: None,
        related_mod: None,
        related_component: None,
        loaded_from: None,
    }
}
