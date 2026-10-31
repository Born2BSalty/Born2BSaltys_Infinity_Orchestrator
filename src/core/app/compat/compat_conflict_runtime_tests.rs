use super::{build_conflict_scan_context, scan_conflict_hit_with_context};
use crate::app::compat_rule_runtime::{CompatActiveItem, collect_step3_active_items};
use crate::app::state::Step3ItemState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture(files: &[(&str, &str)]) -> Fixture {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "bio-conflict-runtime-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("directory");
    for (name, text) in files {
        std::fs::write(dir.join(name), text).expect("fixture");
    }
    Fixture(dir)
}
fn item(name: &str, id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: name.to_string(),
        component_id: id.to_string(),
        mod_name: name.to_string(),
        component_label: id.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 99,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}
fn selected(root: &Fixture, items: &[Step3ItemState]) -> Vec<CompatActiveItem> {
    let paths = items
        .iter()
        .map(|item| {
            (
                format!(
                    "{}|{}",
                    item.tp_file.to_uppercase(),
                    item.mod_name.to_uppercase()
                ),
                root.0
                    .join(format!("{}.tp2", item.tp_file))
                    .to_string_lossy()
                    .into_owned(),
            )
        })
        .collect();
    collect_step3_active_items(items, &paths)
}

#[test]
fn selected_real_step3_items_use_positions_and_preserve_inherited_evidence() {
    let root = fixture(&[
        (
            "ascension.tp2",
            "BEGIN ~First~ DESIGNATED 60 SUBCOMPONENT @100\nFORBID_COMPONENT ~stratagems.tp2~ 5900 ~Install BEFORE SCS~\nBEGIN ~Second~ DESIGNATED 61 SUBCOMPONENT @100\n",
        ),
        ("stratagems.tp2", "BEGIN ~SCS~ DESIGNATED 5900\n"),
    ]);
    for id in ["60", "61"] {
        let active = selected(&root, &[item("stratagems", "5900"), item("ascension", id)]);
        let context = build_conflict_scan_context(&active, &mut HashMap::new());
        let hit =
            scan_conflict_hit_with_context("ascension", id, Some(2), &context).expect("order hit");
        assert_eq!(hit.kind, "order_block");
        assert!(hit.message.contains("BEFORE"));
        assert_eq!(hit.source, root.0.join("ascension.tp2").to_string_lossy());
        if id == "61" {
            assert!(hit.raw_evidence.contains("#60"));
            assert!(hit.raw_evidence.contains("line 2"));
        }
        let active = selected(&root, &[item("ascension", id), item("stratagems", "5900")]);
        let context = build_conflict_scan_context(&active, &mut HashMap::new());
        assert!(scan_conflict_hit_with_context("ascension", id, Some(1), &context).is_none());
    }
    let active = selected(&root, &[item("ascension", "61")]);
    let context = build_conflict_scan_context(&active, &mut HashMap::new());
    assert!(scan_conflict_hit_with_context("ascension", "61", Some(1), &context).is_none());
}

#[test]
fn inherited_reciprocal_and_longer_cycles_remain_conflicts() {
    for third in [false, true] {
        let target = if third { "c" } else { "a" };
        let second = format!("BEGIN ~B~\nFORBID_COMPONENT ~{target}.tp2~ 1 ~Before~\n");
        let root = fixture(&[
            (
                "a.tp2",
                "BEGIN ~A0~ SUBCOMPONENT ~family~\nFORBID_COMPONENT ~b.tp2~ 0 ~Before~\nBEGIN ~A1~ SUBCOMPONENT ~family~\n",
            ),
            ("b.tp2", &second),
            (
                "c.tp2",
                "BEGIN ~C0~\nBEGIN ~C1~\nFORBID_COMPONENT ~a.tp2~ 1 ~Before~\n",
            ),
        ]);
        let mut items = vec![item("a", "1"), item("b", "0")];
        if third {
            items.push(item("c", "1"));
        }
        let active = selected(&root, &items);
        let context = build_conflict_scan_context(&active, &mut HashMap::new());
        let hit = scan_conflict_hit_with_context("a", "1", Some(1), &context).expect("cycle");
        assert_eq!(hit.kind, "conflict");
        assert!(hit.raw_evidence.contains("#0"));
    }
}

fn component(id: &str, order: usize) -> crate::app::state::Step2ComponentState {
    crate::app::state::Step2ComponentState {
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
        selected_order: Some(order),
    }
}
fn mod_state(
    root: &Fixture,
    name: &str,
    id: &str,
    order: usize,
) -> crate::app::state::Step2ModState {
    crate::app::state::Step2ModState {
        name: name.to_string(),
        tp_file: name.to_string(),
        tp2_path: root
            .0
            .join(format!("{name}.tp2"))
            .to_string_lossy()
            .into_owned(),
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
        components: vec![component(id, order)],
    }
}

#[test]
fn step2_suppresses_one_way_order_and_step3_presents_nonblocking_install_order() {
    use crate::app::compat_conflict_scan::apply_step2_scan_conflict;
    use crate::app::compat_rule_runtime::kind_disables_selection;
    use crate::app::compat_step3_rules::{collect_step3_compat_markers, marker_key};
    let root = fixture(&[
        (
            "source.tp2",
            "BEGIN ~First~ DESIGNATED 60 SUBCOMPONENT @100\nFORBID_COMPONENT ~target.tp2~ 5900 ~Install BEFORE target~\nBEGIN ~Second~ DESIGNATED 61 SUBCOMPONENT @100\n",
        ),
        ("target.tp2", "BEGIN ~Target~ DESIGNATED 5900\n"),
    ]);
    let mut mods = vec![
        mod_state(&root, "target", "5900", 1),
        mod_state(&root, "source", "61", 2),
    ];
    apply_step2_scan_conflict(&mut mods);
    assert!(
        mods.iter()
            .flat_map(|state| &state.components)
            .all(|component| component.compat_kind.is_none() && !component.disabled)
    );
    let state = crate::app::state::Step1State {
        game_install: "BG2EE".to_string(),
        ..crate::app::state::Step1State::default()
    };
    let items = [item("target", "5900"), item("source", "61")];
    let markers = collect_step3_compat_markers(&state, "BG2EE", &mods, &items, &[]);
    let marker = markers
        .get(&marker_key(&items[1]))
        .expect("Step3 order marker");
    assert_eq!(marker.kind, "order_block");
    assert!(!kind_disables_selection(&marker.kind));
    assert!(
        marker
            .raw_evidence
            .as_ref()
            .expect("evidence")
            .contains("#60")
    );
    let reversed = [items[1].clone(), items[0].clone()];
    let markers = collect_step3_compat_markers(&state, "BG2EE", &mods, &reversed, &[]);
    assert!(!markers.contains_key(&marker_key(&reversed[0])));
}
