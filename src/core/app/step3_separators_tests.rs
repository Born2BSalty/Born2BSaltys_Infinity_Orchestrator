// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::*;
use crate::app::step3_history;

fn header(mod_name: &str, block_id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}/SETUP-{mod_name}.TP2"),
        component_id: "__PARENT__".to_string(),
        mod_name: mod_name.to_string(),
        component_label: mod_name.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: block_id.to_string(),
        is_parent: true,
        parent_placeholder: false,
    }
}

fn component(mod_name: &str, block_id: &str, component_id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("{mod_name}/setup-{mod_name}.tp2"),
        component_id: component_id.to_string(),
        mod_name: mod_name.to_string(),
        component_label: format!("Component {component_id}"),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: block_id.to_string(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn pair(mod_name: &str, component_id: &str) -> (String, String) {
    (
        format!("{mod_name}/SETUP-{mod_name}.TP2"),
        component_id.to_string(),
    )
}

fn anchor(name: &str, block_id: Option<&str>, following: Vec<(String, String)>) -> SeparatorAnchor {
    SeparatorAnchor {
        name: name.to_string(),
        block_id: block_id.map(str::to_string),
        collapsed: false,
        following,
    }
}

fn shape(items: &[Step3ItemState]) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            if item.is_separator() {
                format!("[{}]", item.mod_name)
            } else {
                format!("{}{}", item.mod_name, item.component_id)
            }
        })
        .collect()
}

#[test]
fn sanitize_trims_hashes_and_at_signs() {
    assert_eq!(
        sanitize_separator_name("  ### Fi@xes ##  "),
        Some("Fixes".to_string())
    );
    assert_eq!(
        sanitize_separator_name("@wlb-inputs: 1"),
        Some("wlb-inputs: 1".to_string())
    );
    assert_eq!(
        sanitize_separator_name("Big\nworld\r\nlast"),
        Some("Big world  last".to_string())
    );
    assert_eq!(
        sanitize_separator_name("C# tweaks"),
        Some("C# tweaks".to_string())
    );
}

#[test]
fn sanitize_caps_at_sixty_chars() {
    let long = "x".repeat(SEPARATOR_NAME_MAX_CHARS + 25);
    let cleaned = sanitize_separator_name(&long).expect("a long name is kept");
    assert_eq!(cleaned.chars().count(), SEPARATOR_NAME_MAX_CHARS);

    let wide = "é".repeat(SEPARATOR_NAME_MAX_CHARS + 1);
    let cleaned = sanitize_separator_name(&wide).expect("a long name is kept");
    assert_eq!(cleaned.chars().count(), SEPARATOR_NAME_MAX_CHARS);
}

#[test]
fn sanitize_blank_is_none() {
    assert_eq!(sanitize_separator_name(""), None);
    assert_eq!(sanitize_separator_name("   "), None);
    assert_eq!(sanitize_separator_name(" ## @ ## "), None);
    assert_eq!(sanitize_separator_name("\n\t"), None);
}

#[test]
fn format_then_parse_round_trips() {
    for name in ["Fixes", "NPCs and quests", "Big-world last", "C# tweaks"] {
        let line = format_separator_log_line(name);
        assert_eq!(parse_separator_log_line(&line), Some(name.to_string()));
    }
    assert_eq!(format_separator_log_line("NPCs"), "// ###### NPCs ######");
    assert_eq!(
        parse_separator_log_line("  //###NPCs###  "),
        Some("NPCs".to_string())
    );
}

#[test]
fn parse_ignores_weidu_header_comments() {
    for line in [
        "// Log of Currently Installed WeiDU Mods",
        "// The top of the file is the 'oldest' mod",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
        "~EEFIXPACK/SETUP-EEFIXPACK.TP2~ #0 #0 // Core fixes",
        "// a note from the user",
        "",
    ] {
        assert_eq!(parse_separator_log_line(line), None, "{line}");
    }
}

#[test]
fn parse_needs_three_hashes_each_side() {
    assert_eq!(parse_separator_log_line("// ## NPCs ##"), None);
    assert_eq!(parse_separator_log_line("// ### NPCs ##"), None);
    assert_eq!(parse_separator_log_line("// ## NPCs ###"), None);
    assert_eq!(parse_separator_log_line("// ### NPCs"), None);
    assert_eq!(parse_separator_log_line("// ######"), None);
    assert_eq!(parse_separator_log_line("// ### ###"), None);
    assert_eq!(
        parse_separator_log_line("// ### NPCs ###"),
        Some("NPCs".to_string())
    );
}

#[test]
fn insert_separators_uses_first_surviving_anchor() {
    let mut children = vec![
        component("A", "", "0"),
        component("B", "", "0"),
        component("D", "", "0"),
        component("E", "", "0"),
    ];
    let anchors = vec![
        anchor(
            "Fixes",
            Some("SEPARATOR::1"),
            vec![
                pair("A", "0"),
                pair("B", "0"),
                pair("C", "0"),
                pair("D", "0"),
            ],
        ),
        anchor(
            "NPCs",
            Some("SEPARATOR::2"),
            vec![pair("C", "0"), pair("D", "0")],
        ),
    ];

    insert_separators(&mut children, &anchors);

    assert_eq!(
        shape(&children),
        ["[Fixes]", "A0", "B0", "[NPCs]", "D0", "E0"]
    );
    assert_eq!(children[0].block_id, "SEPARATOR::1");
    assert_eq!(children[3].block_id, "SEPARATOR::2");
}

#[test]
fn insert_separators_without_survivor_goes_to_end() {
    let mut children = vec![component("A", "", "0"), component("B", "", "0")];
    let anchors = vec![
        anchor("Gone", None, vec![pair("C", "0"), pair("D", "1")]),
        anchor("Empty", None, Vec::new()),
    ];

    insert_separators(&mut children, &anchors);

    assert_eq!(shape(&children), ["A0", "B0", "[Gone]", "[Empty]"]);
    assert_eq!(children[2].block_id, "SEPARATOR::1");
    assert_eq!(children[3].block_id, "SEPARATOR::2");
}

#[test]
fn insert_separators_keeps_order_at_same_spot() {
    let mut children = vec![component("A", "", "0"), component("B", "", "0")];
    let anchors = vec![
        anchor("First", Some("SEPARATOR::4"), vec![pair("B", "0")]),
        anchor(
            "Second",
            Some("SEPARATOR::2"),
            vec![pair("X", "9"), pair("B", "0")],
        ),
        anchor("Third", Some("SEPARATOR::7"), vec![pair("B", "0")]),
    ];

    insert_separators(&mut children, &anchors);

    assert_eq!(
        shape(&children),
        ["A0", "[First]", "[Second]", "[Third]", "B0"]
    );
}

#[test]
fn capture_records_following_components_and_collapse() {
    let items = vec![
        separator_row("Fixes", "SEPARATOR::1".to_string()),
        header("A", "A::b0"),
        component("A", "A::b0", "0"),
        separator_row("NPCs", "SEPARATOR::2".to_string()),
        header("B", "B::b0"),
        component("B", "B::b0", "3"),
        component("B", "B::b0", "5"),
    ];
    let collapsed = vec!["SEPARATOR::2".to_string(), "A::b0".to_string()];

    let anchors = capture_separator_anchors(&items, &collapsed);

    assert_eq!(
        anchors,
        vec![
            SeparatorAnchor {
                name: "Fixes".to_string(),
                block_id: Some("SEPARATOR::1".to_string()),
                collapsed: false,
                following: vec![pair("A", "0"), pair("B", "3"), pair("B", "5")],
            },
            SeparatorAnchor {
                name: "NPCs".to_string(),
                block_id: Some("SEPARATOR::2".to_string()),
                collapsed: true,
                following: vec![pair("B", "3"), pair("B", "5")],
            },
        ]
    );
}

#[test]
fn section_indices_stop_at_next_separator() {
    let items = vec![
        header("A", "A::b0"),
        component("A", "A::b0", "0"),
        separator_row("Fixes", "SEPARATOR::1".to_string()),
        header("B", "B::b0"),
        component("B", "B::b0", "0"),
        separator_row("NPCs", "SEPARATOR::2".to_string()),
        header("C", "C::b0"),
        component("C", "C::b0", "0"),
    ];

    assert_eq!(section_indices(&items, 2), vec![2, 3, 4]);
    assert_eq!(section_indices(&items, 5), vec![5, 6, 7]);
    assert_eq!(enclosing_separator(&items, 1), None);
    assert_eq!(enclosing_separator(&items, 2), Some(2));
    assert_eq!(enclosing_separator(&items, 4), Some(2));
    assert_eq!(enclosing_separator(&items, 7), Some(5));
}

fn three_sections() -> Vec<Step3ItemState> {
    vec![
        header("Fixpack", "Fixpack::b0"),
        separator_row("Utilities", "SEPARATOR::1".to_string()),
        header("Eeex", "Eeex::b0"),
        separator_row("Quests", "SEPARATOR::2".to_string()),
        header("Heart", "Heart::b0"),
        header("Ruins", "Ruins::b0"),
        separator_row("Tweaks", "SEPARATOR::3".to_string()),
        header("Klatu", "Klatu::b0"),
    ]
}

fn names(items: &[Step3ItemState]) -> Vec<&str> {
    items.iter().map(|item| item.mod_name.as_str()).collect()
}

#[test]
fn a_section_moves_before_another_separator_or_to_the_end() {
    let mut items = three_sections();
    assert!(move_section(&mut items, 1, Some(6)));
    assert_eq!(
        names(&items),
        [
            "Fixpack",
            "Quests",
            "Heart",
            "Ruins",
            "Utilities",
            "Eeex",
            "Tweaks",
            "Klatu"
        ]
    );

    let mut items = three_sections();
    let top = first_separator(&items);
    assert!(move_section(&mut items, 6, top));
    assert_eq!(
        names(&items),
        [
            "Fixpack",
            "Tweaks",
            "Klatu",
            "Utilities",
            "Eeex",
            "Quests",
            "Heart",
            "Ruins"
        ]
    );

    let mut items = three_sections();
    assert!(move_section(&mut items, 1, None));
    assert_eq!(names(&items)[5..], ["Klatu", "Utilities", "Eeex"]);
}

#[test]
fn a_section_never_lands_where_it_sits_or_inside_another_section() {
    for before in [Some(1), Some(3), Some(4)] {
        let mut items = three_sections();
        assert!(!move_section(&mut items, 1, before));
        assert_eq!(items, three_sections());
    }
    let mut items = three_sections();
    assert!(!move_section(&mut items, 6, None));
    assert_eq!(items, three_sections());
}

#[test]
fn next_block_id_increments() {
    let mut items = vec![header("A", "A::b0"), component("A", "A::b0", "0")];
    assert_eq!(next_separator_block_id(&items), "SEPARATOR::1");

    items.push(separator_row("Fixes", "SEPARATOR::1".to_string()));
    assert_eq!(next_separator_block_id(&items), "SEPARATOR::2");

    items.push(separator_row("NPCs", "SEPARATOR::7".to_string()));
    assert_eq!(next_separator_block_id(&items), "SEPARATOR::8");
}

fn two_mods() -> Vec<Step3ItemState> {
    vec![
        header("A", "A::b0"),
        component("A", "A::b0", "0"),
        header("B", "B::b0"),
        component("B", "B::b0", "0"),
        component("B", "B::b0", "1"),
    ]
}

#[test]
fn add_separator_goes_above_the_lit_mod_block() {
    let mut items = two_mods();
    let mut undo = Vec::new();
    let mut redo = Vec::new();

    let block_id = add_separator(&mut items, Some(4), &mut undo, &mut redo);

    assert_eq!(block_id, "SEPARATOR::1");
    assert!(items[2].is_separator());
    assert_eq!(items[2].mod_name, DEFAULT_SEPARATOR_NAME);
    assert_eq!(items[2].block_id, block_id);
    assert!(items[3].is_mod_header());
    assert_eq!(items[3].mod_name, "B");

    let second = add_separator(&mut items, Some(2), &mut undo, &mut redo);
    assert_eq!(second, "SEPARATOR::2");
    assert!(items[2].is_separator());
    assert_eq!(items[2].block_id, second);
    assert_eq!(items[3].block_id, block_id);
}

#[test]
fn add_separator_without_selection_goes_to_end() {
    let mut items = two_mods();
    let mut undo = Vec::new();
    let mut redo = Vec::new();

    add_separator(&mut items, None, &mut undo, &mut redo);

    assert_eq!(items.len(), 6);
    assert!(items[5].is_separator());
    assert!(items[..5].iter().all(|item| !item.is_separator()));
}

#[test]
fn add_then_undo_removes_it() {
    let original = two_mods();
    let mut items = original.clone();
    let mut undo = Vec::new();
    let mut redo = Vec::new();

    add_separator(&mut items, Some(1), &mut undo, &mut redo);
    assert_eq!(items.len(), original.len() + 1);
    assert_eq!(undo.len(), 1);

    step3_history::undo(&mut items, &mut undo, &mut redo);

    assert_eq!(items, original);
    assert_eq!(redo.len(), 1);
}
