use super::load_component_conflicts;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture(text: &str) -> Fixture {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "bio-conflict-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("fixture directory");
    std::fs::write(dir.join("fixture.tp2"), text).expect("fixture");
    Fixture(dir)
}
fn targets(rules: &HashMap<String, Vec<super::ComponentConflict>>, id: &str) -> Vec<String> {
    let mut out = rules
        .get(id)
        .into_iter()
        .flatten()
        .map(|rule| format!("{}:{}", rule.target_mod, rule.target_component_id))
        .collect::<Vec<_>>();
    out.sort();
    out
}

#[test]
fn shared_header_forbids_have_original_provenance_and_local_rules_stay_local() {
    let root = fixture(
        "BEGIN ~First~ DESIGNATED 60 SUBCOMPONENT @004\nFORBID_COMPONENT ~base.tp2~ 0 ~BEFORE base~\nCOPY ~a~ ~b~\nBEGIN ~Second~ DESIGNATED 61 SUBCOMPONENT @4\nFORBID_COMPONENT ~local.tp2~ 9 ~Local~\nCOPY ~a~ ~b~\nBEGIN ~Third~ SUBCOMPONENT @4\n",
    );
    let path = root.0.join("fixture.tp2");
    let rules = load_component_conflicts(path.to_str().expect("path"));
    assert_eq!(targets(&rules, "60"), ["base:0"]);
    assert_eq!(targets(&rules, "61"), ["base:0", "local:9"]);
    assert_eq!(targets(&rules, "62"), ["base:0"]);
    let inherited = rules["61"]
        .iter()
        .find(|rule| rule.target_mod == "base")
        .expect("shared");
    let origin = inherited.origin.as_ref().expect("origin");
    assert_eq!(origin.component_id, "60");
    assert_eq!(origin.line, 2);
    assert!(origin.inherited);
    assert_eq!(inherited.message.as_deref(), Some("BEFORE base"));
    assert!(inherited.raw_line.starts_with("FORBID_COMPONENT"));
    assert!(
        !rules["60"][0]
            .origin
            .as_ref()
            .expect("direct origin")
            .inherited
    );
}

#[test]
fn empty_first_and_typed_families_prevent_sideways_or_backward_inheritance() {
    let root = fixture(
        "BEGIN ~Empty~ DESIGNATED 10 SUBCOMPONENT @1\nCOPY ~a~ ~b~\nBEGIN ~Later~ SUBCOMPONENT @1\nFORBID_COMPONENT ~later.tp2~ 0 ~Later~\nBEGIN ~Third~ SUBCOMPONENT @1\nBEGIN ~Literal~ SUBCOMPONENT ~1~\nFORBID_COMPONENT ~literal.tp2~ 0 ~Literal~\nBEGIN ~Translation~ SUBCOMPONENT @01\nBEGIN ~Dynamic~ SUBCOMPONENT %unknown%\nFORBID_COMPONENT ~dynamic.tp2~ 0 ~Dynamic~\nBEGIN ~Dynamic2~ SUBCOMPONENT %unknown%\n",
    );
    let rules = load_component_conflicts(root.0.join("fixture.tp2").to_str().expect("path"));
    for id in ["10", "12", "14", "16"] {
        assert!(targets(&rules, id).is_empty(), "{id}");
    }
    assert_eq!(targets(&rules, "11"), ["later:0"]);
    assert_eq!(targets(&rules, "13"), ["literal:0"]);
}

#[test]
fn action_and_conditional_rules_do_not_seed_shared_header_family() {
    for action in [
        "COPY ~a~ ~b~\nFORBID_COMPONENT ~action.tp2~ 1 ~Action~",
        "ACTION_IF 1 BEGIN\nFORBID_COMPONENT ~action.tp2~ 1 ~Conditional~\nEND",
    ] {
        let root = fixture(&format!(
            "BEGIN ~First~ SUBCOMPONENT ~family~\n{action}\nBEGIN ~Second~ SUBCOMPONENT ~family~\n"
        ));
        let rules = load_component_conflicts(root.0.join("fixture.tp2").to_str().expect("path"));
        assert_eq!(targets(&rules, "1"), Vec::<String>::new());
    }
}

#[test]
fn identical_direct_target_wins_without_losing_distinct_shared_targets() {
    let root = fixture(
        "BEGIN ~First~ SUBCOMPONENT ~family~\nFORBID_COMPONENT ~base.tp2~ 0 ~Inherited~\nFORBID_COMPONENT ~other.tp2~ 1 ~Other~\nBEGIN ~Second~ SUBCOMPONENT ~family~\nFORBID_COMPONENT ~base.tp2~ 0 ~Direct~\n",
    );
    let rules = load_component_conflicts(root.0.join("fixture.tp2").to_str().expect("path"));
    assert_eq!(targets(&rules, "1"), ["base:0", "other:1"]);
    let direct = rules["1"]
        .iter()
        .find(|rule| rule.target_mod == "base")
        .expect("base");
    assert_eq!(direct.message.as_deref(), Some("Direct"));
    assert!(!direct.origin.as_ref().expect("origin").inherited);
}

#[test]
fn interpolated_literal_families_never_seed_speculative_inheritance() {
    for family in ["~%dynamic%~", "\"%dynamic%\"", "~prefix%unknown%suffix~"] {
        let root = fixture(&format!(
            "BEGIN ~First~ SUBCOMPONENT {family}\nFORBID_COMPONENT ~target.tp2~ 0 ~Before~\nBEGIN ~Second~ SUBCOMPONENT {family}\n"
        ));
        let rules = load_component_conflicts(root.0.join("fixture.tp2").to_str().expect("path"));
        assert_eq!(targets(&rules, "0"), ["target:0"]);
        assert_eq!(targets(&rules, "1"), Vec::<String>::new(), "{family}");
    }
}

#[test]
fn translation_display_collisions_and_legacy_tra_merge_do_not_mix_families() {
    let root = fixture(
        "BEGIN ~First~ SUBCOMPONENT @1\nFORBID_COMPONENT ~target.tp2~ 0 @3\nBEGIN ~Other~ SUBCOMPONENT @2\nBEGIN ~Literal~ SUBCOMPONENT ~Same~\nBEGIN ~Shared~ SUBCOMPONENT @01\n",
    );
    let preferred = root.0.join("lang/english");
    std::fs::create_dir_all(&preferred).expect("TRA directory");
    std::fs::write(
        preferred.join("setup.tra"),
        b"@1 = ~Same~\n@2 = ~Same~\n@3 = ~BEFORE Caf\xe9~\n",
    )
    .expect("legacy TRA");
    std::fs::write(root.0.join("setup.tra"), "@3 = ~Wrong later candidate~\n").expect("later TRA");
    let rules = load_component_conflicts(root.0.join("fixture.tp2").to_str().expect("path"));
    assert_eq!(targets(&rules, "1"), Vec::<String>::new());
    assert_eq!(targets(&rules, "2"), Vec::<String>::new());
    assert_eq!(targets(&rules, "3"), ["target:0"]);
    assert_eq!(rules["3"][0].message.as_deref(), Some("BEFORE Café"));
}
