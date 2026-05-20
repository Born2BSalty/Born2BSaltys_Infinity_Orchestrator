// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync;
use crate::app::state::Step2ModState;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};

pub fn snapshot_current_selection(orchestrator: &mut OrchestratorApp) {
    let snapshot = RescanSnapshot {
        bgee: capture_tab(&orchestrator.wizard_state.step2.bgee_mods),
        bg2ee: capture_tab(&orchestrator.wizard_state.step2.bg2ee_mods),
    };
    let step2 = &mut orchestrator.workspace_view.step2;
    step2.rescan_snapshot = Some(snapshot);
    step2.rescan_drop_warning = None;
    step2.was_scanning = armed_was_scanning_for_inflight_scan();
}

pub(crate) const fn armed_was_scanning_for_inflight_scan() -> bool {
    true
}

fn capture_tab(mods: &[Step2ModState]) -> Vec<RescanSelection> {
    let mut out = Vec::new();
    for mod_state in mods {
        let tp2_upper = mod_state.tp_file.to_ascii_uppercase();
        for component in &mod_state.components {
            if component.checked {
                out.push(RescanSelection {
                    tp2_upper: tp2_upper.clone(),
                    component_id: component.component_id.clone(),
                    selected_order: component.selected_order,
                });
            }
        }
    }
    out
}

pub fn reconcile_on_scan_complete(orchestrator: &mut OrchestratorApp) {
    let scanning_now = orchestrator.wizard_state.step2.is_scanning;
    let was_scanning = orchestrator.workspace_view.step2.was_scanning;
    orchestrator.workspace_view.step2.was_scanning = scanning_now;

    if !completion_edge_fires(was_scanning, scanning_now) {
        return;
    }

    let Some(snapshot) = orchestrator.workspace_view.step2.rescan_snapshot.take() else {
        return;
    };

    if orchestrator.wizard_state.step2.last_scan_report.is_none() {
        return;
    }

    let first_dropped = reapply_snapshot(
        &snapshot.bgee,
        &mut orchestrator.wizard_state.step2.bgee_mods,
    );
    let second_dropped = reapply_snapshot(
        &snapshot.bg2ee,
        &mut orchestrator.wizard_state.step2.bg2ee_mods,
    );
    recompute_mod_checked(&mut orchestrator.wizard_state.step2.bgee_mods);
    recompute_mod_checked(&mut orchestrator.wizard_state.step2.bg2ee_mods);

    let max_order = max_selected_order(&orchestrator.wizard_state.step2.bgee_mods).max(
        max_selected_order(&orchestrator.wizard_state.step2.bg2ee_mods),
    );
    orchestrator.wizard_state.step2.next_selection_order = max_order + 1;

    if std::mem::take(&mut orchestrator.workspace_view.step2.resume_pending) {
        orchestrator.wizard_state.step3.bgee_items =
            step3_sync::build_step3_items(&orchestrator.wizard_state.step2.bgee_mods);
        orchestrator.wizard_state.step3.bg2ee_items =
            step3_sync::build_step3_items(&orchestrator.wizard_state.step2.bg2ee_mods);
    }

    let mut dropped: Vec<&RescanSelection> = Vec::new();
    dropped.extend(first_dropped.iter());
    dropped.extend(second_dropped.iter());
    if dropped.is_empty() {
        orchestrator.workspace_view.step2.rescan_drop_warning = None;
        return;
    }
    let dropped_components = dropped.len();
    let missing_mods = distinct_tp2_count(&dropped);
    orchestrator.workspace_view.step2.rescan_drop_warning = Some(format!(
        "{dropped_components} component(s) dropped \u{2014} {missing_mods} mod(s) no longer present"
    ));
}

const fn completion_edge_fires(was_scanning: bool, scanning_now: bool) -> bool {
    was_scanning && !scanning_now
}

fn reapply_snapshot<'a>(
    snapshot: &'a [RescanSelection],
    mods: &mut [Step2ModState],
) -> Vec<&'a RescanSelection> {
    let mut dropped = Vec::new();
    for entry in snapshot {
        let mut matched = false;
        for mod_state in mods.iter_mut() {
            if mod_state.tp_file.to_ascii_uppercase() != entry.tp2_upper {
                continue;
            }
            for component in &mut mod_state.components {
                if component.component_id == entry.component_id {
                    component.checked = true;
                    component.selected_order = entry.selected_order;
                    matched = true;
                }
            }
        }
        if !matched {
            dropped.push(entry);
        }
    }
    dropped
}

fn recompute_mod_checked(mods: &mut [Step2ModState]) {
    for mod_state in mods.iter_mut() {
        mod_state.checked = mod_state.components.iter().any(|c| c.checked);
    }
}

fn max_selected_order(mods: &[Step2ModState]) -> usize {
    mods.iter()
        .flat_map(|m| m.components.iter())
        .filter_map(|c| c.selected_order)
        .max()
        .unwrap_or(0)
}

fn distinct_tp2_count(dropped: &[&RescanSelection]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    for d in dropped {
        seen.insert(d.tp2_upper.as_str());
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{Step2ComponentState, Step2ModState};

    fn comp(id: &str, checked: bool, order: Option<usize>) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: String::new(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
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
            checked,
            selected_order: order,
        }
    }

    fn mod_state(tp: &str, comps: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: tp.to_string(),
            tp_file: tp.to_string(),
            tp2_path: String::new(),
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
            components: comps,
        }
    }

    #[test]
    fn capture_only_grabs_checked_components_upper_tp2() {
        let mods = vec![mod_state(
            "EeFixPack.tp2",
            vec![comp("0", true, Some(2)), comp("5", false, None)],
        )];
        let snap = capture_tab(&mods);
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].tp2_upper, "EEFIXPACK.TP2");
        assert_eq!(snap[0].component_id, "0");
        assert_eq!(snap[0].selected_order, Some(2));
    }

    #[test]
    fn reapply_restores_checked_and_order_and_reports_drops() {
        let snapshot = vec![
            RescanSelection {
                tp2_upper: "EEFIXPACK.TP2".to_string(),
                component_id: "0".to_string(),
                selected_order: Some(2),
            },
            RescanSelection {
                tp2_upper: "GONEMOD.TP2".to_string(),
                component_id: "1".to_string(),
                selected_order: Some(1),
            },
        ];
        let mut mods = vec![mod_state(
            "eefixpack.tp2",
            vec![comp("0", false, None), comp("5", false, None)],
        )];
        let dropped = reapply_snapshot(&snapshot, &mut mods);
        assert!(mods[0].components[0].checked);
        assert_eq!(mods[0].components[0].selected_order, Some(2));
        assert!(!mods[0].components[1].checked);
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].component_id, "1");
        assert_eq!(distinct_tp2_count(&dropped), 1);
    }

    #[test]
    fn recompute_mod_checked_is_tri_state_any() {
        let mut mods = vec![
            mod_state("a.tp2", vec![comp("0", true, Some(1))]),
            mod_state("b.tp2", vec![comp("0", false, None)]),
        ];
        recompute_mod_checked(&mut mods);
        assert!(mods[0].checked);
        assert!(!mods[1].checked);
    }

    #[test]
    fn max_selected_order_tracks_largest() {
        let mods = vec![mod_state(
            "a.tp2",
            vec![comp("0", true, Some(3)), comp("1", true, Some(7))],
        )];
        assert_eq!(max_selected_order(&mods), 7);
    }

    use crate::app::controller::step3_sync;

    fn run_fast_scan_completion_frame(
        armed_was_scanning: bool,
    ) -> (
        Vec<Step2ModState>,
        Vec<crate::app::state::Step3ItemState>,
        bool,
    ) {
        let snapshot = RescanSnapshot {
            bgee: vec![
                RescanSelection {
                    tp2_upper: "BG1UB/BG1UB.TP2".to_string(),
                    component_id: "11".to_string(),
                    selected_order: Some(1),
                },
                RescanSelection {
                    tp2_upper: "BG1UB/BG1UB.TP2".to_string(),
                    component_id: "0".to_string(),
                    selected_order: Some(2),
                },
            ],
            bg2ee: Vec::new(),
        };
        let scanning_now = false;
        let mut bgee_mods = vec![mod_state(
            "BG1UB/BG1UB.TP2",
            vec![
                comp("0", false, None),
                comp("11", false, None),
                comp("5", false, None),
            ],
        )];
        let mut resume_pending = true;
        let mut step3_items: Vec<crate::app::state::Step3ItemState> = Vec::new();

        if !completion_edge_fires(armed_was_scanning, scanning_now) {
            return (bgee_mods, step3_items, resume_pending);
        }

        let _ = reapply_snapshot(&snapshot.bgee, &mut bgee_mods);
        recompute_mod_checked(&mut bgee_mods);
        if std::mem::take(&mut resume_pending) {
            step3_items = step3_sync::build_step3_items(&bgee_mods);
        }
        (bgee_mods, step3_items, resume_pending)
    }

    #[test]
    fn fixrun4_armed_true_detects_one_frame_warm_cache_completion() {
        let (mods, step3, resume_pending) =
            run_fast_scan_completion_frame(armed_was_scanning_for_inflight_scan());

        assert!(mods[0].components[0].checked, "#0 re-checked");
        assert_eq!(mods[0].components[0].selected_order, Some(2));
        assert!(mods[0].components[1].checked, "#11 re-checked");
        assert_eq!(mods[0].components[1].selected_order, Some(1));
        assert!(!mods[0].components[2].checked, "#5 never selected");
        assert!(mods[0].checked, "mod tri-state re-derived");
        assert!(!resume_pending, "resume_pending consumed by the fired edge");
        let leaves: Vec<&crate::app::state::Step3ItemState> =
            step3.iter().filter(|i| !i.is_parent).collect();
        assert_eq!(leaves.len(), 2, "Step 3 rebuilt with two component rows");
        assert_eq!(leaves[0].component_id, "11");
        assert_eq!(leaves[1].component_id, "0");
    }

    #[test]
    fn fixrun4_armed_false_misses_one_frame_warm_cache_completion_the_bug() {
        let (mods, step3, resume_pending) = run_fast_scan_completion_frame(false);

        assert!(
            !mods[0].components[0].checked && !mods[0].components[1].checked,
            "OLD seeding: snapshot never re-applied (the missed-edge bug)"
        );
        assert!(
            !mods[0].checked,
            "OLD seeding: mod tri-state never restored"
        );
        assert!(
            step3.is_empty(),
            "OLD seeding: Step 3 never rebuilt (Step 3 stays empty — the bug)"
        );
        assert!(
            resume_pending,
            "OLD seeding: resume_pending never consumed (restore silently skipped)"
        );
    }

    #[test]
    fn fixrun4_completion_edge_predicate_is_true_to_false_only() {
        assert!(completion_edge_fires(true, false), "true→false: the edge");
        assert!(
            !completion_edge_fires(false, false),
            "old seeding warm cache: missed"
        );
        assert!(
            !completion_edge_fires(true, true),
            "still scanning: not yet"
        );
        assert!(
            !completion_edge_fires(false, true),
            "scan just started: not yet"
        );
    }
}
