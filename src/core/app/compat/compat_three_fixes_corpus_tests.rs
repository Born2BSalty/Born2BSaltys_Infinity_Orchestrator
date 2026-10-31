use crate::app::compat_conflict_parse::load_component_conflicts;
use crate::app::compat_conflict_runtime::{
    build_conflict_scan_context, scan_conflict_hit_with_context,
};
use crate::app::compat_dependency_parse::load_component_requirements;
use crate::app::compat_rule_runtime::{CompatActiveItem, kind_disables_selection};
use crate::app::compat_text::read_compat_text;
use crate::app::compat_tp2_blocks::collect_component_flags;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;

#[test]
fn opt_in_native_original_corpus_and_genuine_runtime_proof() {
    let Ok(input) = std::env::var("BIO_COMPAT_CORPUS_PATHS") else {
        return;
    };
    let output = std::env::var("BIO_COMPAT_CORPUS_OUTPUT").expect("explicit corpus output");
    let paths = std::fs::read_to_string(input).expect("corpus path list");
    let paths = paths
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 139);
    let mut records = Vec::<Value>::new();
    for path in &paths {
        let text = match read_compat_text(Path::new(path)) {
            Ok(text) => text,
            Err(error) => {
                records.push(json!({"path":path,"read_error":format!("{error:?}")}));
                continue;
            }
        };
        let flags=collect_component_flags(&text).into_iter().map(|flag| json!({"id":flag.component_id,"family":format!("{:?}",flag.family),"predicates":flag.predicates.iter().map(|predicate|json!({"line":predicate.line,"kind":format!("{:?}",predicate.kind),"prefix":predicate.prefix,"expression":predicate.expression})).collect::<Vec<_>>()})).collect::<Vec<_>>();
        let requirements=load_component_requirements(path).into_iter().map(|(id,rules)|(id,rules.iter().map(|rule|json!({"raw":rule.raw_line,"message":rule.message,"targets":rule.targets.iter().map(|target|json!([target.target_mod,target.target_component_id])).collect::<Vec<_>>()})).collect::<Vec<_>>())).collect::<HashMap<_,_>>();
        let conflicts=load_component_conflicts(path).into_iter().map(|(id,rules)|(id,rules.iter().map(|rule|json!({"raw":rule.raw_line,"message":rule.message,"target_mod":rule.target_mod,"target_id":rule.target_component_id,"origin":rule.origin.as_ref().map(|origin|json!({"component_id":origin.component_id,"line":origin.line,"inherited":origin.inherited}))})).collect::<Vec<_>>())).collect::<HashMap<_,_>>();
        let guards=super::super::load_component_guards(path).into_iter().map(|(id,rules)|(id,rules.iter().map(|rule|json!({"expression":rule.eval_text,"display":rule.display_line,"provenance":rule.inherited_provenance()})).collect::<Vec<_>>())).collect::<HashMap<_,_>>();
        records.push(json!({"path":path,"flags":flags,"requirements":requirements,"conflicts":conflicts,"guards":guards}));
    }
    assert_native_legacy_recovery(&records);
    let find = |needle: &str| {
        paths
            .iter()
            .find(|path| path.to_ascii_lowercase().contains(needle))
            .copied()
            .expect("original witness path")
    };
    assert_bubb_literal_guard(find("bubb_revert_pathfinding.tp2"));
    let ascension = find("ascension.tp2");
    let ja = find("ja#bgt_advpack.tp2");
    let bgee = super::mismatch_context("BGEE", "BGEE", &[]);
    let ee = super::mismatch_context("BG2EE", "BG2EE", &[]);
    let asc_hit = super::super::scan_predicate_guard_hit(ascension, "0", &bgee)
        .expect("Ascension0 wrong game");
    assert_eq!(asc_hit.kind, "mismatch");
    let ja_hit =
        super::super::scan_predicate_guard_hit(ja, "27", &ee).expect("JA27 classic restriction");
    assert_eq!(ja_hit.kind, "mismatch");
    assert!(ja_hit.raw_evidence.to_ascii_lowercase().contains("bgt"));
    let rules = load_component_conflicts(ascension);
    for target in [
        ("stratagems", "5900"),
        ("spell_rev", "0"),
        ("wheels", "0"),
        ("iylos", "0"),
    ] {
        assert!(
            rules["61"]
                .iter()
                .any(|rule| rule.target_mod == target.0 && rule.target_component_id == target.1),
            "{target:?}"
        );
    }
    let mut orders = Vec::<Value>::new();
    for id in ["60", "61"] {
        for reversed in [false, true] {
            let order = if reversed { 2 } else { 1 };
            let active = vec![
                CompatActiveItem {
                    tp_file: "ascension".to_string(),
                    mod_name: "Ascension".to_string(),
                    tp2_path: ascension.to_string(),
                    component_id: id.to_string(),
                    order: Some(order),
                },
                CompatActiveItem {
                    tp_file: "stratagems".to_string(),
                    mod_name: "SCS".to_string(),
                    tp2_path: String::new(),
                    component_id: "5900".to_string(),
                    order: Some(if reversed { 1 } else { 2 }),
                },
            ];
            let context = build_conflict_scan_context(&active, &mut HashMap::new());
            let hit = scan_conflict_hit_with_context("ascension", id, Some(order), &context);
            if reversed {
                let hit = hit.as_ref().expect("Ascension after SCS order warning");
                assert_eq!(hit.kind, "order_block");
                assert!(!kind_disables_selection(hit.kind));
                assert!(hit.message.contains("BEFORE"));
                if id == "61" {
                    assert!(hit.raw_evidence.contains("#60"));
                }
            } else {
                assert!(hit.is_none());
            }
            orders.push(json!({"component":id,"after_scs":reversed,"hit":hit.map(|hit|json!({"kind":hit.kind,"message":hit.message,"source":hit.source,"evidence":hit.raw_evidence}))}));
        }
    }
    std::fs::write(output,serde_json::to_vec_pretty(&json!({"records":records,"proof":{"ascension0_bgee":asc_hit.kind,"ja27_bg2ee":ja_hit.kind,"orders":orders}})).expect("serialize corpus")).expect("write corpus output");
}

fn assert_native_legacy_recovery(records: &[Value]) {
    assert_eq!(records.len(), 139);
    assert!(
        records
            .iter()
            .all(|record| record.get("read_error").is_none())
    );
    for name in [
        "ac_quest.tp2",
        "setup-bgqe.tp2",
        "bubb_revert_pathfinding.tp2",
        "c#endlessbg1.tp2",
        "c#sodboabri.tp2",
        "garrick-tt.tp2",
        "setup-ja#bgt_advpack.tp2",
        "setup-sirinescall.tp2",
    ] {
        let record = records
            .iter()
            .find(|record| {
                record["path"]
                    .as_str()
                    .expect("path")
                    .to_ascii_lowercase()
                    .ends_with(name)
            })
            .expect("legacy witness");
        assert!(
            !record["flags"]
                .as_array()
                .expect("native components")
                .is_empty(),
            "{name}"
        );
    }
}

fn assert_bubb_literal_guard(bubb: &str) {
    let bubb_guards = super::super::load_component_guards(bubb);
    let bubb_guard = &bubb_guards["0"][0];
    for game in ["bgee", "bg2ee", "eet", "iwdee"] {
        assert!(bubb_guard.eval_text.contains(game));
    }
    assert!(bubb_guard.eval_text.contains("FILE_EXISTS"));
    let bubb_flags =
        collect_component_flags(&read_compat_text(Path::new(bubb)).expect("Bubb native decode"));
    assert_eq!(bubb_flags[0].predicates[0].line, 6);
}
