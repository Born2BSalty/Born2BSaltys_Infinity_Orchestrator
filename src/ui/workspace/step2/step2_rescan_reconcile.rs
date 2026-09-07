// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync;
use crate::app::state::{Step2ComponentState, Step2ModState};
use crate::registry::workspace_model::ModsSource;
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};
use crate::ui::workspace::workspace_state_loader::{extract_wlb_inputs, reattach_wlb_inputs};

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

pub fn arm_post_download_snapshot(orchestrator: &mut OrchestratorApp) {
    let snapshot = RescanSnapshot {
        bgee: capture_tab(&orchestrator.wizard_state.step2.bgee_mods),
        bg2ee: capture_tab(&orchestrator.wizard_state.step2.bg2ee_mods),
    };
    orchestrator
        .workspace_view
        .step2
        .pending_update_download_snapshot = Some(snapshot);
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
                    wlb_inputs: extract_wlb_inputs(&component.raw_line),
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

    advance_pending_download_snapshot(orchestrator, was_scanning, scanning_now);

    if !completion_edge_fires(was_scanning, scanning_now) {
        return;
    }

    let modlist_id = orchestrator.workspace_view.modlist_id.trim().to_string();
    let current_source = orchestrator
        .workspace_state
        .get(modlist_id.as_str())
        .map_or_else(ModsSource::default, |w| w.mods_source);
    if let Some(workspace) = orchestrator.workspace_state.get_mut(modlist_id.as_str()) {
        workspace.last_rescanned_mods_source = current_source;
    }

    let Some(snapshot) = orchestrator.workspace_view.step2.rescan_snapshot.take() else {
        return;
    };

    if orchestrator.wizard_state.step2.last_scan_report.is_none() {
        return;
    }

    reapply_snapshot(
        &snapshot.bgee,
        &mut orchestrator.wizard_state.step2.bgee_mods,
    );
    reapply_snapshot(
        &snapshot.bg2ee,
        &mut orchestrator.wizard_state.step2.bg2ee_mods,
    );

    if let Some(err) = crate::ui::step2::service_compat_rules_step2::apply_compat_rules(
        &orchestrator.wizard_state.step1,
        &mut orchestrator.wizard_state.step2.bgee_mods,
        &mut orchestrator.wizard_state.step2.bg2ee_mods,
    ) {
        orchestrator.wizard_state.step2.scan_status = format!("Compat rules load failed: {err}");
    }

    let mut unrestored =
        collect_unrestored(&snapshot.bgee, &orchestrator.wizard_state.step2.bgee_mods);
    unrestored.extend(collect_unrestored(
        &snapshot.bg2ee,
        &orchestrator.wizard_state.step2.bg2ee_mods,
    ));

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

    if unrestored.is_empty() {
        orchestrator.workspace_view.step2.rescan_drop_warning = None;
        return;
    }
    orchestrator.workspace_view.step2.rescan_drop_warning =
        Some(format_unrestored_status(&unrestored));
    orchestrator
        .notification_manager
        .warn_persistent(format_unrestored_notification(&unrestored));
}

struct UnrestoredComponent {
    mod_name: String,
    component_id: String,
    reason: String,
}

fn collect_unrestored(
    snapshot: &[RescanSelection],
    mods: &[Step2ModState],
) -> Vec<UnrestoredComponent> {
    let mut out = Vec::new();
    for entry in snapshot {
        let matched_mods = mods
            .iter()
            .filter(|mod_state| mod_state.tp_file.to_ascii_uppercase() == entry.tp2_upper)
            .collect::<Vec<_>>();
        let Some(first_mod) = matched_mods.first() else {
            out.push(UnrestoredComponent {
                mod_name: tp2_display_name(&entry.tp2_upper),
                component_id: entry.component_id.clone(),
                reason: "mod no longer present".to_string(),
            });
            continue;
        };
        let matched_components = matched_mods
            .iter()
            .flat_map(|mod_state| {
                mod_state
                    .components
                    .iter()
                    .map(move |component| (*mod_state, component))
            })
            .filter(|(_, component)| component.component_id == entry.component_id)
            .collect::<Vec<_>>();
        let Some(first_match) = matched_components.first().copied() else {
            out.push(UnrestoredComponent {
                mod_name: first_mod.name.clone(),
                component_id: entry.component_id.clone(),
                reason: "component no longer present".to_string(),
            });
            continue;
        };
        if matched_components
            .iter()
            .any(|(_, component)| component.checked)
        {
            continue;
        }
        let (reason_mod, reason_component) = matched_components
            .iter()
            .copied()
            .find(|(_, component)| has_disabled_reason(component))
            .unwrap_or(first_match);
        out.push(UnrestoredComponent {
            mod_name: reason_mod.name.clone(),
            component_id: entry.component_id.clone(),
            reason: exclusion_reason(reason_component),
        });
    }
    out
}

fn has_disabled_reason(component: &Step2ComponentState) -> bool {
    component
        .disabled_reason
        .as_deref()
        .map(str::trim)
        .is_some_and(|reason| !reason.is_empty())
}

fn exclusion_reason(component: &Step2ComponentState) -> String {
    component
        .disabled_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map_or_else(
            || "excluded by compatibility rules".to_string(),
            std::string::ToString::to_string,
        )
}

fn tp2_display_name(tp2_upper: &str) -> String {
    tp2_upper
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(tp2_upper)
        .to_string()
}

fn format_unrestored_status(unrestored: &[UnrestoredComponent]) -> String {
    let components = unrestored.len();
    let mods = unrestored
        .iter()
        .map(|entry| entry.mod_name.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    format!("{components} saved component(s) could not be restored \u{2014} {mods} mod(s) affected")
}

fn format_unrestored_notification(unrestored: &[UnrestoredComponent]) -> String {
    let mut lines = vec![format!(
        "{} saved component(s) could not be restored",
        unrestored.len()
    )];
    lines.extend(unrestored.iter().map(|entry| {
        format!(
            "{} #{}: {}",
            entry.mod_name, entry.component_id, entry.reason
        )
    }));
    lines.join("\n")
}

fn advance_pending_download_snapshot(
    orchestrator: &mut OrchestratorApp,
    was_scanning: bool,
    scanning_now: bool,
) {
    let pipeline_running = orchestrator
        .wizard_state
        .step2
        .update_selected_download_running
        || orchestrator
            .wizard_state
            .step2
            .update_selected_extract_running;
    advance_pending_download_snapshot_state(
        &mut orchestrator.workspace_view.step2,
        was_scanning,
        scanning_now,
        pipeline_running,
    );
}

fn advance_pending_download_snapshot_state(
    step2_view: &mut crate::ui::workspace::state_workspace::WorkspaceStep2State,
    was_scanning: bool,
    scanning_now: bool,
    pipeline_running: bool,
) {
    if step2_view.pending_update_download_snapshot.is_none() {
        return;
    }

    if !was_scanning && scanning_now {
        let snapshot = step2_view.pending_update_download_snapshot.take();
        step2_view.rescan_snapshot = snapshot;
        step2_view.rescan_drop_warning = None;
        step2_view.was_scanning = true;
        return;
    }

    if !scanning_now && !pipeline_running {
        step2_view.pending_update_download_snapshot = None;
    }
}

const fn completion_edge_fires(was_scanning: bool, scanning_now: bool) -> bool {
    was_scanning && !scanning_now
}

fn reapply_snapshot(snapshot: &[RescanSelection], mods: &mut [Step2ModState]) {
    for entry in snapshot {
        for mod_state in mods.iter_mut() {
            if mod_state.tp_file.to_ascii_uppercase() != entry.tp2_upper {
                continue;
            }
            for component in &mut mod_state.components {
                if component.component_id == entry.component_id {
                    component.checked = true;
                    component.selected_order = entry.selected_order;
                    if let Some(inputs) = entry.wlb_inputs.as_deref() {
                        reattach_wlb_inputs(component, inputs);
                    }
                }
            }
        }
    }
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
                wlb_inputs: None,
            },
            RescanSelection {
                tp2_upper: "GONEMOD.TP2".to_string(),
                component_id: "1".to_string(),
                selected_order: Some(1),
                wlb_inputs: None,
            },
        ];
        let mut mods = vec![mod_state(
            "eefixpack.tp2",
            vec![comp("0", false, None), comp("5", false, None)],
        )];
        reapply_snapshot(&snapshot, &mut mods);
        assert!(mods[0].components[0].checked);
        assert_eq!(mods[0].components[0].selected_order, Some(2));
        assert!(!mods[0].components[1].checked);
        let unrestored = collect_unrestored(&snapshot, &mods);
        assert_eq!(unrestored.len(), 1);
        assert_eq!(unrestored[0].component_id, "1");
        assert_eq!(unrestored[0].mod_name, "GONEMOD.TP2");
        assert_eq!(unrestored[0].reason, "mod no longer present");
    }

    #[test]
    fn unrestored_reports_component_excluded_by_compatibility_with_its_reason() {
        let snapshot = vec![RescanSelection {
            tp2_upper: "OLIRP.TP2".to_string(),
            component_id: "7".to_string(),
            selected_order: Some(1),
            wlb_inputs: None,
        }];
        let mut excluded = comp("7", false, None);
        excluded.disabled = true;
        excluded.disabled_reason = Some("Conditional on component #3".to_string());
        let mut named = mod_state("olirp.tp2", vec![excluded]);
        named.name = "Oli's Roleplay".to_string();
        let mods = vec![named];

        let unrestored = collect_unrestored(&snapshot, &mods);
        assert_eq!(unrestored.len(), 1);
        assert_eq!(unrestored[0].mod_name, "Oli's Roleplay");
        assert_eq!(unrestored[0].component_id, "7");
        assert_eq!(unrestored[0].reason, "Conditional on component #3");
        assert_eq!(
            format_unrestored_notification(&unrestored),
            "1 saved component(s) could not be restored\nOli's Roleplay #7: Conditional on component #3"
        );
    }

    #[test]
    fn unrestored_reason_comes_from_the_first_match_that_carries_one() {
        let snapshot = vec![RescanSelection {
            tp2_upper: "MOD.TP2".to_string(),
            component_id: "1".to_string(),
            selected_order: None,
            wlb_inputs: None,
        }];
        let mut with_reason = comp("1", false, None);
        with_reason.disabled_reason = Some("Blocked by rule".to_string());
        let mut second = mod_state("mod.tp2", vec![with_reason]);
        second.name = "Second Copy".to_string();
        let mods = vec![mod_state("mod.tp2", vec![comp("1", false, None)]), second];

        let unrestored = collect_unrestored(&snapshot, &mods);
        assert_eq!(unrestored.len(), 1);
        assert_eq!(unrestored[0].reason, "Blocked by rule");
        assert_eq!(unrestored[0].mod_name, "Second Copy");
    }

    #[test]
    fn unrestored_falls_back_to_generic_exclusion_reason() {
        let snapshot = vec![RescanSelection {
            tp2_upper: "MOD.TP2".to_string(),
            component_id: "1".to_string(),
            selected_order: None,
            wlb_inputs: None,
        }];
        let mods = vec![mod_state("mod.tp2", vec![comp("1", false, None)])];
        let unrestored = collect_unrestored(&snapshot, &mods);
        assert_eq!(unrestored.len(), 1);
        assert_eq!(unrestored[0].reason, "excluded by compatibility rules");
    }

    #[test]
    fn unrestored_distinguishes_missing_mod_from_missing_component() {
        let snapshot = vec![
            RescanSelection {
                tp2_upper: "GONE.TP2".to_string(),
                component_id: "0".to_string(),
                selected_order: None,
                wlb_inputs: None,
            },
            RescanSelection {
                tp2_upper: "MOD.TP2".to_string(),
                component_id: "9".to_string(),
                selected_order: None,
                wlb_inputs: None,
            },
        ];
        let mods = vec![mod_state("mod.tp2", vec![comp("1", true, Some(1))])];
        let unrestored = collect_unrestored(&snapshot, &mods);
        assert_eq!(unrestored.len(), 2);
        assert_eq!(unrestored[0].reason, "mod no longer present");
        assert_eq!(unrestored[1].reason, "component no longer present");
        assert_eq!(unrestored[1].mod_name, "mod.tp2");
    }

    #[test]
    fn unrestored_is_empty_when_every_saved_component_survives() {
        let snapshot = vec![RescanSelection {
            tp2_upper: "MOD.TP2".to_string(),
            component_id: "1".to_string(),
            selected_order: Some(1),
            wlb_inputs: None,
        }];
        let mut mods = vec![mod_state("mod.tp2", vec![comp("1", false, None)])];
        reapply_snapshot(&snapshot, &mut mods);
        assert!(collect_unrestored(&snapshot, &mods).is_empty());
    }

    #[test]
    fn unrestored_status_counts_all_reasons_and_distinct_mods() {
        let unrestored = vec![
            UnrestoredComponent {
                mod_name: "A".to_string(),
                component_id: "1".to_string(),
                reason: "mod no longer present".to_string(),
            },
            UnrestoredComponent {
                mod_name: "A".to_string(),
                component_id: "2".to_string(),
                reason: "component no longer present".to_string(),
            },
            UnrestoredComponent {
                mod_name: "B".to_string(),
                component_id: "3".to_string(),
                reason: "excluded by compatibility rules".to_string(),
            },
        ];
        assert_eq!(
            format_unrestored_status(&unrestored),
            "3 saved component(s) could not be restored \u{2014} 2 mod(s) affected"
        );
    }

    #[test]
    fn tp2_display_name_strips_the_path() {
        assert_eq!(tp2_display_name("BG1UB/BG1UB.TP2"), "BG1UB.TP2");
        assert_eq!(tp2_display_name("MOD.TP2"), "MOD.TP2");
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
                    wlb_inputs: None,
                },
                RescanSelection {
                    tp2_upper: "BG1UB/BG1UB.TP2".to_string(),
                    component_id: "0".to_string(),
                    selected_order: Some(2),
                    wlb_inputs: None,
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

        reapply_snapshot(&snapshot.bgee, &mut bgee_mods);
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

    #[test]
    fn wlb_persist2_capture_reapply_step3_round_trip() {
        use crate::app::state::Step2ComponentState;

        let raw = r"~MOD/MOD.TP2~ #0 #5 // Label // @wlb-inputs: y,D:\test1";

        let mods_with_marker = vec![mod_state(
            "MOD/MOD.TP2",
            vec![Step2ComponentState {
                component_id: "5".to_string(),
                label: "Label".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: raw.to_string(),
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
            }],
        )];

        let captured = capture_tab(&mods_with_marker);
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0].wlb_inputs.as_deref(),
            Some(r"y,D:\test1"),
            "capture_tab must extract the wlb_inputs marker"
        );

        let mut fresh_mods = vec![mod_state(
            "MOD/MOD.TP2",
            vec![Step2ComponentState {
                component_id: "5".to_string(),
                label: "Label".to_string(),
                weidu_group: None,
                collapsible_group: None,
                collapsible_group_is_umbrella: false,
                collapsible_group_combinable: false,
                raw_line: "~MOD/MOD.TP2~ #0 #5 // Label".to_string(),
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
                checked: false,
                selected_order: None,
            }],
        )];

        reapply_snapshot(&captured, &mut fresh_mods);
        assert!(
            collect_unrestored(&captured, &fresh_mods).is_empty(),
            "component must be found, not dropped"
        );
        assert!(
            fresh_mods[0].components[0]
                .raw_line
                .contains(r"@wlb-inputs: y,D:\test1"),
            "reapply_snapshot must re-attach the wlb_inputs marker; got: {}",
            fresh_mods[0].components[0].raw_line
        );

        let step3 = step3_sync::build_step3_items(&fresh_mods);
        let leaves: Vec<&crate::app::state::Step3ItemState> =
            step3.iter().filter(|i| !i.is_parent).collect();
        assert_eq!(leaves.len(), 1);
        assert!(
            leaves[0].raw_line.contains(r"@wlb-inputs: y,D:\test1"),
            "build_step3_items must carry the wlb_inputs marker into Step 3; got: {}",
            leaves[0].raw_line
        );
    }

    use crate::ui::workspace::state_workspace::WorkspaceStep2State;

    fn view_with_pending(bgee_tp2: &str, component_id: &str) -> WorkspaceStep2State {
        WorkspaceStep2State {
            pending_update_download_snapshot: Some(RescanSnapshot {
                bgee: vec![RescanSelection {
                    tp2_upper: bgee_tp2.to_ascii_uppercase(),
                    component_id: component_id.to_string(),
                    selected_order: Some(1),
                    wlb_inputs: None,
                }],
                bg2ee: Vec::new(),
            }),
            ..WorkspaceStep2State::default()
        }
    }

    #[test]
    fn pending_snapshot_transfers_on_scan_start_edge() {
        let mut view = view_with_pending("MOD/MOD.TP2", "0");
        assert!(view.rescan_snapshot.is_none(), "not yet transferred");
        assert!(!view.was_scanning, "initially false");

        advance_pending_download_snapshot_state(&mut view, false, true, false);

        assert!(
            view.pending_update_download_snapshot.is_none(),
            "consumed from pending"
        );
        assert!(
            view.rescan_snapshot.is_some(),
            "transferred to rescan_snapshot"
        );
        assert!(view.was_scanning, "armed so completion edge will fire");
    }

    #[test]
    fn pending_snapshot_not_consumed_while_pipeline_running() {
        let mut view = view_with_pending("MOD/MOD.TP2", "0");

        advance_pending_download_snapshot_state(&mut view, false, false, true);
        assert!(
            view.pending_update_download_snapshot.is_some(),
            "still held while pipeline active"
        );
    }

    #[test]
    fn pending_snapshot_cleared_when_pipeline_goes_idle_without_scan() {
        let mut view = view_with_pending("MOD/MOD.TP2", "0");

        advance_pending_download_snapshot_state(&mut view, false, false, false);

        assert!(
            view.pending_update_download_snapshot.is_none(),
            "cleared on idle pipeline (failure/cancel path)"
        );
        assert!(
            view.rescan_snapshot.is_none(),
            "not incorrectly transferred"
        );
    }

    #[test]
    fn pending_snapshot_not_premature_when_already_scanning() {
        let mut view = view_with_pending("MOD/MOD.TP2", "0");

        advance_pending_download_snapshot_state(&mut view, true, true, false);
        assert!(
            view.pending_update_download_snapshot.is_some(),
            "held while already scanning (not a new scan-start edge)"
        );

        advance_pending_download_snapshot_state(&mut view, true, false, false);
        assert!(
            view.pending_update_download_snapshot.is_none(),
            "idle after scan finished without transfer — cleared"
        );
        assert!(
            view.rescan_snapshot.is_none(),
            "not armed after scan-already-running path"
        );
    }

    #[test]
    fn pending_snapshot_eet_dual_tab_round_trip() {
        let mut view = WorkspaceStep2State {
            pending_update_download_snapshot: Some(RescanSnapshot {
                bgee: vec![RescanSelection {
                    tp2_upper: "BGEE_MOD.TP2".to_string(),
                    component_id: "1".to_string(),
                    selected_order: Some(1),
                    wlb_inputs: None,
                }],
                bg2ee: vec![RescanSelection {
                    tp2_upper: "BG2EE_MOD.TP2".to_string(),
                    component_id: "2".to_string(),
                    selected_order: Some(2),
                    wlb_inputs: None,
                }],
            }),
            ..WorkspaceStep2State::default()
        };

        advance_pending_download_snapshot_state(&mut view, false, true, false);

        let snap = view.rescan_snapshot.expect("transferred");
        assert_eq!(snap.bgee.len(), 1, "bgee tab preserved");
        assert_eq!(snap.bg2ee.len(), 1, "bg2ee tab preserved");
        assert_eq!(snap.bgee[0].component_id, "1");
        assert_eq!(snap.bg2ee[0].component_id, "2");
    }

    #[test]
    fn no_op_when_no_pending_snapshot() {
        let mut view = WorkspaceStep2State::default();
        advance_pending_download_snapshot_state(&mut view, false, true, false);
        assert!(view.rescan_snapshot.is_none());
        assert!(!view.was_scanning);
    }
}
