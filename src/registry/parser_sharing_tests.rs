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
