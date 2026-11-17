// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use super::*;
use crate::app::state::{Step1State, Step2ComponentState, Step2ModState, Step3ItemState};
use crate::app::step3_separators::separator_row;

struct SeparatorTestRoot(PathBuf);

impl SeparatorTestRoot {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bio_modlist_share_separator_test_{}_{id}_{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create temp root");
        Self(path)
    }

    fn folder(&self, name: &str) -> String {
        self.0.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for SeparatorTestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

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
        component_label: "Component".to_string(),
        raw_line: format!("~{mod_name}/SETUP-{mod_name}.TP2~ #0 #{component_id} // Component"),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 0,
        block_id: format!("{mod_name}::segment1"),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn sectioned_items(first: &str, second: &str) -> Vec<Step3ItemState> {
    vec![
        separator_row(first, "SEPARATOR::1".to_string()),
        header("A"),
        component("A", "0"),
        separator_row(second, "SEPARATOR::2".to_string()),
        header("B"),
        component("B", "1"),
        component("B", "2"),
    ]
}

fn pair(mod_name: &str, component_id: &str) -> (String, String) {
    (format!("SETUP-{mod_name}.TP2"), component_id.to_string())
}

fn log_anchor(name: &str, following: Vec<(String, String)>) -> SeparatorAnchor {
    SeparatorAnchor {
        name: name.to_string(),
        block_id: None,
        collapsed: false,
        following,
    }
}

fn anchors_as_read_from_a_log(items: &[Step3ItemState]) -> Vec<SeparatorAnchor> {
    capture_separator_anchors(items, &[])
        .into_iter()
        .map(|anchor| SeparatorAnchor {
            block_id: None,
            ..anchor
        })
        .collect()
}

#[test]
fn share_code_round_trips_separators() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
        "separator-share-round-trip",
    );
    let root = SeparatorTestRoot::new("round_trip");
    let mut state = WizardState::default();
    state.step1.game_install = "EET".to_string();
    state.step3.bgee_items = sectioned_items("Fixes", "NPCs");
    state.step3.bg2ee_items = vec![
        separator_row("Tweaks", "SEPARATOR::1".to_string()),
        header("C"),
        component("C", "3"),
    ];
    let code =
        export_modlist_share_code_with(&state, &ShareExportSources::default()).expect("export");

    let mut imported = WizardState::default();
    imported.step1.eet_bgee_log_folder = root.folder("bgee");
    imported.step1.eet_bg2ee_log_folder = root.folder("bg2ee");
    import_modlist_share_code(&mut imported, &code).expect("import");

    assert_eq!(
        imported.step3.pending_separators,
        vec![
            (
                GameSlot::First,
                anchors_as_read_from_a_log(&state.step3.bgee_items)
            ),
            (
                GameSlot::Second,
                anchors_as_read_from_a_log(&state.step3.bg2ee_items)
            ),
        ]
    );
    let written = std::fs::read_to_string(root.0.join("bgee").join("weidu.log")).expect("bgee log");
    assert!(
        written.contains("// ###### Fixes ######\n~A\\SETUP-A.TP2~ #0 #0 // Component"),
        "{written}"
    );
    assert!(
        written.contains("// ###### NPCs ######\n~B\\SETUP-B.TP2~ #0 #1 // Component"),
        "{written}"
    );
    drop(app);
}

#[test]
fn installed_log_gets_separator_lines_back() {
    let root = SeparatorTestRoot::new("installed");
    let weidu_header = "// Log of Currently Installed WeiDU Mods\r\n// The top of the file is the 'oldest' mod\r\n";
    let installed = format!(
        "{weidu_header}~A/SETUP-A.TP2~ #0 #0 // Component\r\n~C/SETUP-C.TP2~ #0 #9 // Other\r\n~B/SETUP-B.TP2~ #0 #2 // Component\r\n"
    );
    std::fs::write(root.0.join("weidu.log"), &installed).expect("installed log");
    let dir = root.0.to_string_lossy().into_owned();

    assert_eq!(
        installed_or_rebuilt_log(&dir, &sectioned_items("Fixes", "NPCs")),
        format!(
            "{weidu_header}// ###### Fixes ######\r\n~A/SETUP-A.TP2~ #0 #0 // Component\r\n~C/SETUP-C.TP2~ #0 #9 // Other\r\n// ###### NPCs ######\r\n~B/SETUP-B.TP2~ #0 #2 // Component\r\n"
        )
    );

    let without_separators: Vec<Step3ItemState> = sectioned_items("Fixes", "NPCs")
        .into_iter()
        .filter(|item| !item.is_separator())
        .collect();
    assert_eq!(
        installed_or_rebuilt_log(&dir, &without_separators),
        installed
    );

    assert_eq!(
        insert_separator_lines(
            "~A/SETUP-A.TP2~ #0 #0 // Component",
            &[log_anchor("Late", vec![pair("Z", "1")])]
        ),
        "~A/SETUP-A.TP2~ #0 #0 // Component\n// ###### Late ######\n"
    );
}

#[test]
fn separator_anchors_from_log_text_reads_names_and_following() {
    let text = [
        "// Log of Currently Installed WeiDU Mods",
        "// ~TP2_File~ #language_number #component_number // [Subcomponent Name -> ] Component Name [ : Version]",
        "~A/SETUP-A.TP2~ #0 #0 // Before",
        "// ###### Fixes ######",
        "~A/setup-a.tp2~ #0 #1 // One",
        "// a note ~X/SETUP-X.TP2~ #0 #5",
        "// ### Odd ~X/SETUP-X.TP2~ name ###",
        "~B\\SETUP-B.TP2~ #0 #2 // Two // @wlb-inputs: y",
    ]
    .join("\n");

    assert_eq!(
        separator_anchors_from_log_text(&text),
        vec![
            log_anchor("Fixes", vec![pair("A", "1"), pair("B", "2")]),
            log_anchor("Odd ~X/SETUP-X.TP2~ name", vec![pair("B", "2")]),
        ]
    );
    assert_eq!(
        separator_anchors_from_log_text("~A/SETUP-A.TP2~ #0 #0 // Before\n"),
        Vec::new()
    );

    let long_name = "L".repeat(75);
    let unclean = format!(
        "// ###### Ask @wlb-inputs: y ######\n// ###### {long_name} ######\n// ###### @@ ######\n~A/SETUP-A.TP2~ #0 #1 // One"
    );
    assert_eq!(
        separator_anchors_from_log_text(&unclean),
        vec![
            log_anchor("Ask wlb-inputs: y", vec![pair("A", "1")]),
            log_anchor(&"L".repeat(60), vec![pair("A", "1")]),
        ]
    );
}

#[test]
fn preview_counts_ignore_separator_lines() {
    let payload: ModlistSharePayload = serde_json::from_value(json!({
        "format_version": 1,
        "game_install": "BGEE",
        "install_mode": "start_from_scratch",
        "weidu_logs": {
            "bgee": "// ###### Fixes ######\n~A/SETUP-A.TP2~ #0 #0 // A\n// ###### NPCs ######\n~B/SETUP-B.TP2~ #0 #1 // B\n"
        }
    }))
    .expect("payload parses");

    assert_eq!(share_preview(&payload).expect("preview").bgee_entries, 2);
}

fn scanned_mod(mod_name: &str, component_id: &str, raw_line: &str) -> Step2ModState {
    Step2ModState {
        name: mod_name.to_string(),
        tp_file: format!("SETUP-{mod_name}.TP2"),
        tp2_path: format!("{mod_name}/SETUP-{mod_name}.TP2"),
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
        components: vec![Step2ComponentState {
            component_id: component_id.to_string(),
            label: "Component".to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: raw_line.to_string(),
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
        }],
    }
}

#[test]
fn language_rewrite_passes_separator_lines() {
    let root = SeparatorTestRoot::new("language");
    let log_path = root.0.join("weidu.log");
    std::fs::write(
        &log_path,
        "// ###### Fixes ######\r\n~A/SETUP-A.TP2~ #0 #0 // Component\r\n// ###### NPCs ######\r\n~B/SETUP-B.TP2~ #0 #1 // Component\r\n",
    )
    .expect("imported log");
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step1.have_weidu_logs = true;
    state.step1.bgee_log_file = log_path.to_string_lossy().into_owned();
    state.step2.bgee_mods = vec![scanned_mod("A", "0", "~A/SETUP-A.TP2~ #2 #0 // Component")];

    let changed =
        crate::app::install_language_rewrite::rewrite_imported_logs(&state).expect("rewrite");

    assert_eq!(changed, 1);
    assert_eq!(
        std::fs::read_to_string(&log_path).expect("rewritten log"),
        "// ###### Fixes ######\r\n~A/SETUP-A.TP2~ #2 #0 // Component\r\n// ###### NPCs ######\r\n~B/SETUP-B.TP2~ #0 #1 // Component\r\n"
    );
}

#[test]
fn eet_path_rewrite_passes_separator_lines() {
    use crate::app::wlb_inputs::{decode_answers, encode_answers, marker_spec};
    let step1 = Step1State {
        eet_bgee_game_folder: r"D:\Games\BGEE".to_string(),
        ..Default::default()
    };
    let encoded = encode_answers(&[r"C:\Old\BGEE".to_string()])
        .expect("codec")
        .expect("marker");
    let text = format!(
        "// ###### Fixes ######\n~MOD/MOD.TP2~ #0 #17 // Label // @wlb-inputs: {encoded}\n// ###### NPCs ######\n~B/SETUP-B.TP2~ #0 #1 // Component"
    );

    let rewritten = rewrite_imported_eet_bg2ee_wlb_paths(&step1, Some(&text))
        .expect("localized")
        .expect("log");

    let lines: Vec<&str> = rewritten.lines().collect();
    assert_eq!(lines.len(), 4, "{rewritten}");
    assert_eq!(lines[0], "// ###### Fixes ######");
    assert_eq!(lines[2], "// ###### NPCs ######");
    assert_eq!(lines[3], "~B/SETUP-B.TP2~ #0 #1 // Component");
    assert_eq!(
        decode_answers(marker_spec(lines[1]).expect("marker")).expect("codec"),
        [r"D:\Games\BGEE"]
    );
}
