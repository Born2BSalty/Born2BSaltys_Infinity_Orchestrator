// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{detect_derived_collapsible_groups, detect_weidu_groups};
use crate::ui::scan::ScannedComponent;

#[test]
fn deprecated_placeholder_without_group_inherits_surrounding_group() {
    let tp2_text = r#"
BEGIN ~Choice A~ DESIGNATED 2350
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~

BEGIN ~Choice B~ DESIGNATED 2351
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~

BEGIN ~deprecated~ DESIGNATED 2352 DEPRECATED @18

BEGIN ~Choice C~ DESIGNATED 2353
GROUP ~Gameplay~
SUBCOMPONENT ~Multiclass~

BEGIN ~Other Choice~ DESIGNATED 2400
GROUP ~Other~
"#;
    let components = vec![
        component("2350", "Multiclass -> Choice A"),
        component("2351", "Multiclass -> Choice B"),
        component("2352", "deprecated"),
        component("2353", "Multiclass -> Choice C"),
        component("2400", "Other Choice"),
    ];

    let groups = detect_weidu_groups("", tp2_text, &components);

    assert_eq!(groups.get("2350").map(String::as_str), Some("Gameplay"));
    assert_eq!(groups.get("2351").map(String::as_str), Some("Gameplay"));
    assert_eq!(groups.get("2352").map(String::as_str), Some("Gameplay"));
    assert_eq!(groups.get("2353").map(String::as_str), Some("Gameplay"));
    assert_eq!(groups.get("2400").map(String::as_str), Some("Other"));
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

#[test]
fn deprecated_placeholder_choice_stays_in_same_collapsible_family() {
    let tp2_text = r#"
BEGIN ~Allow humans to multi-class~ DESIGNATED 2350
SUBCOMPONENT ~Alter Multi-Class Restrictions~

BEGIN ~Allow non-humans access to all multi-class combinations~ DESIGNATED 2351
SUBCOMPONENT ~Alter Multi-Class Restrictions~

BEGIN ~deprecated~ DESIGNATED 2352 DEPRECATED @18

BEGIN ~Allow non-humans access to selected combinations~ DESIGNATED 2353
SUBCOMPONENT ~Alter Multi-Class Restrictions~

BEGIN ~Install options one and two (everyone can multi-class anything)~ DESIGNATED 2357
SUBCOMPONENT ~Alter Multi-Class Restrictions~

BEGIN ~Install options one and three (everyone can multi-class anything they can single-class)~ DESIGNATED 2358
SUBCOMPONENT ~Alter Multi-Class Restrictions~
"#;
    let components = vec![
        component(
            "2350",
            "Alter Multi-Class Restrictions -> Allow humans to multi-class",
        ),
        component(
            "2351",
            "Alter Multi-Class Restrictions -> Allow non-humans access to all multi-class combinations",
        ),
        component(
            "2352",
            "Install options one and two (everyone can multi-class anything)",
        ),
        component(
            "2353",
            "Alter Multi-Class Restrictions -> Allow non-humans access to selected combinations",
        ),
        component(
            "2357",
            "Alter Multi-Class Restrictions -> Install options one and two (everyone can multi-class anything)",
        ),
        component(
            "2358",
            "Alter Multi-Class Restrictions -> Install options one and three (everyone can multi-class anything they can single-class)",
        ),
    ];

    let groups = detect_derived_collapsible_groups("setup-cdtweaks.tp2", tp2_text, &components);

    for component_id in ["2350", "2351", "2352", "2353", "2357", "2358"] {
        let group = groups.get(component_id).expect("missing derived collapsible group");
        assert_eq!(group.header, "Alter Multi-Class Restrictions");
        assert!(!group.is_umbrella);
    }
}

#[test]
fn subcomponent_parent_component_stays_in_same_collapsible_family() {
    let tp2_text = r#"
BEGIN @151 DESIGNATED 151 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager1

BEGIN @152 DESIGNATED 152 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager2

BEGIN @150 DESIGNATED 150 GROUP @1 LABEL Morpheus562sKitpackBattlerager3
"#;
    let components = vec![
        component("151", "Install Battlerager Kit -> Assign kit to Korgan"),
        component("152", "Install Battlerager Kit -> Do NOT assign kit to Korgan"),
        component("150", "Install Battlerager Kit"),
    ];

    let groups =
        detect_derived_collapsible_groups("morpheus562-s-kitpack.tp2", tp2_text, &components);

    let root = groups.get("150").expect("missing root derived collapsible group");
    assert_eq!(root.header, "Install Battlerager Kit");
    assert!(root.is_umbrella);

    for component_id in ["151", "152"] {
        let group = groups
            .get(component_id)
            .expect("missing child derived collapsible group");
        assert_eq!(group.header, "Install Battlerager Kit");
        assert!(!group.is_umbrella);
    }
}
