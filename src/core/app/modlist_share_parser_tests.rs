use super::*;

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
