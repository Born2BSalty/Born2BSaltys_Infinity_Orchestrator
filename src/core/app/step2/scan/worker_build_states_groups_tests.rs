// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::{detect_derived_collapsible_groups, detect_weidu_groups};
use crate::app::scan::ScannedComponent;

#[test]
fn deprecated_placeholder_without_group_inherits_surrounding_group() {
    let tp2_text = r"
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
";
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
    let tp2_text = r"
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
";
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
        let group = groups
            .get(component_id)
            .expect("missing derived collapsible group");
        assert_eq!(group.header, "Alter Multi-Class Restrictions");
        assert!(!group.is_umbrella);
    }
}

#[test]
fn subcomponent_parent_component_stays_in_same_collapsible_family() {
    let tp2_text = r"
BEGIN @151 DESIGNATED 151 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager1

BEGIN @152 DESIGNATED 152 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager2

BEGIN @150 DESIGNATED 150 GROUP @1 LABEL Morpheus562sKitpackBattlerager3
";
    let components = vec![
        component("151", "Install Battlerager Kit -> Assign kit to Korgan"),
        component(
            "152",
            "Install Battlerager Kit -> Do NOT assign kit to Korgan",
        ),
        component("150", "Install Battlerager Kit"),
    ];

    let groups =
        detect_derived_collapsible_groups("morpheus562-s-kitpack.tp2", tp2_text, &components);

    let root = groups
        .get("150")
        .expect("missing root derived collapsible group");
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

#[test]
fn versioned_subcomponent_parent_groups_with_its_children() {
    let tp2_text = r"
BEGIN @151 DESIGNATED 151 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager1

BEGIN @152 DESIGNATED 152 SUBCOMPONENT @150 GROUP @1 LABEL Morpheus562sKitpackBattlerager2

BEGIN @150 DESIGNATED 150 GROUP @1 LABEL Morpheus562sKitpackBattlerager3
";
    let components = vec![
        component(
            "151",
            "Install Battlerager Kit -> Assign kit to Korgan: v2.0",
        ),
        component(
            "152",
            "Install Battlerager Kit -> Do NOT assign kit to Korgan: v2.0",
        ),
        component("150", "Install Battlerager Kit: v2.0"),
    ];

    let groups =
        detect_derived_collapsible_groups("morpheus562-s-kitpack.tp2", tp2_text, &components);

    let root = groups
        .get("150")
        .expect("missing root derived collapsible group");
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

#[test]
fn versioned_questpack_translation_references_still_do_not_group() {
    let (tp2_text, components) = questpack_translation_collision_fixture();
    let components = components
        .into_iter()
        .map(|scanned| {
            let display = format!("{}: v1.0", scanned.display);
            component(&scanned.component_id, &display)
        })
        .collect::<Vec<_>>();
    let groups = detect_derived_collapsible_groups("setup-d0questpack.tp2", tp2_text, &components);

    for component in &components {
        assert!(
            !groups.contains_key(&component.component_id),
            "component #{} must not acquire a parent family from a translation ID collision",
            component.component_id,
        );
    }
}

fn questpack_translation_collision_fixture() -> (&'static str, Vec<ScannedComponent>) {
    let tp2_text = r"
BEGIN @39
SUBCOMPONENT @6
DESIGNATED 4

BEGIN @40
SUBCOMPONENT @6
DESIGNATED 401

BEGIN @2
DESIGNATED 6

BEGIN @11
DESIGNATED 11
SUBCOMPONENT @13

BEGIN @12
DESIGNATED 12
SUBCOMPONENT @13

BEGIN @17
DESIGNATED 13
";
    let components = vec![
        component(
            "4",
            "Miscellaneous Enhancements -> With Additional Random Encounters",
        ),
        component(
            "401",
            "Miscellaneous Enhancements -> Without Additional Random Encounters",
        ),
        component("6", "Alternative Harper/Xzar Plot"),
        component(
            "11",
            "Improved Oasis II -> IO2 Dialogue & Combat Enhancement",
        ),
        component("12", "Improved Oasis II -> IO2 Dialogue Enhancement Only"),
        component("13", "Saving Sanik In Brynnlaw"),
    ];
    (tp2_text, components)
}

#[test]
fn questpack_translation_references_do_not_create_unrelated_parents() {
    let (tp2_text, components) = questpack_translation_collision_fixture();
    let groups = detect_derived_collapsible_groups("setup-d0questpack.tp2", tp2_text, &components);

    for component in &components {
        assert!(
            !groups.contains_key(&component.component_id),
            "component #{} must not acquire a parent family from a translation ID collision",
            component.component_id,
        );
    }
}

#[test]
fn questpack_selection_preserves_independent_components_and_radio_choices() {
    use crate::app::state::Step2ModState;
    use crate::ui::step2::tree_selection_rules_step2::{
        enforce_collapsible_group_umbrella_rules, enforce_subcomponent_single_select,
    };
    use std::collections::{HashMap, HashSet};

    let (tp2_text, components) = questpack_translation_collision_fixture();
    let groups = detect_derived_collapsible_groups("setup-d0questpack.tp2", tp2_text, &components);
    let states =
        super::super::component_states_for(components, &HashMap::new(), &groups, &HashSet::new());
    let initial = Step2ModState {
        name: "Quest Pack".to_string(),
        tp_file: "setup-d0questpack.tp2".to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: false,
        hidden_components: Vec::new(),
        components: states,
    };
    let select = |state: &mut Step2ModState, id: &str| {
        let index = state
            .components
            .iter()
            .position(|c| c.component_id == id)
            .unwrap();
        state.components[index].checked = true;
        state.components[index].selected_order = Some(index + 1);
        enforce_subcomponent_single_select(state, index);
        enforce_collapsible_group_umbrella_rules(state, index);
    };
    let checked = |state: &Step2ModState, id: &str| {
        state
            .components
            .iter()
            .find(|c| c.component_id == id)
            .unwrap()
            .checked
    };

    for (choice, independent) in [("4", "6"), ("401", "6"), ("11", "13"), ("12", "13")] {
        for (first, second) in [(choice, independent), (independent, choice)] {
            let mut state = initial.clone();
            select(&mut state, first);
            select(&mut state, second);
            assert!(
                checked(&state, first) && checked(&state, second),
                "#{first} and #{second} must coexist"
            );
        }
    }
    for (first, second, independent) in [
        ("4", "401", "6"),
        ("401", "4", "6"),
        ("11", "12", "13"),
        ("12", "11", "13"),
    ] {
        let mut state = initial.clone();
        select(&mut state, independent);
        select(&mut state, first);
        select(&mut state, second);
        assert!(!checked(&state, first), "#{second} must replace #{first}");
        assert!(checked(&state, second));
        assert!(
            checked(&state, independent),
            "#{independent} must stay selected"
        );
    }
}
