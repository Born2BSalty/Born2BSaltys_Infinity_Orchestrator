// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::detect_hidden_prompt_like_component_ids;
use crate::ui::scan::ScannedComponent;

#[test]
fn does_not_hide_non_dummy_deprecated_placeholder_label() {
    let tp2_text = r#"
BEGIN @235001 DESIGNATED 2350
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~

BEGIN @235100 DESIGNATED 2351
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~

BEGIN @235200 DESIGNATED 2352 DEPRECATED @18

// this used to be the multiclass madness component, but with the new multiclass option the component number had to be changed

BEGIN @235300 DESIGNATED 2353
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~
"#;

    let hidden = detect_hidden_prompt_like_component_ids(
        None,
        Some(tp2_text),
        &[
            component("2350", "Multiclass -> Choice A"),
            component("2351", "Multiclass -> Choice B"),
            component("2352", "Install options one and two"),
            component("2353", "Multiclass -> Choice C"),
        ],
    );

    assert!(!hidden.contains_key("2352"));
}

#[test]
fn hides_dummy_deprecated_placeholder_by_component_id() {
    let tp2_text = r#"
BEGIN @100 DESIGNATED 100 DEPRECATED @18
"#;

    let hidden = detect_hidden_prompt_like_component_ids(
        None,
        Some(tp2_text),
        &[component("100", "dummy")],
    );

    assert_eq!(
        hidden.get("100").map(String::as_str),
        Some("deprecated_dummy_placeholder")
    );
}

#[test]
fn hides_nested_other_no_log_record_utility_component() {
    let tp2_text = r#"
BEGIN ~BGEE_to_EET_mod_checker~ DESIGNATED 0
NO_LOG_RECORD
"#;

    let hidden = detect_hidden_prompt_like_component_ids(
        Some(
            "/mods/EET/other/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker/BGEE_to_EET_mod_checker.tp2",
        ),
        Some(tp2_text),
        &[component("0", "BGEE_to_EET_mod_checker: beta 0.1")],
    );

    assert_eq!(
        hidden.get("0").map(String::as_str),
        Some("nested_other_no_log_record_utility")
    );
}

fn component(component_id: &str, display: &str) -> ScannedComponent {
    ScannedComponent {
        tp_file: None,
        component_id: component_id.to_string(),
        display: display.to_string(),
        raw_line: display.to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
    }
}
