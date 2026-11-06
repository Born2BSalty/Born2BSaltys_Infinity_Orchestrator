use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::compat_predicate_constraints::{
    builtin_metadata, canonical_id, guard_matches, metadata_from_text, planning_snapshot,
};
use crate::app::state::{Step1State, Step2ComponentState, Step2ModState, Step3ItemState};

const GUARD: &str =
    "NOT MOD_IS_INSTALLED ~EET_End.TP2~ ~0~ AND NOT MOD_IS_INSTALLED ~EET_End.TP2~ ~1~";
const METADATA: &str = include_str!("../../config/builtin_predicate_constraints.toml");

struct Fixture(PathBuf, bool);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "bio-continuous-order-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        Self::at(path).expect("create unique fixture")
    }

    fn at(path: PathBuf) -> std::io::Result<Self> {
        let mut fixture = Self(path, false);
        std::fs::create_dir(&fixture.0)?;
        fixture.1 = true;
        crate::platform_defaults::set_config_dir_override(Some(fixture.0.join("config")));
        Ok(fixture)
    }
    fn source(&self, text: &str) -> String {
        let path = self.0.join("Continuous_End.tp2");
        std::fs::write(&path, text).expect("write TP2");
        path.to_string_lossy().into_owned()
    }

    fn finish(self) {
        std::fs::remove_dir_all(&self.0).expect("checked fixture teardown");
        assert!(!self.0.exists(), "fixture leftover");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.1 {
            crate::platform_defaults::clear_config_dir_override_if(&self.0.join("config"));
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn component(id: &str) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: id.to_string(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: String::new(),
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

fn source_mod(path: &str, ids: &[&str]) -> Step2ModState {
    Step2ModState {
        name: "Different imported display name".to_string(),
        tp_file: "Continuous_End.tp2".to_string(),
        tp2_path: path.to_string(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: true,
        hidden_components: Vec::new(),
        components: ids.iter().map(|id| component(id)).collect(),
    }
}

fn item(tp: &str, id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: tp.to_string(),
        component_id: id.to_string(),
        mod_name: "Imported".to_string(),
        component_label: id.to_string(),
        raw_line: "identical imported raw line".to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 99,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn state() -> Step1State {
    Step1State {
        game_install: "EET".to_string(),
        ..Step1State::default()
    }
}

#[test]
fn continuous_constraint_shipped_metadata_and_strict_full_input_matrix() {
    let metadata = builtin_metadata();
    assert!(
        metadata.diagnostics.is_empty(),
        "{:?}",
        metadata.diagnostics
    );
    assert_eq!(metadata.records.len(), 1);
    let record = &metadata.records[0];
    assert_eq!(
        record.components,
        ["0", "1", "2", "3", "4", "5", "7", "8", "9", "10"]
    );
    for expression in [
        GUARD,
        "(not MOD_IS_INSTALLED ~mods/EET_End.tp2~ ~00~) AND (NOT MOD_IS_INSTALLED ~mods\\setup-EET_End.TP2~ ~01~)",
        " ((NOT MOD_IS_INSTALLED \"EET_End.tp2\" \"1\") AND (NOT MOD_IS_INSTALLED ~EET_End.tp2~ ~0~)) ",
    ] {
        assert!(
            guard_matches(record, expression),
            "accepted grammar: {expression}"
        );
    }
    for expression in [
        format!("{GUARD} $"),
        format!("{GUARD},"),
        format!("{GUARD} trailing"),
        format!("{GUARD} AND"),
        format!("{GUARD} AND NOT MOD_IS_INSTALLED ~EET_End.tp2~ ~0~"),
        GUARD.replace(" AND ", " OR "),
        GUARD.replace("NOT ", "!"),
        GUARD.replacen("NOT ", "NOT NOT ", 1),
        GUARD.replace("~1~", "~0~"),
        GUARD.replace("~1~", "~+1~"),
        GUARD.replace("~1~", "~4294967296~"),
        GUARD.replace("EET_End.TP2", "other.tp2"),
        GUARD.replace("~1~", "~1"),
        format!("({GUARD})=0"),
        format!("{GUARD} AND UNKNOWN ~x~"),
    ] {
        assert!(
            !guard_matches(record, &expression),
            "reject complete input: {expression}"
        );
    }
    for (input, expected) in [
        ("000", Some("0")),
        ("4294967295", Some("4294967295")),
        ("+1", None),
        ("-1", None),
        ("4294967296", None),
        (" 1", None),
        ("", None),
    ] {
        assert_eq!(canonical_id(input).as_deref(), expected, "{input}");
    }
}

#[test]
fn continuous_constraint_document_and_entry_rejection_recover_independently() {
    for text in [
        "not valid toml",
        &METADATA.replace("schema_version = 1", "schema_version = 2"),
        &METADATA.replacen(
            "schema_version = 1",
            "schema_version = 1\nunknown = true",
            1,
        ),
    ] {
        let rejected = metadata_from_text(text);
        assert!(rejected.records.is_empty());
        assert!(!rejected.diagnostics.is_empty());
        assert_eq!(metadata_from_text(METADATA).records.len(), 1);
    }
    let entry = METADATA.split_once("[[constraints]]").expect("record").1;
    let mixed = format!(
        "{METADATA}\n[[constraints]]\n{}",
        entry
            .replace("continuous-end-before-eet-end-v1", "invalid-neighbor")
            .replace("install_before", "unsupported")
            .replace(
                r#"components = ["0", "1", "2", "3", "4", "5", "7", "8", "9", "10"]"#,
                r#"components = ["999"]"#
            )
    );
    let parsed = metadata_from_text(&mixed);
    assert_eq!(parsed.records.len(), 1);
    assert!(!parsed.diagnostics.is_empty());
    let duplicate = format!(
        "{METADATA}\n[[constraints]]\n{}",
        entry.replace("continuous-end-before-eet-end-v1", "duplicate")
    );
    assert_eq!(metadata_from_text(&duplicate).records.len(), 1);
    let disjoint = duplicate.replacen("mode = \"EET\"", "mode = \"BG2EE\"", 1);
    assert_eq!(metadata_from_text(&disjoint).records.len(), 2);
}

#[test]
fn continuous_order_every_earlier_occurrence_and_duplicate_source_row_is_retained() {
    let fixture = Fixture::new();
    let text = format!("BEGIN ~Continuous~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n");
    let path = fixture.source(&text);
    let mods = [source_mod(&path, &["0"])];
    let continuous = item("Continuous_End.tp2", "0");
    let end0 = item("EET_End.tp2", "0");
    let end1 = item("setup-EET_End.tp2", "1");
    for (items, expected) in [
        (vec![continuous.clone(), end0.clone(), end1.clone()], 0),
        (vec![end0.clone(), continuous.clone(), end1.clone()], 1),
        (vec![end1, continuous.clone(), end0.clone()], 1),
        (vec![end0.clone(), end0.clone(), continuous.clone()], 2),
        (vec![end0, continuous.clone(), continuous.clone()], 2),
        (vec![continuous], 0),
    ] {
        let result = planning_snapshot(&state(), "BG2EE", &mods, &items, &builtin_metadata());
        assert_eq!(result.outcomes.len(), expected, "{items:?}");
        assert!(
            result
                .outcomes
                .iter()
                .all(|outcome| outcome.kind == "order_violation")
        );
    }
    fixture.finish();
}

#[test]
fn continuous_order_source_freshness_delete_restore_and_scope() {
    let fixture = Fixture::new();
    let text = format!("BEGIN ~Continuous~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n");
    let path = fixture.source(&text);
    let mods = [source_mod(&path, &["0"])];
    let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    let snapshot = || planning_snapshot(&state(), "BG2EE", &mods, &items, &builtin_metadata());
    assert_eq!(snapshot().outcomes[0].kind, "order_violation");
    let modified = std::fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    fixture.source(&text.replace("EET_End.TP2", "not_end.TP2"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("fixture handle");
    file.set_times(std::fs::FileTimes::new().set_modified(modified))
        .expect("restore mtime");
    drop(file);
    assert!(
        snapshot().outcomes.is_empty(),
        "same-size same-mtime edit must not reuse permission"
    );
    std::fs::remove_file(&path).expect("delete fixture source");
    assert_eq!(snapshot().outcomes[0].kind, "source_unverified");
    fixture.source(&text);
    assert_eq!(snapshot().outcomes[0].kind, "order_violation");
    for (mode, tab) in [("BG2EE", "BG2EE"), ("EET", "BGEE"), ("IWDEE", "IWDEE")] {
        let mut step1 = state();
        step1.game_install = mode.to_string();
        assert!(
            planning_snapshot(&step1, tab, &mods, &items, &builtin_metadata())
                .outcomes
                .is_empty()
        );
    }
    fixture.finish();
}

#[test]
fn continuous_order_fixture_unwind_cleanup() {
    let fixture = Fixture::new();
    let root = fixture.0.clone();
    let result = std::panic::catch_unwind(move || {
        fixture.source("BEGIN ~Fixture~ DESIGNATED 0\n");
        panic!("controlled fixture unwind");
    });
    assert!(result.is_err());
    assert!(!root.exists(), "unwind must clean owned fixture");
}

#[test]
fn continuous_order_complete_scanner_evidence_rejects_ambiguous_inputs() {
    let fixture = Fixture::new();
    let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    for (case, text, expected) in [
        (
            "clean",
            format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"),
            "order_violation",
        ),
        (
            "unknown-leading-continuation",
            format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD}\n$ UNKNOWN ~tail~\n"),
            "source_unverified",
        ),
        (
            "unterminated-comment",
            format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n/* unfinished"),
            "source_unverified",
        ),
        (
            "unterminated-header",
            format!(
                "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\nBEGIN ~unfinished"
            ),
            "source_unverified",
        ),
        (
            "duplicate-designated",
            format!(
                "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\nBEGIN ~Again~ DESIGNATED 0\n"
            ),
            "source_unverified",
        ),
    ] {
        let path = fixture.source(&text);
        let mods = [source_mod(&path, &["0"])];
        let result = planning_snapshot(&state(), "BG2EE", &mods, &items, &builtin_metadata());
        assert_eq!(result.outcomes.len(), 1, "{case}");
        assert_eq!(result.outcomes[0].kind, expected, "{case}");
    }
    fixture.finish();
}

#[test]
fn continuous_order_operation_reads_each_physical_source_once_and_errors_recover() {
    use crate::app::compat_predicate_constraints::planning_snapshot_with_reader;
    let fixture = Fixture::new();
    let text = format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n");
    let path = fixture.source(&text);
    let mods = [source_mod(&path, &["0"]), source_mod(&path, &["0"])];
    let items = [
        item("EET_End.tp2", "0"),
        item("Continuous_End.tp2", "0"),
        item("Continuous_End.tp2", "0"),
    ];
    for failed in [false, true, false] {
        let mut reads = 0;
        let mut reader = |_: &std::path::Path| {
            reads += 1;
            if failed {
                Err("injected unreadable source".to_string())
            } else {
                Ok(text.clone())
            }
        };
        let result = planning_snapshot_with_reader(
            &state(),
            "BG2EE",
            &mods,
            &items,
            &builtin_metadata(),
            &mut reader,
        );
        assert_eq!(reads, 1, "one read per operation for shared source");
        assert_eq!(result.outcomes.len(), 2);
        assert!(result.outcomes.iter().all(|outcome| outcome.kind
            == if failed {
                "source_unverified"
            } else {
                "order_violation"
            }));
    }
    fixture.finish();
}

fn named_mod(tp: &str, id: &str) -> Step2ModState {
    let mut value = source_mod("", &[id]);
    value.tp_file = tp.to_string();
    value.name = tp.to_string();
    value
}

#[test]
fn continuous_order_actual_step2_all_covered_components_survive_both_click_orders() {
    use crate::app::compat_logic::apply_step2_compat_rules;
    let fixture = Fixture::new();
    for id in ["0", "1", "2", "3", "4", "5", "7", "8", "9", "10"] {
        let text = format!(
            "BEGIN ~C~ DESIGNATED {id}\nREQUIRE_PREDICATE GAME_IS ~eet~ ~Game~\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
        );
        let path = fixture.source(&text);
        for end_id in ["0", "1"] {
            for continuous_first in [false, true] {
                let mut mods = vec![
                    source_mod(&path, &[id]),
                    named_mod("EET_End.tp2", end_id),
                    named_mod("EET.tp2", "0"),
                ];
                mods[usize::from(continuous_first)].components[0].checked = false;
                apply_step2_compat_rules(&state(), &mut [], &mut mods);
                mods[usize::from(continuous_first)].components[0].checked = true;
                apply_step2_compat_rules(&state(), &mut [], &mut mods);
                let current = &mods[0].components[0];
                assert!(
                    current.checked && !current.disabled,
                    "{id}/{end_id}/{continuous_first}: {current:?}"
                );
                assert_eq!(current.compat_kind, None);
                assert_eq!(current.compat_source, None);
            }
        }
    }
    fixture.finish();
}

#[test]
fn continuous_gate_direct_unrendered_gate_and_preparation_rejection_use_current_source() {
    use crate::app::state::WizardState;
    use crate::app::step5::install_flow::{prepare_start_request, step3_install_block_reason};
    let fixture = Fixture::new();
    let path = fixture.source(&format!(
        "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
    ));
    let mut wizard = WizardState {
        step1: state(),
        ..WizardState::default()
    };
    wizard.step1.eet_bgee_game_folder =
        fixture.0.join("empty-first").to_string_lossy().into_owned();
    wizard.step1.eet_bg2ee_game_folder = fixture
        .0
        .join("empty-second")
        .to_string_lossy()
        .into_owned();
    wizard.step1.eet_pre_dir = fixture.0.join("empty-pre").to_string_lossy().into_owned();
    wizard.step1.eet_new_dir = fixture
        .0
        .join("empty-destination")
        .to_string_lossy()
        .into_owned();
    wizard.step2.bg2ee_mods = vec![source_mod(&path, &["0"])];
    wizard.step3.bg2ee_items = vec![item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    assert!(step3_install_block_reason(&wizard).is_some());
    assert!(!wizard.step3.bg2ee_has_conflict);
    wizard.step5.basic_install_report_path = Some(fixture.0.join("reports/blocked.txt"));
    let mut terminal = crate::app::terminal::EmbeddedTerminal::new().expect("headless terminal");
    assert!(prepare_start_request(&mut wizard, &mut terminal).is_none());
    assert!(!wizard.step5.prep_running && !wizard.step5.prep_ran);
    assert!(wizard.step5.last_status_text.contains("before install"));
    assert!(fixture.0.join("reports/blocked.txt").is_file());
    assert!(!fixture.0.join("empty-destination").exists());
    wizard.step3.bg2ee_items.swap(0, 1);
    assert!(step3_install_block_reason(&wizard).is_none());
    std::fs::remove_file(&path).expect("remove source after previous collection");
    let reason = step3_install_block_reason(&wizard).expect("fresh source failure blocks");
    assert!(reason.contains("source_unverified"));
    fixture.finish();
}

#[test]
fn continuous_gate_legacy_rules_cannot_clear_raw_blockers_or_match_them_as_legacy() {
    use crate::app::compat_rules::CompatRulesFile;
    use crate::app::compat_step3_rules::{
        collect_step3_compat_result_with_metadata_and_rules, marker_key,
    };
    let fixture = Fixture::new();
    let path = fixture.source(&format!(
        "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
    ));
    let mods = [source_mod(&path, &["0"])];
    let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    for body in [
        "[[rules]]\nmod='Continuous_End'\nkind='allow'\nclear_kinds=['order_block','source_unverified']",
        "[[rules]]\nmod='Continuous_End'\nkind='conflict'\nmessage='Generic conflict'\n[[rules]]\nmod='Continuous_End'\nkind='allow'",
        "[[rules]]\nmod='Continuous_End'\nkind='allow'\n[[rules]]\nmod='Continuous_End'\nkind='conflict'\nmessage='Generic conflict'",
        "[[rules]]\nmod='Continuous_End'\nmatch_kind='order_block'\nkind='conflict'\nmessage='Must not see raw constraint as legacy'",
    ] {
        let rules = toml::from_str::<CompatRulesFile>(body)
            .expect("injected rules")
            .rules;
        let result = collect_step3_compat_result_with_metadata_and_rules(
            &state(),
            "BG2EE",
            &mods,
            &items,
            &[],
            &builtin_metadata(),
            &rules,
        );
        assert_eq!(result.constraint_outcomes.len(), 1, "{body}");
        let marker = result
            .markers
            .get(&marker_key(&items[1]))
            .expect("raw issue must be projected");
        assert!(
            marker
                .raw_evidence
                .as_deref()
                .is_some_and(|evidence| evidence.contains("continuous-end-before-eet-end-v1")),
            "{body}: {marker:?}"
        );
        assert_ne!(
            marker.message.as_deref(),
            Some("Must not see raw constraint as legacy")
        );
    }
    fixture.finish();
}

#[test]
fn continuous_order_independent_scans_keep_their_primary_issue() {
    use crate::app::compat_logic::apply_step2_compat_rules_with_load;
    use crate::app::compat_rules::CompatRulesLoad;
    use crate::app::compat_step3_rules::{
        collect_step3_compat_result_with_metadata_and_rules, marker_key,
    };
    let fixture = Fixture::new();
    for (guard, expected) in [
        ("REQUIRE_PREDICATE GAME_IS ~iwdee~ ~Wrong game~", "mismatch"),
        (
            "REQUIRE_COMPONENT ~missing.tp2~ 0 ~Dependency~",
            "missing_dep",
        ),
        (
            "REQUIRE_PREDICATE DIRECTORY_EXISTS ~definitely-absent-file~ ~Path~",
            "path_requirement",
        ),
    ] {
        let guard = guard.replace(
            "definitely-absent-file",
            fixture
                .0
                .join("missing-file")
                .to_str()
                .expect("fixture path"),
        );
        for deferred_first in [false, true] {
            let before = format!("REQUIRE_PREDICATE {GUARD} ~Before~");
            let declarations = if deferred_first {
                format!("{before}\n{guard}")
            } else {
                format!("{guard}\n{before}")
            };
            let path = fixture.source(&format!("BEGIN ~C~ DESIGNATED 0\n{declarations}\n"));
            let mut mods = vec![source_mod(&path, &["0"]), named_mod("EET_End.tp2", "0")];
            let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
            let result = collect_step3_compat_result_with_metadata_and_rules(
                &state(),
                "BG2EE",
                &mods,
                &items,
                &[],
                &builtin_metadata(),
                &[],
            );
            assert_eq!(
                result.constraint_outcomes.len(),
                1,
                "{guard}/{deferred_first}"
            );
            let marker = &result.markers[&marker_key(&items[1])];
            assert_eq!(
                marker.kind, expected,
                "{guard}/{deferred_first}: {marker:?}"
            );
            apply_step2_compat_rules_with_load(
                &state(),
                &mut [],
                &mut mods,
                &builtin_metadata(),
                CompatRulesLoad {
                    rules: Vec::new(),
                    error: None,
                },
            );
            assert_eq!(
                mods[0].components[0].compat_kind.as_deref(),
                Some(expected),
                "{guard}/{deferred_first}"
            );
        }
    }
    fixture.finish();
}

#[test]
fn continuous_order_public_share_import_reaches_step2_sync_and_repeated_next() {
    use crate::app::app_nav::{NextAction, decide_next_action};
    use crate::app::modlist_share::{encode_share_payload_text, import_modlist_share_code};
    use crate::app::state::WizardState;
    let fixture = Fixture::new();
    let path = fixture.source(&format!(
        "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
    ));
    let payload = serde_json::json!({
        "format_version": 1,
        "game_install": "EET",
        "install_mode": "start_from_scratch",
        "weidu_logs": { "bgee": "", "bg2ee": "~Continuous_End.tp2~ #0 #0 // Imported renamed component: 1\n~EET_End.tp2~ #0 #0 // End: 1\n" }
    });
    let code = encode_share_payload_text(&payload.to_string()).expect("encode public share");
    let mut wizard = WizardState::default();
    wizard.step1.eet_bgee_log_folder = fixture.0.join("logs-first").to_string_lossy().into_owned();
    wizard.step1.eet_bg2ee_log_folder =
        fixture.0.join("logs-second").to_string_lossy().into_owned();
    import_modlist_share_code(&mut wizard, &code)
        .expect("public import into isolated config/log roots");
    wizard.step2.bg2ee_mods = vec![source_mod(&path, &["0"]), named_mod("EET_End.tp2", "0")];
    for value in &mut wizard.step2.bg2ee_mods {
        value.components[0].checked = false;
    }
    crate::app::app_step2_log::apply_weidu_log_selection_from_path(
        &mut wizard,
        false,
        Some(fixture.0.join("logs-second/weidu.log")),
    );
    assert!(
        wizard
            .step2
            .bg2ee_mods
            .iter()
            .all(|value| value.components[0].checked)
    );
    wizard.current_step = 1;
    let NextAction::SyncStep3AndAdvance { signature } = decide_next_action(&wizard) else {
        panic!("imported selection must request actual sync")
    };
    crate::app::app_step3_sync_flow::sync_step3_from_step2(&mut wizard);
    wizard.last_step2_sync_signature = Some(signature);
    assert_eq!(decide_next_action(&wizard), NextAction::Advance);
    wizard.current_step = 2;
    assert_eq!(decide_next_action(&wizard), NextAction::Advance);
    assert_eq!(decide_next_action(&wizard), NextAction::Advance);
    let result = crate::app::compat_step3_rules::collect_step3_compat_result(
        &wizard.step1,
        "BG2EE",
        &wizard.step2.bg2ee_mods,
        &wizard.step3.bg2ee_items,
        &[],
    );
    assert!(
        result.constraint_outcomes.is_empty(),
        "valid imported order"
    );
    fixture.finish();
}

#[test]
fn continuous_order_same_mod_exclusions_remain_conflicts_in_both_plan_orders() {
    use crate::app::compat_step3_rules::{
        collect_step3_compat_result_with_metadata_and_rules, marker_key,
    };
    let fixture = Fixture::new();
    for id in ["1", "2", "3", "4", "5", "7", "8", "9", "10"] {
        for deferred_first in [false, true] {
            let order = format!("REQUIRE_PREDICATE {GUARD} ~Before~");
            let conflict = format!(
                "REQUIRE_PREDICATE NOT MOD_IS_INSTALLED ~Continuous_End.tp2~ ~{id}~ ~Exclusive~"
            );
            let guards = if deferred_first {
                format!("{order}\n{conflict}")
            } else {
                format!("{conflict}\n{order}")
            };
            let path = fixture.source(&format!(
                "BEGIN ~All~ DESIGNATED 0\n{guards}\nBEGIN ~Individual~ DESIGNATED {id}\n{order}\n"
            ));
            let mods = [source_mod(&path, &["0", id])];
            for reversed in [false, true] {
                let mut rows = vec![
                    item("Continuous_End.tp2", "0"),
                    item("Continuous_End.tp2", id),
                ];
                if reversed {
                    rows.reverse();
                }
                for earlier_end in [false, true] {
                    let mut items = rows.clone();
                    if earlier_end {
                        items.insert(0, item("EET_End.tp2", "0"));
                    } else {
                        items.push(item("EET_End.tp2", "0"));
                    }
                    let result = collect_step3_compat_result_with_metadata_and_rules(
                        &state(),
                        "BG2EE",
                        &mods,
                        &items,
                        &[],
                        &builtin_metadata(),
                        &[],
                    );
                    let all = items
                        .iter()
                        .find(|row| row.tp_file == "Continuous_End.tp2" && row.component_id == "0")
                        .expect("all row");
                    assert_eq!(
                        result.markers[&marker_key(all)].kind,
                        "conflict",
                        "{id}/{deferred_first}/{reversed}/{earlier_end}"
                    );
                    assert_eq!(
                        result.constraint_outcomes.len(),
                        if earlier_end { 2 } else { 0 }
                    );
                }
            }
        }
    }
    fixture.finish();
}

#[test]
fn continuous_constraint_rejected_overlap_and_legacy_unknown_field_have_no_authority() {
    use crate::app::compat_rules::CompatRulesFile;
    let entry = METADATA.split_once("[[constraints]]").expect("record").1;
    let conflict = format!(
        "{METADATA}\n[[constraints]]\n{}",
        entry
            .replace("continuous-end-before-eet-end-v1", "contradiction")
            .replace("install_before", "install_after")
    );
    let rejected = metadata_from_text(&conflict);
    assert!(rejected.records.is_empty());
    assert!(!rejected.diagnostics.is_empty());
    let legacy = toml::from_str::<CompatRulesFile>("schema_version=1\npredicate_constraints=[{ id='pretend-authority' }]\n[[rules]]\nmod='legacy'\nkind='warning'\nmessage='kept'").expect("existing legacy unknown field behavior");
    assert_eq!(legacy.rules.len(), 1);
    assert_eq!(legacy.rules[0].message, "kept");
    assert_eq!(builtin_metadata().records.len(), 1);
}

#[test]
fn continuous_order_source_resolution_rejects_missing_mismatched_and_multiple_physical_files() {
    let fixture = Fixture::new();
    let text = format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n");
    let path = fixture.source(&text);
    let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    let alias = fixture.0.join("setup-Continuous_End.tp2");
    std::fs::write(&alias, &text).expect("distinct physical alias source");
    let wrong = fixture.0.join("unrelated.tp2");
    std::fs::write(&wrong, &text).expect("mismatched physical basename");
    for mods in [
        vec![],
        vec![source_mod("", &["0"])],
        vec![source_mod(wrong.to_str().expect("path"), &["0"])],
        vec![source_mod(&path, &["1"])],
        vec![
            source_mod(&path, &["0"]),
            source_mod(alias.to_str().expect("path"), &["0"]),
        ],
    ] {
        let result = planning_snapshot(&state(), "BG2EE", &mods, &items, &builtin_metadata());
        assert_eq!(result.outcomes.len(), 1);
        assert_eq!(result.outcomes[0].kind, "source_unverified");
    }
    for id in ["6", "11", "+0", "4294967296"] {
        assert!(
            planning_snapshot(
                &state(),
                "BG2EE",
                &[source_mod(&path, &[id])],
                &[item("Continuous_End.tp2", id)],
                &builtin_metadata()
            )
            .rows
            .is_empty()
        );
    }
    let mut display_only = source_mod(&path, &["0"]);
    display_only.tp_file = "unrelated.tp2".to_string();
    display_only.name = "Continuous_End".to_string();
    assert!(
        planning_snapshot(
            &state(),
            "BG2EE",
            &[display_only],
            &[item("unrelated.tp2", "0")],
            &builtin_metadata()
        )
        .rows
        .is_empty()
    );
    fixture.finish();
}

#[test]
fn continuous_gate_actual_collector_and_gate_refresh_same_metadata_source() {
    use crate::app::compat_step3_rules::collect_step3_compat_result;
    use crate::app::state::WizardState;
    use crate::app::step5::install_flow::step3_install_block_reason;
    let fixture = Fixture::new();
    let text = format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n");
    let path = fixture.source(&text);
    let modified = std::fs::metadata(&path)
        .expect("metadata")
        .modified()
        .expect("mtime");
    let mut wizard = WizardState {
        step1: state(),
        ..WizardState::default()
    };
    wizard.step2.bg2ee_mods = vec![source_mod(&path, &["0"])];
    wizard.step3.bg2ee_items = vec![item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    for (source, blocked) in [
        (Some(text.clone()), true),
        (Some(text.replace("EET_End.TP2", "not_end.TP2")), false),
        (None, true),
        (Some(text), true),
    ] {
        if let Some(source) = source {
            fixture.source(&source);
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("fixture handle");
            file.set_times(std::fs::FileTimes::new().set_modified(modified))
                .expect("preserve mtime");
        } else {
            std::fs::remove_file(&path).expect("delete fixture source");
        }
        let result = collect_step3_compat_result(
            &wizard.step1,
            "BG2EE",
            &wizard.step2.bg2ee_mods,
            &wizard.step3.bg2ee_items,
            &[],
        );
        assert_eq!(!result.constraint_outcomes.is_empty(), blocked);
        assert_eq!(step3_install_block_reason(&wizard).is_some(), blocked);
    }
    fixture.finish();
}

#[test]
fn continuous_constraint_legacy_valid_malformed_recovery_does_not_change_builtin_permission() {
    use crate::app::compat_rules::load_rules;
    use crate::app::compat_step3_rules::collect_step3_compat_result;
    let fixture = Fixture::new();
    let path = fixture.source(&format!(
        "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\n"
    ));
    let rules_path = fixture.0.join("config/step2_compat_rules_user.toml");
    std::fs::create_dir_all(rules_path.parent().expect("config root")).expect("config directory");
    let mods = [source_mod(&path, &["0"])];
    let items = [item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")];
    for (content, failed) in [
        (
            "schema_version=1\n[[rules]]\nmod='Continuous_End'\nkind='allow'\n",
            false,
        ),
        ("malformed = [", true),
        (
            "schema_version=1\n[[rules]]\nmod='Continuous_End'\nkind='warning'\nmessage='recovered'\n",
            false,
        ),
    ] {
        std::fs::write(&rules_path, content).expect("write isolated legacy configuration");
        assert_eq!(load_rules().error.is_some(), failed);
        let result = collect_step3_compat_result(&state(), "BG2EE", &mods, &items, &[]);
        assert_eq!(result.constraint_outcomes.len(), 1);
        assert_eq!(result.constraint_outcomes[0].kind, "order_violation");
        let mut wizard = crate::app::state::WizardState {
            step1: state(),
            ..crate::app::state::WizardState::default()
        };
        wizard.step2.bg2ee_mods = mods.to_vec();
        wizard.step3.bg2ee_items = items.to_vec();
        assert!(
            crate::app::step5::install_flow::step3_install_block_reason(&wizard).is_some(),
            "legacy loads cannot hide actual raw gate blockers"
        );
    }
    fixture.finish();
}

#[test]
fn continuous_order_fixture_collision_preserves_existing_and_partial_write_unwinds() {
    let fixture = Fixture::new();
    let root = fixture.0.clone();
    let marker = root.join("preexisting.txt");
    std::fs::write(&marker, "preserved").expect("fixture marker");
    assert!(Fixture::at(root).is_err());
    assert_eq!(
        std::fs::read_to_string(&marker).expect("existing marker"),
        "preserved"
    );
    fixture.finish();
    let fixture = Fixture::new();
    let root = fixture.0.clone();
    let result = std::panic::catch_unwind(move || {
        fixture.source("partial setup completed");
        std::fs::write(fixture.0.join("missing-parent/file.tp2"), "fails")
            .expect("controlled partial setup failure");
    });
    assert!(result.is_err());
    assert!(!root.exists());
}

#[test]
fn continuous_order_commented_independent_game_guard_preserves_mismatch() {
    use crate::app::compat_logic::apply_step2_compat_rules_with_load;
    use crate::app::compat_rules::CompatRulesLoad;
    let fixture = Fixture::new();
    let path = fixture.source(&format!("BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE GAME_IS /* note */ ~iwdee~ ~Wrong~\nREQUIRE_PREDICATE {GUARD} ~Before~\n"));
    let mut mods = vec![source_mod(&path, &["0"]), named_mod("EET_End.tp2", "0")];
    apply_step2_compat_rules_with_load(
        &state(),
        &mut [],
        &mut mods,
        &builtin_metadata(),
        CompatRulesLoad {
            rules: Vec::new(),
            error: None,
        },
    );
    assert_eq!(
        mods[0].components[0].compat_kind.as_deref(),
        Some("mismatch")
    );
    assert!(mods[0].components[0].disabled);
    fixture.finish();
}

#[test]
fn continuous_order_malformed_header_after_action_is_unverified() {
    let fixture = Fixture::new();
    for header in ["BEGIN", "BEGIN 123"] {
        let path = fixture.source(&format!(
            "BEGIN ~C~ DESIGNATED 0\nREQUIRE_PREDICATE {GUARD} ~Before~\nOUTER_SET x=1\n{header}\n"
        ));
        let result = planning_snapshot(
            &state(),
            "BG2EE",
            &[source_mod(&path, &["0"])],
            &[item("EET_End.tp2", "0"), item("Continuous_End.tp2", "0")],
            &builtin_metadata(),
        );
        assert_eq!(result.outcomes.len(), 1, "{header}");
        assert_eq!(result.outcomes[0].kind, "source_unverified", "{header}");
    }
    fixture.finish();
}

#[path = "continuous_regression_tests.rs"]
mod regression;
