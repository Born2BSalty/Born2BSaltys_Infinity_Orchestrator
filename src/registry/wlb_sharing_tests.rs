use crate::app::modlist_share::{base64url_decode, encode_share_payload_text};
use crate::registry::share_export::overlay_workspace_wlb;
use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use std::io::Read;

fn payload(code: &str) -> serde_json::Value {
    let bytes =
        base64url_decode(code.strip_prefix("BIO-MODLIST-V1:").expect("prefix")).expect("base64");
    let mut text = String::new();
    flate2::read::ZlibDecoder::new(bytes.as_slice())
        .read_to_string(&mut text)
        .expect("inflate");
    serde_json::from_str(&text).expect("JSON")
}

fn code() -> String {
    encode_share_payload_text(&serde_json::json!({"format_version":1,"game_install":"EET","install_mode":"start_from_scratch","weidu_logs":{"bgee":"~MOD/MOD.TP2~ #2 #17 // First // @wlb-inputs: old\n~OTHER/OTHER.TP2~ #0 #1 // Untouched", "bg2ee":"~MOD/MOD.TP2~ #2 #17 // Second // @wlb-inputs: keep"},"parser_choice_phrases":["Embedded custom"],"future_key":{"opaque":[42,"世界"]},"archive_meta":[{"name":"mod.zip","size":99,"hash":"abcd"}],"allow_auto_install":false,"forked_from":[{"name":"Root","author":"@author"}]}).to_string()).expect("code")
}

fn workspace() -> ModlistWorkspaceState {
    ModlistWorkspaceState {
        order_bgee: vec![ComponentRef {
            tp2: "mod/mod.tp2".to_string(),
            id: 17,
            language: 2,
            wlb_inputs: Some("bio-json-v1:[\"a,b\",\"\"]".to_string()),
        }],
        ..Default::default()
    }
}

#[test]
fn manual_wlb_overlay_changes_only_correct_slot_and_preserves_all_metadata() {
    let original = code();
    let result = overlay_workspace_wlb(&original, &workspace()).expect("overlay");
    let mut expected = payload(&original);
    expected["weidu_logs"]["bgee"] = serde_json::json!(
        "~MOD/MOD.TP2~ #2 #17 // First // @wlb-inputs: bio-json-v1:[\"a,b\",\"\"]\n~OTHER/OTHER.TP2~ #0 #1 // Untouched"
    );
    assert_eq!(payload(&result), expected);
    assert_eq!(
        overlay_workspace_wlb(&result, &workspace()).expect("idempotent"),
        result
    );
    let unchanged = overlay_workspace_wlb(&original, &ModlistWorkspaceState::default())
        .expect("None is not delete");
    assert_eq!(payload(&unchanged), payload(&original));
}

#[test]
fn manual_wlb_overlay_missing_language_duplicate_workspace_or_target_is_error() {
    let original = code();
    let mut wrong_language = workspace();
    wrong_language.order_bgee[0].language = 0;
    assert!(overlay_workspace_wlb(&original, &wrong_language).is_err());
    let mut duplicate = workspace();
    duplicate.order_bgee.push(duplicate.order_bgee[0].clone());
    assert!(overlay_workspace_wlb(&original, &duplicate).is_err());
    let mut raw = payload(&original);
    let line = raw["weidu_logs"]["bgee"].as_str().expect("log").to_string();
    raw["weidu_logs"]["bgee"] = serde_json::json!(format!("{line}\n{line}"));
    let duplicate_target = encode_share_payload_text(&raw.to_string()).expect("duplicate fixture");
    assert!(overlay_workspace_wlb(&duplicate_target, &workspace()).is_err());
}
