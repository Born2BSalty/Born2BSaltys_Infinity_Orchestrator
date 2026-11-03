use super::*;
use crate::app::modlist_share::{decode_share_payload, encode_share_payload_text};
use crate::registry::workspace_model::ModlistWorkspaceState;

fn fixture_code(phrases: &serde_json::Value) -> String {
    encode_share_payload_text(&serde_json::json!({"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #0 #0 // Component: 1.0\n"},"parser_choice_phrases":phrases,"future_key":{"keep":[true,42,"世界"]}}).to_string()).expect("fixture code")
}

fn opaque_payload(code: &str) -> Value {
    let compressed =
        base64url_decode(code.strip_prefix(SHARE_CODE_PREFIX).expect("prefix")).expect("base64");
    serde_json::from_slice(&zlib_decompress(&compressed).expect("decompress")).expect("JSON")
}

#[test]
fn parser_sharing_enrichment_preserves_unknown_json_and_exact_union() {
    let original = fixture_code(&serde_json::json!(["Embedded", "Shared"]));
    let enriched = enrich_share_code_with_phrases(
        &original,
        &["Shared".to_string(), " Local 世界 ".to_string()],
    )
    .expect("enrich");
    let mut expected = opaque_payload(&original);
    expected["parser_choice_phrases"] = serde_json::json!(["Embedded", "Shared", " Local 世界 "]);
    assert_eq!(opaque_payload(&enriched), expected);
    let permitted = set_allow_auto_install(&enriched, false).expect("permission rewrite");
    let named = set_packed_name(&permitted, "Renamed").expect("name rewrite");
    assert_eq!(opaque_payload(&named)["future_key"], expected["future_key"]);
    assert_eq!(
        decode_share_payload(&named)
            .expect("decode")
            .parser_choice_phrases,
        ["Embedded", "Shared", " Local 世界 "]
    );
    for invalid in [
        serde_json::json!(null),
        serde_json::json!([" "]),
        serde_json::json!([7]),
    ] {
        assert!(enrich_share_code_with_phrases(&fixture_code(&invalid), &[]).is_err());
    }
}

#[test]
fn parser_sharing_workspace_distinguishes_legacy_from_explicit_clear() {
    let legacy: ModlistWorkspaceState = serde_json::from_str("{}").expect("legacy");
    assert_eq!(legacy.parser_choice_phrases, None);
    assert!(legacy.is_empty());
    assert!(
        serde_json::to_value(&legacy)
            .expect("serialize")
            .get("parser_choice_phrases")
            .is_none()
    );
    let cleared = ModlistWorkspaceState {
        parser_choice_phrases: Some(Vec::new()),
        ..Default::default()
    };
    assert!(cleared.is_empty());
    assert_eq!(
        serde_json::to_value(&cleared).expect("serialize")["parser_choice_phrases"],
        serde_json::json!([])
    );
    let populated = ModlistWorkspaceState {
        parser_choice_phrases: Some(vec!["Imported".to_string()]),
        ..Default::default()
    };
    assert!(!populated.is_empty());
    assert_eq!(
        serde_json::from_str::<ModlistWorkspaceState>(
            &serde_json::to_string(&populated).expect("serialize")
        )
        .expect("reopen"),
        populated
    );
}

fn parser_rules_registry_map() -> crate::app::mod_installer_parser::ParserRecognitionRules {
    [
        (
            "choice_words".to_string(),
            vec!["auswählen".to_string(), "elige".to_string()],
        ),
        (
            "choice_phrase".to_string(),
            vec!["embedded phrase".to_string()],
        ),
        ("finished".to_string(), vec!["ist fertig".to_string()]),
    ]
    .into_iter()
    .collect()
}

#[test]
fn parser_rules_enrichment_and_rewrites_keep_opaque_payload_and_per_field_union() {
    let mut payload = opaque_payload(&fixture_code(&serde_json::json!(["legacy embedded"])));
    payload["parser_recognition_rules"] = serde_json::json!(parser_rules_registry_map());
    let original = encode_share_payload_text(&payload.to_string()).unwrap();
    let additions = [
        (
            "choice_words".to_string(),
            vec!["elige".to_string(), "choose".to_string()],
        ),
        (
            "failed_with_error".to_string(),
            vec!["instalación falló".to_string()],
        ),
    ]
    .into_iter()
    .collect();
    let enriched = enrich_share_code_with_recognition_rules(&original, &additions).unwrap();
    let renamed = set_packed_name(
        &set_allow_auto_install(&enriched, false).unwrap(),
        "Rules list",
    )
    .unwrap();
    assert_eq!(
        opaque_payload(&renamed)["future_key"],
        payload["future_key"]
    );
    let decoded = decode_share_payload(&renamed).unwrap();
    assert_eq!(
        decoded.parser_recognition_rules["choice_words"],
        ["auswählen", "elige", "choose"]
    );
    assert_eq!(
        decoded.parser_recognition_rules["failed_with_error"],
        ["instalación falló"]
    );
    assert_eq!(
        decoded.parser_choice_phrases,
        decoded.parser_recognition_rules["choice_phrase"]
    );
    assert!(!decoded.allow_auto_install);
}

#[test]
fn parser_rules_workspace_presence_and_legacy_override_have_one_authority() {
    let mut payload = opaque_payload(&fixture_code(&serde_json::json!(["old phrase"])));
    payload["parser_recognition_rules"] = serde_json::json!(parser_rules_registry_map());
    let code = encode_share_payload_text(&payload.to_string()).unwrap();
    let absent: ModlistWorkspaceState = serde_json::from_str("{}").unwrap();
    let inherited = workspace_recognition_rules(&code, &absent).unwrap();
    assert_eq!(inherited["choice_words"], ["auswählen", "elige"]);
    let old_clear = ModlistWorkspaceState {
        parser_choice_phrases: Some(Vec::new()),
        ..Default::default()
    };
    let clear_phrase = workspace_recognition_rules(&code, &old_clear).unwrap();
    assert_eq!(clear_phrase["choice_words"], inherited["choice_words"]);
    assert!(clear_phrase.get("choice_phrase").is_none_or(Vec::is_empty));
    let new_clear = ModlistWorkspaceState {
        parser_recognition_rules: Some(
            crate::app::mod_installer_parser::ParserRecognitionRules::new(),
        ),
        parser_choice_phrases: Some(vec!["stale mirror".to_string()]),
        ..Default::default()
    };
    assert!(
        workspace_recognition_rules(&code, &new_clear)
            .unwrap()
            .is_empty()
    );
    let explicit = ModlistWorkspaceState {
        parser_recognition_rules: Some(
            std::iter::once(("choice_words".to_string(), vec!["explicit".to_string()])).collect(),
        ),
        parser_choice_phrases: Some(vec!["stale mirror".to_string()]),
        ..Default::default()
    };
    let chosen = workspace_recognition_rules(&code, &explicit).unwrap();
    assert_eq!(chosen.len(), 1);
    assert_eq!(chosen["choice_words"], ["explicit"]);
    let saved = serde_json::to_string(&explicit).unwrap();
    let reopened = serde_json::from_str::<ModlistWorkspaceState>(&saved).unwrap();
    assert_eq!(
        workspace_recognition_rules(&code, &reopened).unwrap(),
        chosen
    );
    assert!(!reopened.is_empty());
}
