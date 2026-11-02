use super::*;

struct RecognitionTransportFixture(std::path::PathBuf);

impl RecognitionTransportFixture {
    fn new() -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "recognition-transport-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        Self(root)
    }
}

impl Drop for RecognitionTransportFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn combination_red_public_import_export_retains_finished_recognition() {
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
        "recognition-share",
    );
    let value = json!({
        "format_version": 1,
        "game_install": "BGEE",
        "install_mode": "start_from_scratch",
        "weidu_logs": {"bgee": "~MOD/MOD.TP2~ #0 #0 // Component: 1.0\n"},
        "parser_recognition_rules": {"finished": ["Fertig 世界"]}
    });
    let code = encode_share_payload_text(&value.to_string()).unwrap();
    let mut state = WizardState::default();
    let fixture = RecognitionTransportFixture::new();
    state.step1.bgee_log_folder = fixture.0.to_string_lossy().into_owned();
    import_modlist_share_code(&mut state, &code).unwrap();
    state
        .step3
        .bgee_items
        .push(crate::app::state::Step3ItemState {
            tp_file: "MOD/MOD.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "Mod".to_string(),
            component_label: "Component".to_string(),
            raw_line: "~MOD/MOD.TP2~ #0 #0 // Component: 1.0".to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: "mod".to_string(),
            is_parent: false,
            parent_placeholder: false,
        });
    let exported = export_modlist_share_code_with(&state, &ShareExportSources::default()).unwrap();
    let bytes = base64url_decode(exported.strip_prefix(SHARE_CODE_PREFIX).unwrap()).unwrap();
    let exported: serde_json::Value =
        serde_json::from_slice(&zlib_decompress(&bytes).unwrap()).unwrap();
    assert_eq!(
        exported["parser_recognition_rules"]["finished"],
        value["parser_recognition_rules"]["finished"]
    );
    drop(app);
}

#[test]
fn combination_all_seven_arrays_public_import_store_reopen_and_actual_preparation() {
    use crate::app::mod_installer_parser::{ParserInitializer, VersionOutput};
    struct Initialize(std::path::PathBuf);
    impl ParserInitializer for Initialize {
        fn run_version(&self, _executable: &std::path::Path) -> Result<VersionOutput, String> {
            std::fs::write(&self.0, "choice_phrase = ['Default']\nfinished = ['Default finished']\nbell_character = '!'\n[metadata]\nmod_installer_version = '15.0.1'\n[custom]\nkept = 17\n").map_err(|error| error.to_string())?;
            Ok(VersionOutput {
                display: "fixture 15.0.1".to_string(),
                version: "15.0.1".to_string(),
            })
        }
    }
    let _lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let fixture = RecognitionTransportFixture::new();
    let app = crate::ui::orchestrator::orchestrator_app::OrchestratorApp::new_isolated_for_test(
        "full-recognition-transport",
    );
    let expected = parser_rules_transport_map();
    let code = parser_rules_transport_code(json!(expected), json!([]));
    let mut wizard = WizardState::default();
    wizard.step1.bgee_log_folder = fixture.0.to_string_lossy().into_owned();
    import_modlist_share_code(&mut wizard, &code).unwrap();
    let workspace =
        crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
            &wizard,
            &crate::registry::workspace_model::ModlistWorkspaceState::default(),
        );
    let store = crate::registry::store_workspace::WorkspaceStore::new_with_path(
        fixture.0.join("selected-workspace.json"),
    );
    store.save(&workspace).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.parser_recognition_rules.as_ref(), Some(&expected));
    let entry = crate::registry::model::ModlistEntry {
        id: "SELECTED".to_string(),
        ..Default::default()
    };
    let mut reopened = WizardState::default();
    let _ = crate::ui::workspace::workspace_state_loader::populate_wizard_state_from_workspace(
        &loaded,
        &entry,
        &app.settings_store,
        &mut reopened,
    );
    assert_eq!(reopened.parser_recognition_rules, expected);
    let parser = fixture.0.join("parser.toml");
    let prepared = crate::app::mod_installer_parser::prepare_parser_with_rules_at(
        &parser,
        &fixture.0.join("never-executed.exe"),
        &reopened.parser_recognition_rules,
        &Initialize(parser.clone()),
    )
    .unwrap();
    let merged: toml::Value = toml::from_str(&std::fs::read_to_string(&parser).unwrap()).unwrap();
    for (key, values) in &expected {
        for value in values {
            assert!(
                merged[key]
                    .as_array()
                    .unwrap()
                    .contains(&toml::Value::String(value.clone())),
                "{key}: {value}"
            );
        }
    }
    assert_eq!(merged["bell_character"].as_str(), Some("!"));
    assert_eq!(merged["custom"]["kept"].as_integer(), Some(17));
    drop(prepared);
}

fn parser_sharing_code(phrases: Option<serde_json::Value>) -> String {
    let mut value = json!({"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #0 #0 // Component: 1.0\n"}});
    if let Some(phrases) = phrases {
        value["parser_choice_phrases"] = phrases;
    }
    encode_share_payload_text(&value.to_string()).expect("fixture code")
}

#[test]
fn parser_sharing_payload_and_preview_preserve_unicode_and_spaces() {
    let phrases = vec!["Default".to_string(), " Custom 世界 ".to_string()];
    let code = parser_sharing_code(Some(json!(phrases)));
    assert_eq!(
        decode_share_payload(&code)
            .expect("payload")
            .parser_choice_phrases,
        phrases
    );
    assert_eq!(
        preview_modlist_share_code(&code)
            .expect("preview")
            .parser_choice_phrases,
        phrases
    );
    let legacy = parser_sharing_code(None);
    assert_eq!(
        decode_share_payload(&legacy)
            .expect("legacy payload")
            .parser_choice_phrases,
        [] as [String; 0]
    );
    assert_eq!(
        preview_modlist_share_code(&legacy)
            .expect("legacy preview")
            .parser_choice_phrases,
        [] as [String; 0]
    );
}

#[test]
fn parser_sharing_decode_rejects_invalid_metadata_before_import_mutation() {
    for invalid in [
        json!(null),
        json!("phrase"),
        json!([1]),
        json!([" "]),
        json!(["x".repeat(8193)]),
        json!(
            (0..4097)
                .map(|index| format!("p{index}"))
                .collect::<Vec<_>>()
        ),
    ] {
        let code = parser_sharing_code(Some(invalid));
        assert!(decode_share_payload(&code).is_err());
        let mut state = WizardState {
            parser_choice_phrases: vec!["Keep".to_string()],
            ..Default::default()
        };
        assert!(import_modlist_share_code(&mut state, &code).is_err());
        assert_eq!(state.parser_choice_phrases, ["Keep"]);
    }
}

#[test]
fn parser_sharing_injected_export_sources_union_wizard_before_local() {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step1.install_mode = "start_from_scratch".to_string();
    state
        .step3
        .bgee_items
        .push(crate::app::state::Step3ItemState {
            tp_file: "MOD/MOD.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "Mod".to_string(),
            component_label: "Component".to_string(),
            raw_line: "~MOD/MOD.TP2~ #0 #0 // Component: 1.0".to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: "mod".to_string(),
            is_parent: false,
            parent_placeholder: false,
        });
    state.parser_choice_phrases = vec!["Imported".to_string(), "Shared".to_string()];
    let sources = ShareExportSources {
        parser_choice_phrases: vec![
            "Shared".to_string(),
            "Default".to_string(),
            " Custom 世界 ".to_string(),
        ],
        ..Default::default()
    };
    let code = export_modlist_share_code_with(&state, &sources).expect("pure export");
    assert_eq!(
        decode_share_payload(&code)
            .expect("decode")
            .parser_choice_phrases,
        ["Imported", "Shared", "Default", " Custom 世界 "]
    );
    assert_eq!(
        ShareExportSources::default().parser_choice_phrases,
        [] as [String; 0]
    );
    state.reset_workflow_keep_step1();
    assert_eq!(state.parser_choice_phrases, [] as [String; 0]);
}

fn parser_rules_transport_map() -> crate::app::mod_installer_parser::ParserRecognitionRules {
    [
        ("in_progress_words", vec!["installing", "installiere"]),
        ("useful_status_words", vec!["copied", "copiado"]),
        ("choice_words", vec!["choose", "auswählen", "elige"]),
        (
            "choice_phrase",
            vec!["would you like", "möchten Sie", "¿desea continuar?"],
        ),
        ("completed_with_warnings", vec!["advertencia"]),
        ("failed_with_error", vec!["fehlgeschlagen"]),
        ("finished", vec!["instalado"]),
    ]
    .into_iter()
    .map(|(key, values)| {
        (
            key.to_string(),
            values.into_iter().map(str::to_string).collect(),
        )
    })
    .collect()
}

fn parser_rules_transport_code(rules: serde_json::Value, legacy: serde_json::Value) -> String {
    let mut payload = json!({"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #0 #0 // Component: 1.0\n"},"future_key":{"keep":"世界"}});
    payload["parser_recognition_rules"] = rules;
    payload["parser_choice_phrases"] = legacy;
    encode_share_payload_text(&payload.to_string()).unwrap()
}

#[test]
fn parser_rules_transport_preserves_seven_fields_preview_import_and_legacy_mirror() {
    let rules = parser_rules_transport_map();
    let code = parser_rules_transport_code(json!(rules), json!(["legacy phrase"]));
    let expected = crate::app::mod_installer_parser::recognition_rules_with_legacy(
        &rules,
        &["legacy phrase".to_string()],
    )
    .unwrap();
    let payload = decode_share_payload(&code).unwrap();
    let preview = preview_modlist_share_code(&code).unwrap();
    assert_eq!(
        crate::app::mod_installer_parser::recognition_rules_with_legacy(
            &payload.parser_recognition_rules,
            &payload.parser_choice_phrases
        )
        .unwrap(),
        expected
    );
    assert_eq!(preview.parser_recognition_rules, expected);
    assert_eq!(preview.parser_choice_phrases, expected["choice_phrase"]);
    let root = std::env::temp_dir().join(format!("bio-parser-rules-import-{}", std::process::id()));
    let mut state = WizardState::default();
    state.step1.bgee_log_folder = root.to_string_lossy().into_owned();
    import_modlist_share_code(&mut state, &code).unwrap();
    assert_eq!(state.parser_recognition_rules, expected);
    assert_eq!(state.parser_choice_phrases, expected["choice_phrase"]);
    state.reset_workflow_keep_step1();
    assert!(state.parser_recognition_rules.is_empty());
    assert_eq!(state.parser_choice_phrases, [] as [String; 0]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn parser_rules_transport_rejects_invalid_map_without_overwriting_current_wizard() {
    for invalid in [
        json!(null),
        json!([]),
        json!({"choice_words":"choose"}),
        json!({"finished":[1]}),
        json!({"choice_words":[" "]}),
        json!({"bell_character":["!"]}),
        json!({"unknown":["x"]}),
    ] {
        let code = parser_rules_transport_code(invalid.clone(), json!([]));
        assert!(decode_share_payload(&code).is_err(), "{invalid}");
        let mut state = WizardState {
            parser_recognition_rules: parser_rules_transport_map(),
            ..Default::default()
        };
        let before = state.parser_recognition_rules.clone();
        assert!(import_modlist_share_code(&mut state, &code).is_err());
        assert_eq!(state.parser_recognition_rules, before);
    }
    let oversized =
        parser_rules_transport_code(json!({"choice_words":["x".repeat(8193)]}), json!([]));
    assert!(decode_share_payload(&oversized).is_err());
}

#[test]
fn parser_rules_export_unions_wizard_and_local_by_field_without_dropping_legacy_phrase() {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step1.install_mode = "start_from_scratch".to_string();
    state
        .step3
        .bgee_items
        .push(crate::app::state::Step3ItemState {
            tp_file: "MOD/MOD.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "Mod".to_string(),
            component_label: "Component".to_string(),
            raw_line: "~MOD/MOD.TP2~ #0 #0 // Component: 1.0".to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: "mod".to_string(),
            is_parent: false,
            parent_placeholder: false,
        });
    state.parser_recognition_rules = parser_rules_transport_map();
    state.parser_choice_phrases = vec!["legacy wizard".to_string()];
    let sources = ShareExportSources {
        parser_recognition_rules: [
            (
                "choice_words".to_string(),
                vec!["elige".to_string(), "local word".to_string()],
            ),
            ("finished".to_string(), vec!["local complete".to_string()]),
        ]
        .into_iter()
        .collect(),
        parser_choice_phrases: vec!["legacy local".to_string()],
        ..Default::default()
    };
    let code = export_modlist_share_code_with(&state, &sources).unwrap();
    let payload = decode_share_payload(&code).unwrap();
    assert_eq!(
        payload.parser_recognition_rules["choice_words"],
        ["choose", "auswählen", "elige", "local word"]
    );
    assert_eq!(
        payload.parser_recognition_rules["finished"],
        ["instalado", "local complete"]
    );
    assert_eq!(
        payload.parser_recognition_rules["choice_phrase"],
        [
            "would you like",
            "möchten Sie",
            "¿desea continuar?",
            "legacy wizard",
            "legacy local"
        ]
    );
    assert_eq!(
        payload.parser_choice_phrases,
        payload.parser_recognition_rules["choice_phrase"]
    );
}

#[test]
fn parser_rules_legacy_phrase_only_payload_is_promoted_without_inventing_other_fields() {
    let code = parser_sharing_code(Some(json!(["Only legacy", " 世界 "])));
    let preview = preview_modlist_share_code(&code).unwrap();
    assert_eq!(preview.parser_recognition_rules.len(), 1);
    assert_eq!(
        preview.parser_recognition_rules["choice_phrase"],
        ["Only legacy", " 世界 "]
    );
    let no_rules = parser_sharing_code(None);
    assert!(
        preview_modlist_share_code(&no_rules)
            .unwrap()
            .parser_recognition_rules
            .is_empty()
    );
}

#[test]
fn parser_rules_fresh_export_keeps_all_seven_local_arrays_when_wizard_has_no_imported_rules() {
    let mut state = WizardState::default();
    state.step1.game_install = "BGEE".to_string();
    state.step1.install_mode = "start_from_scratch".to_string();
    state
        .step3
        .bgee_items
        .push(crate::app::state::Step3ItemState {
            tp_file: "MOD/MOD.TP2".to_string(),
            component_id: "0".to_string(),
            mod_name: "Mod".to_string(),
            component_label: "Component".to_string(),
            raw_line: "~MOD/MOD.TP2~ #0 #0 // Component: 1.0".to_string(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            selected_order: 0,
            block_id: "mod".to_string(),
            is_parent: false,
            parent_placeholder: false,
        });
    let sources = ShareExportSources {
        parser_recognition_rules: parser_rules_transport_map(),
        ..Default::default()
    };
    let code = export_modlist_share_code_with(&state, &sources).unwrap();
    let payload = decode_share_payload(&code).unwrap();
    assert_eq!(
        payload.parser_recognition_rules,
        sources.parser_recognition_rules
    );
    assert_eq!(
        payload.parser_choice_phrases,
        sources.parser_recognition_rules["choice_phrase"]
    );
    assert!(state.parser_recognition_rules.is_empty());
}
