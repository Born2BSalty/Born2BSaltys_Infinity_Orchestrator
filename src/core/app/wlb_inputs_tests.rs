use crate::app::wlb_inputs::{
    decode_answers, encode_answers, marker_spec, parse_identity, replace_marker,
};

#[test]
fn manual_wlb_codec_round_trips_lossless_single_line_sequences() {
    for answers in [
        vec!["y", "1", r"D:\Games\BG1"],
        vec![""],
        vec!["  世界  ", "a,b", ""],
        vec!["bio-json-v1:[\"x\"]"],
        vec!["bio-json-v9:literal"],
    ] {
        let answers: Vec<String> = answers.into_iter().map(str::to_string).collect();
        let spec = encode_answers(&answers).expect("encode").expect("marker");
        assert_eq!(decode_answers(&spec).expect("decode"), answers);
    }
    assert_eq!(encode_answers(&[]).expect("empty"), None);
    assert_eq!(
        encode_answers(&[String::new()]).expect("blank"),
        Some("bio-json-v1:[\"\"]".to_string())
    );
    assert_eq!(
        encode_answers(&["y".to_string(), "1".to_string()]).expect("legacy"),
        Some("y,1".to_string())
    );
    assert_eq!(
        decode_answers(" y , 1 ").expect("legacy trimming"),
        ["y", "1"]
    );
}

#[test]
fn manual_wlb_codec_rejects_invalid_tags_and_protocol_injection() {
    for spec in [
        "bio-json-v2:[\"x\"]",
        "bio-json-v1:42",
        "bio-json-v1:[1]",
        "bio-json-v1:[",
        "a\nb",
        "a\rb",
        "a\0b",
        r#"bio-json-v1:["a\nb"]"#,
    ] {
        assert!(decode_answers(spec).is_err(), "{spec:?}");
    }
    for value in ["a\nb", "a\rb", "a\0b"] {
        assert!(encode_answers(&[value.to_string()]).is_err());
    }
}

#[test]
fn manual_wlb_marker_replacement_preserves_component_text_and_identity() {
    let raw = "~mod/setup-mod.tp2~ #2 #17 // Keep label: v1 // @WLB-INPUTS: old";
    let replaced = replace_marker(raw, "bio-json-v1:[\"\"]").expect("replace");
    assert!(replaced.starts_with("~mod/setup-mod.tp2~ #2 #17 // Keep label: v1"));
    assert_eq!(
        replaced
            .to_ascii_lowercase()
            .matches("@wlb-inputs:")
            .count(),
        1
    );
    assert_eq!(marker_spec(&replaced), Some("bio-json-v1:[\"\"]"));
    let identity = parse_identity(&replaced).expect("real component");
    assert_eq!(identity.component_id, 17);
    assert_eq!(identity.language, 2);
    assert!(parse_identity("~MOD/MOD.TP2~ #0 #__PARENT__ // Group").is_none());
    assert!(parse_identity("~MOD/MOD.TP2~ #256 #1 // Bad language").is_none());
    assert!(replace_marker(raw, "bio-json-v1:[1]").is_err());
}
