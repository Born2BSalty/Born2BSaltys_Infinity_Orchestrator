// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use serde_json::json;

use crate::app::answer_sets::AnswerSets;
use crate::app::game_authority::GameSlot;
use crate::app::share_answers::{ShareAnswers, sets_label, slot_for_key};

fn sets(pairs: &[(&str, &str)]) -> AnswerSets {
    let mut sets = AnswerSets::default();
    for (language, spec) in pairs {
        sets.set(language, spec);
    }
    sets
}

#[test]
fn answers_section_round_trips_and_skips_malformed_entries() {
    let section = json!({
        "bgee": {
            "languages": { "SETUP-TWEAKS.TP2": "polish", "eefix": "english", "bad": 7 },
            "components": [
                { "tp2": "tweaks", "id": 1200, "sets": { "english": "y,1", "polish": "t,1" } },
                { "tp2": "tweaks", "id": "1201", "sets": { "english": "y" } },
                { "tp2": "tweaks", "id": 1202, "sets": { "english": 5, "polish": "t" } },
                { "tp2": "tweaks", "id": 1203, "sets": {} },
                { "tp2": "tweaks", "id": 1204, "sets": { "english": "  " } },
                { "id": 1205, "sets": { "english": "y" } },
                "not an entry",
                { "tp2": "tweaks", "id": 1200, "sets": { "english": "late" } }
            ]
        },
        "bg2ee": "not a slot",
        "psst": { "components": [ { "tp2": "x", "id": 1, "sets": { "english": "y" } } ] }
    });

    let parsed = ShareAnswers::parse(&section);

    assert_eq!(parsed.line_language("bgee", "tweaks"), "polish");
    assert_eq!(parsed.line_language("bgee", "eefix"), "english");
    assert_eq!(parsed.line_language("bg2ee", "tweaks"), "english");
    assert_eq!(
        parsed.component("bgee", "SETUP-TWEAKS.TP2", 1200),
        Some(&sets(&[("english", "y,1"), ("polish", "t,1")]))
    );
    assert_eq!(
        parsed.component("bgee", "tweaks", 1202),
        Some(&sets(&[("polish", "t")]))
    );
    for id in [1201, 1203, 1204, 1205] {
        assert_eq!(parsed.component("bgee", "tweaks", id), None, "id {id}");
    }
    assert_eq!(parsed.slots().count(), 1);

    let written = parsed.to_value().expect("section");
    assert_eq!(
        written,
        json!({
            "bgee": {
                "languages": { "tweaks": "polish" },
                "components": [
                    { "tp2": "tweaks", "id": 1200, "sets": { "english": "y,1", "polish": "t,1" } },
                    { "tp2": "tweaks", "id": 1202, "sets": { "polish": "t" } }
                ]
            }
        })
    );
    assert_eq!(ShareAnswers::parse(&written), parsed);
    assert_eq!(
        ShareAnswers::parse(&json!("garbage")),
        ShareAnswers::default()
    );
    assert_eq!(ShareAnswers::default().to_value(), None);
}

#[test]
fn empty_slots_and_lists_are_omitted() {
    let mut answers = ShareAnswers::default();
    answers.set_line_language("bgee", "tweaks", "english");
    answers.set_component("bg2ee", "tweaks", 1, AnswerSets::default());
    assert_eq!(answers.to_value(), None);

    answers.set_component("bg2ee", "B/SETUP-B.TP2", 3, sets(&[("english", "y")]));
    assert_eq!(
        answers.to_value(),
        Some(json!({
            "bg2ee": { "components": [ { "tp2": "b", "id": 3, "sets": { "english": "y" } } ] }
        }))
    );
}

#[test]
fn slot_keys_map_to_game_slots() {
    assert_eq!(slot_for_key("bgee"), Some(GameSlot::First));
    assert_eq!(slot_for_key("iwdee"), Some(GameSlot::First));
    assert_eq!(slot_for_key("bg2ee"), Some(GameSlot::Second));
    assert_eq!(slot_for_key("eet"), None);
}

#[test]
fn sets_label_lists_each_language_in_map_order() {
    assert_eq!(
        sets_label(&sets(&[("polish", "t,1"), ("english", "y,1")])),
        "English: y,1 · Polish: t,1"
    );
    assert_eq!(sets_label(&AnswerSets::default()), "");
}

#[derive(serde::Deserialize)]
struct OldReaderPayload {
    format_version: u64,
    #[serde(default)]
    weidu_logs: OldReaderLogs,
}

#[derive(Default, serde::Deserialize)]
struct OldReaderLogs {
    bgee: Option<String>,
}

fn decoded_json(code: &str) -> Vec<u8> {
    use std::io::Read;
    let bytes = crate::app::modlist_share::base64url_decode(
        code.strip_prefix("BIO-MODLIST-V1:").expect("prefix"),
    )
    .expect("base64");
    let mut json = Vec::new();
    flate2::read::ZlibDecoder::new(bytes.as_slice())
        .read_to_end(&mut json)
        .expect("inflate");
    json
}

#[test]
fn old_reader_ignores_answers_section() {
    let base = crate::app::modlist_share::encode_share_payload_text(
        &json!({
            "format_version": 1,
            "game_install": "BGEE",
            "install_mode": "start_from_scratch",
            "weidu_logs": { "bgee": "~X/SETUP-X.TP2~ #1 #1 // X" },
            "answers": { "bgee": { "languages": { "x": "polish" } } }
        })
        .to_string(),
    )
    .expect("code");
    let workspace = crate::registry::workspace_model::ModlistWorkspaceState {
        order_bgee: vec![
            crate::registry::workspace_model::ComponentRef {
                tp2: "X/SETUP-X.TP2".to_string(),
                id: 1,
                wlb_inputs: None,
                answers: AnswerSets::default(),
            }
            .with_answer_sets(sets(&[("english", "y"), ("polish", "t")])),
        ],
        ..Default::default()
    };
    let code =
        crate::registry::share_export::overlay_workspace_wlb(&base, &workspace).expect("overlay");
    let json = decoded_json(&code);
    assert!(
        String::from_utf8_lossy(&json).contains("\"answers\""),
        "the new code carries the section"
    );

    let old: OldReaderPayload = serde_json::from_slice(&json).expect("old reader decodes");

    assert_eq!(old.format_version, 1);
    assert_eq!(
        old.weidu_logs.bgee.as_deref(),
        Some("~X/SETUP-X.TP2~ #1 #1 // X // @wlb-inputs: t")
    );
}
