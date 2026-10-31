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

fn answered(tp2: &str, id: i64, language: u8, answers: &str) -> ComponentRef {
    ComponentRef {
        tp2: tp2.to_string(),
        id,
        language,
        wlb_inputs: Some(answers.to_string()),
    }
}

fn code_with_bgee_log(log: &str) -> String {
    encode_share_payload_text(
        &serde_json::json!({"format_version":1,"game_install":"BGEE","install_mode":"start_from_scratch","weidu_logs":{"bgee":log}})
            .to_string(),
    )
    .expect("code")
}

fn bgee_log(code: &str) -> String {
    payload(code)["weidu_logs"]["bgee"]
        .as_str()
        .expect("bgee log")
        .to_string()
}

#[test]
fn manual_wlb_overlay_duplicate_workspace_item_first_wins() {
    let mut duplicate = workspace();
    duplicate
        .order_bgee
        .push(answered("MOD/MOD.TP2", 17, 2, "second"));
    let result = overlay_workspace_wlb(&code(), &duplicate).expect("duplicates are not an error");
    let log = bgee_log(&result);
    assert!(log.contains("@wlb-inputs: bio-json-v1:[\"a,b\",\"\"]"));
    assert!(!log.contains("second"));
}

#[test]
fn overlay_first_answered_duplicate_wins_over_an_unanswered_one() {
    let original = code_with_bgee_log("~MOD/MOD.TP2~ #0 #17 // Label\n");
    let mut unanswered = answered("MOD/MOD.TP2", 17, 0, "ignored");
    unanswered.wlb_inputs = None;
    let workspace = ModlistWorkspaceState {
        order_bgee: vec![unanswered, answered("MOD/MOD.TP2", 17, 0, "late")],
        ..Default::default()
    };
    let result = overlay_workspace_wlb(&original, &workspace).expect("overlay");
    assert_eq!(
        bgee_log(&result),
        "~MOD/MOD.TP2~ #0 #17 // Label // @wlb-inputs: late\n"
    );
}

#[test]
fn overlay_matches_rows_across_languages() {
    let original = code_with_bgee_log("~MOD/MOD.TP2~ #0 #17 // Label\n");
    let workspace = ModlistWorkspaceState {
        order_bgee: vec![answered("mod/mod.tp2", 17, 2, "y,n")],
        ..Default::default()
    };
    let result = overlay_workspace_wlb(&original, &workspace).expect("overlay");
    assert_eq!(
        bgee_log(&result),
        "~MOD/MOD.TP2~ #0 #17 // Label // @wlb-inputs: y,n\n"
    );
}

#[test]
fn overlay_skips_component_missing_from_stale_code() {
    let original = code_with_bgee_log("~MOD/MOD.TP2~ #0 #17 // Label\n");
    let workspace = ModlistWorkspaceState {
        order_bgee: vec![
            answered("NEW/NEW.TP2", 5, 0, "added"),
            answered("MOD/MOD.TP2", 17, 0, "y"),
        ],
        ..Default::default()
    };
    let result = overlay_workspace_wlb(&original, &workspace).expect("stale code is not an error");
    assert_eq!(
        bgee_log(&result),
        "~MOD/MOD.TP2~ #0 #17 // Label // @wlb-inputs: y\n"
    );
}

#[test]
fn overlay_skips_ambiguous_rows() {
    let log = "~MOD/MOD.TP2~ #0 #17 // Label\n~MOD/MOD.TP2~ #1 #17 // Label\n~OTHER/OTHER.TP2~ #0 #1 // Other\n";
    let original = code_with_bgee_log(log);
    let workspace = ModlistWorkspaceState {
        order_bgee: vec![
            answered("MOD/MOD.TP2", 17, 0, "ambiguous"),
            answered("OTHER/OTHER.TP2", 1, 0, "other"),
        ],
        ..Default::default()
    };
    let result = overlay_workspace_wlb(&original, &workspace).expect("ambiguity is not an error");
    assert_eq!(
        bgee_log(&result),
        "~MOD/MOD.TP2~ #0 #17 // Label\n~MOD/MOD.TP2~ #1 #17 // Label\n~OTHER/OTHER.TP2~ #0 #1 // Other // @wlb-inputs: other\n"
    );
}

#[test]
fn overlay_skips_missing_slot_and_invalid_answers() {
    let original = code_with_bgee_log("~MOD/MOD.TP2~ #0 #17 // Label\n");
    let workspace = ModlistWorkspaceState {
        order_bgee: vec![answered("MOD/MOD.TP2", 17, 0, "bio-json-v9:[]")],
        order_bg2ee: vec![answered("MOD/MOD.TP2", 17, 0, "y")],
        ..Default::default()
    };
    assert_eq!(
        overlay_workspace_wlb(&original, &workspace).expect("skips are not errors"),
        original
    );
}

#[test]
fn overlay_errs_only_on_an_undecodable_code() {
    assert!(overlay_workspace_wlb("BIO-MODLIST-V1:!!!", &workspace()).is_err());
}
