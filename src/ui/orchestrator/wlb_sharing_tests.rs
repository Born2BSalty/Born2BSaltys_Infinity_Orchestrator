use crate::app::modlist_share::{decode_share_payload, encode_share_payload_text};
use crate::registry::model::{Game, ModlistEntry};
use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;

#[test]
fn manual_wlb_cached_share_uses_requested_workspace_and_refreshes_both_caches() {
    let mut app = OrchestratorApp::new_isolated_for_test("manual-wlb-cached-share");
    let id = "MANUALWLB001";
    let original = encode_share_payload_text(r#"{"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: stale"},"parser_choice_phrases":["Embedded"]}"#).expect("fixture");
    app.registry.entries.push(ModlistEntry {
        id: id.to_string(),
        game: Game::BGEE,
        latest_share_code: Some(original.clone()),
        ..Default::default()
    });
    app.workspace_state.insert(
        id.to_string(),
        ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "MOD/MOD.TP2".to_string(),
                id: 17,
                language: 2,
                wlb_inputs: Some("bio-json-v1:[\"\",\"a,b\"]".to_string()),
            }],
            parser_choice_phrases: Some(vec!["Workspace".to_string()]),
            last_share_code: Some(original),
            ..Default::default()
        },
    );
    app.wizard_state.parser_choice_phrases = vec!["Unrelated wizard".to_string()];
    let code = app
        .code_for_share_with_phrases(id, &["Local".to_string()])
        .expect("share")
        .expect("code");
    let payload = decode_share_payload(&code).expect("decode");
    assert_eq!(
        payload.parser_choice_phrases,
        ["Embedded", "Workspace", "Local"]
    );
    let text = payload.weidu_logs.bgee.expect("first-slot log");
    assert!(text.contains("bio-json-v1:"));
    assert!(!text.contains("stale"));
    assert_eq!(
        app.registry
            .find(id)
            .expect("entry")
            .latest_share_code
            .as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        app.workspace_state
            .get(id)
            .expect("workspace")
            .last_share_code
            .as_deref(),
        Some(code.as_str())
    );
}

#[test]
fn manual_wlb_active_workspace_share_extracts_current_dirty_answers() {
    let mut app = OrchestratorApp::new_isolated_for_test("manual-wlb-active-share");
    let id = "MANUALWLB001";
    let original = encode_share_payload_text(r#"{"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: stale"},"parser_choice_phrases":["Embedded"]}"#).expect("fixture");
    app.registry.entries.push(ModlistEntry {
        id: id.to_string(),
        game: Game::BGEE,
        latest_share_code: Some(original.clone()),
        ..Default::default()
    });
    app.workspace_state.insert(
        id.to_string(),
        ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "MOD/MOD.TP2".to_string(),
                id: 17,
                language: 2,
                wlb_inputs: Some("cached".to_string()),
            }],
            parser_choice_phrases: Some(vec!["Workspace".to_string()]),
            last_share_code: Some(original),
            ..Default::default()
        },
    );
    app.wizard_state.parser_choice_phrases = vec!["Unrelated wizard".to_string()];
    app.workspace_view.loaded_workspace_id = Some(id.to_string());
    app.wizard_state.step1.game_install = "BGEE".to_string();
    app.wizard_state.step1.install_mode =
        crate::app::state::Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
    app.wizard_state.step3.bgee_items = vec![crate::app::state::Step3ItemState {
        tp_file: "MOD/MOD.TP2".to_string(),
        component_id: "17".to_string(),
        mod_name: "Mod".to_string(),
        component_label: "Label".to_string(),
        raw_line: "~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: bio-json-v1:[\"a,b\",\"\"]"
            .to_string(),
        prompt_summary: None,
        prompt_events: vec![],
        selected_order: 1,
        block_id: "MOD".to_string(),
        is_parent: false,
        parent_placeholder: false,
    }];
    let code = app
        .code_for_share_with_phrases(id, &["Local".to_string()])
        .expect("share")
        .expect("code");
    let payload = decode_share_payload(&code).expect("decode");
    assert_eq!(
        payload.parser_choice_phrases,
        ["Embedded", "Unrelated wizard", "Local"]
    );
    let text = payload.weidu_logs.bgee.expect("first-slot log");
    assert!(text.contains("bio-json-v1:"));
    assert!(!text.contains("stale"));
    assert!(!text.contains("cached"));
    assert_eq!(
        app.registry
            .find(id)
            .expect("entry")
            .latest_share_code
            .as_deref(),
        Some(code.as_str())
    );
    assert_eq!(
        app.workspace_state
            .get(id)
            .expect("workspace")
            .last_share_code
            .as_deref(),
        Some(code.as_str())
    );
}
