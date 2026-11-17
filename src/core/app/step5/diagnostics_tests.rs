// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::*;
use crate::app::step3_separators::separator_row;

fn header(mod_name: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("SETUP-{mod_name}.TP2"),
        component_id: "__PARENT__".to_string(),
        mod_name: mod_name.to_string(),
        component_label: mod_name.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: format!("{mod_name}::segment1"),
        is_parent: true,
        parent_placeholder: false,
    }
}

fn component(mod_name: &str, component_id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: format!("SETUP-{mod_name}.TP2"),
        component_id: component_id.to_string(),
        mod_name: mod_name.to_string(),
        component_label: format!("Component {component_id}"),
        raw_line: format!("~{mod_name}/SETUP-{mod_name}.TP2~ #0 #{component_id} // Component"),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: format!("{mod_name}::segment1"),
        is_parent: false,
        parent_placeholder: false,
    }
}

#[test]
fn export_lines_carry_separator_lines_in_place() {
    let items = vec![
        separator_row("Fixes", "SEPARATOR::1".to_string()),
        header("A"),
        component("A", "0"),
        separator_row("NPCs", "SEPARATOR::2".to_string()),
        header("B"),
        component("B", "1"),
        component("B", "2"),
    ];

    assert_eq!(
        build_weidu_export_lines(&items),
        vec![
            "// ###### Fixes ######".to_string(),
            "~A\\SETUP-A.TP2~ #0 #0 // Component".to_string(),
            "// ###### NPCs ######".to_string(),
            "~B\\SETUP-B.TP2~ #0 #1 // Component".to_string(),
            "~B\\SETUP-B.TP2~ #0 #2 // Component".to_string(),
        ]
    );
}

#[test]
fn export_lines_without_separators_unchanged() {
    let items = vec![
        header("A"),
        component("A", "0"),
        header("B"),
        component("B", "1"),
    ];

    let expected: Vec<String> = items
        .iter()
        .filter(|item| !item.is_parent)
        .map(format_step4_item)
        .collect();
    assert_eq!(build_weidu_export_lines(&items), expected);
    assert_eq!(
        expected,
        vec![
            "~A\\SETUP-A.TP2~ #0 #0 // Component".to_string(),
            "~B\\SETUP-B.TP2~ #0 #1 // Component".to_string(),
        ]
    );
}
