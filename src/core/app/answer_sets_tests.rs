use std::collections::BTreeMap;

use super::*;
use crate::app::state::Step2ComponentState;

fn sets(pairs: &[(&str, &str)]) -> AnswerSets {
    let mut out = AnswerSets::default();
    for (language, spec) in pairs {
        out.set(language, spec);
    }
    out
}

fn component(raw_line: &str) -> Step2ComponentState {
    Step2ComponentState {
        component_id: "2".to_string(),
        label: "Label".to_string(),
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
        checked: true,
        selected_order: Some(1),
    }
}

fn polish_languages() -> BTreeMap<String, String> {
    BTreeMap::from([("mod".to_string(), "polish".to_string())])
}

fn key() -> AnswerKey {
    AnswerKey::new(GameSlot::First, "MOD/MOD.TP2", 2)
}

#[test]
fn answer_sets_split_and_merge_round_trip() {
    let all = sets(&[("english", "y,1"), ("polish", "t,1"), ("german", "j")]);
    let (active, rest) = all.clone().split("polish");
    assert_eq!(active.as_deref(), Some("t,1"));
    assert_eq!(rest, sets(&[("english", "y,1"), ("german", "j")]));
    assert_eq!(rest.merged("polish", active.as_deref()), all);
    let (missing, unchanged) = all.clone().split("czech");
    assert_eq!(missing, None);
    assert_eq!(unchanged, all);
    assert_eq!(
        all.merged("german", None),
        sets(&[("english", "y,1"), ("polish", "t,1")])
    );
    assert_eq!(
        all.languages().collect::<Vec<_>>(),
        ["english", "german", "polish"]
    );
    let json = serde_json::to_string(&all).expect("serialize");
    assert_eq!(json, r#"{"english":"y,1","german":"j","polish":"t,1"}"#);
    assert_eq!(
        serde_json::from_str::<AnswerSets>(&json).expect("parse"),
        all
    );
}

#[test]
fn legacy_marker_is_english() {
    assert_eq!(
        AnswerSets::from_legacy(Some("y,1")),
        sets(&[("english", "y,1")])
    );
    assert_eq!(
        AnswerSets::from_legacy(Some("y,1")).get(UNTAGGED_LANGUAGE),
        Some("y,1")
    );
    assert!(AnswerSets::from_legacy(None).is_empty());
    assert!(AnswerSets::from_legacy(Some("  ")).is_empty());
}

#[test]
fn blank_spec_removes_language() {
    let mut answers = sets(&[("english", "y"), ("polish", "t")]);
    answers.set("polish", "");
    assert_eq!(answers, sets(&[("english", "y")]));
    answers.set("english", "   ");
    assert!(answers.is_empty());
    assert_eq!(
        sets(&[("english", "y")]).merged("english", Some("")),
        AnswerSets::default()
    );
    let mut book = AnswerBook::default();
    book.set_others(key(), sets(&[("english", "y")]));
    book.set_others(key(), AnswerSets::default());
    assert!(book.is_empty());
}

#[test]
fn polish_scan_puts_polish_set_on_line_and_keeps_english_in_book() {
    let languages = polish_languages();
    let mut book = AnswerBook::default();
    let mut row = component("~MOD/MOD.TP2~ #1 #2 // Label");
    let all = sets(&[("english", "y"), ("polish", "t")]);
    AnswerSide {
        slot: GameSlot::First,
        languages: &languages,
        book: &mut book,
    }
    .intake_component(&mut row, &all);
    assert_eq!(
        row.raw_line,
        "~MOD/MOD.TP2~ #1 #2 // Label // @wlb-inputs: t"
    );
    assert_eq!(book.others(&key()), Some(&sets(&[("english", "y")])));
    assert_eq!(
        outgoing(&row.raw_line, GameSlot::First, "polish", &book),
        all
    );
    assert_eq!(
        book.others(&AnswerKey::new(GameSlot::Second, "MOD/MOD.TP2", 2)),
        None
    );
}

#[test]
fn english_only_answers_on_polish_scan_leave_line_unanswered() {
    let languages = polish_languages();
    let mut book = AnswerBook::default();
    let mut row = component("~MOD/MOD.TP2~ #1 #2 // Label // @wlb-inputs: stale");
    AnswerSide {
        slot: GameSlot::First,
        languages: &languages,
        book: &mut book,
    }
    .intake_component(&mut row, &sets(&[("english", "y")]));
    assert_eq!(row.raw_line, "~MOD/MOD.TP2~ #1 #2 // Label");
    assert!(!crate::ui::step3::installer_answers_step3::has_saved_answers(&row.raw_line));
    assert_eq!(book.others(&key()), Some(&sets(&[("english", "y")])));
}

#[test]
fn undecodable_install_language_set_stays_on_the_line() {
    let languages = polish_languages();
    let mut book = AnswerBook::default();
    let mut row = component("~MOD/MOD.TP2~ #1 #2 // Label // @wlb-inputs: old");
    let undecodable = "bio-json-v9:[\"t\"]";
    let all = sets(&[("english", "y"), ("polish", undecodable)]);
    AnswerSide {
        slot: GameSlot::First,
        languages: &languages,
        book: &mut book,
    }
    .intake_component(&mut row, &all);
    assert_eq!(
        row.raw_line,
        format!("~MOD/MOD.TP2~ #1 #2 // Label // @wlb-inputs: {undecodable}")
    );
    assert!(crate::app::wlb_inputs::decode_answers(undecodable).is_err());
    assert_eq!(book.others(&key()), Some(&sets(&[("english", "y")])));
    assert_eq!(
        outgoing(&row.raw_line, GameSlot::First, "polish", &book),
        all
    );
}

#[test]
fn a_set_with_a_line_break_never_reaches_the_line() {
    let languages = polish_languages();
    let injected = "y\n~EVIL/SETUP-EVIL.TP2~ #0 #0 // x";
    let mut book = AnswerBook::default();
    let mut row = component("~MOD/MOD.TP2~ #1 #2 // Label // @wlb-inputs: old");
    AnswerSide {
        slot: GameSlot::First,
        languages: &languages,
        book: &mut book,
    }
    .intake_component(&mut row, &sets(&[("polish", injected)]));
    assert_eq!(row.raw_line, "~MOD/MOD.TP2~ #1 #2 // Label");

    let mut book = AnswerBook::default();
    book.set_others(key(), sets(&[("polish", injected)]));
    let mut mods = vec![crate::app::state::Step2ModState {
        name: "Mod".to_string(),
        tp_file: "MOD/MOD.TP2".to_string(),
        tp2_path: "/mods/MOD/MOD.TP2".to_string(),
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
        components: vec![component("~MOD/MOD.TP2~ #1 #2 // Label")],
    }];
    AnswerSide {
        slot: GameSlot::First,
        languages: &languages,
        book: &mut book,
    }
    .settle(&mut mods);
    assert_eq!(
        mods[0].components[0].raw_line,
        "~MOD/MOD.TP2~ #1 #2 // Label"
    );
    assert!(!mods[0].components[0].raw_line.contains('\n'));
}

#[test]
fn install_language_reads_the_side_map_and_defaults_to_english() {
    let step2 = Step2State {
        bg2ee_mod_languages: polish_languages(),
        ..Step2State::default()
    };
    assert_eq!(
        install_language(&step2, GameSlot::Second, "MOD/MOD.TP2"),
        "polish"
    );
    assert_eq!(
        install_language(&step2, GameSlot::First, "MOD/MOD.TP2"),
        "english"
    );
}
