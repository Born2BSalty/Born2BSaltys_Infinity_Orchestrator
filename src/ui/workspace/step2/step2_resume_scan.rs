// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step2::step2_rescan_reconcile;

pub fn maybe_trigger_resume_scan(
    orchestrator: &mut OrchestratorApp,
    workspace: &ModlistWorkspaceState,
) {
    if !orchestrator.dev_mode {
        return;
    }
    let Some(folder) = workspace
        .dev_scanned_mods_folder
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
    else {
        return;
    };
    let has_order = !workspace.order_bgee.is_empty()
        || !workspace.order_bg2ee.is_empty()
        || !workspace.order_iwdee.is_empty();
    if !has_order {
        return;
    }
    let step2_empty = orchestrator.wizard_state.step2.bgee_mods.is_empty()
        && orchestrator.wizard_state.step2.bg2ee_mods.is_empty();
    if !step2_empty {
        return;
    }
    if orchestrator.wizard_state.step2.is_scanning {
        return;
    }

    orchestrator.wizard_state.step1.mods_folder = folder.to_string();

    let mut bgee = snapshot_from_order(&workspace.order_bgee);
    if !workspace.order_iwdee.is_empty() {
        let base = bgee.len();
        for (i, sel) in snapshot_from_order(&workspace.order_iwdee)
            .into_iter()
            .enumerate()
        {
            bgee.push(RescanSelection {
                selected_order: Some(base + i + 1),
                ..sel
            });
        }
    }
    let snapshot = RescanSnapshot {
        bgee,
        bg2ee: snapshot_from_order(&workspace.order_bg2ee),
    };

    let step2 = &mut orchestrator.workspace_view.step2;
    step2.rescan_snapshot = Some(snapshot);
    step2.rescan_drop_warning = None;
    step2.resume_pending = true;

    step_action_dispatch::dispatch_step2(Step2Action::StartScan, orchestrator);

    orchestrator.workspace_view.step2.was_scanning =
        step2_rescan_reconcile::armed_was_scanning_for_inflight_scan();
}

fn snapshot_from_order(order: &[ComponentRef]) -> Vec<RescanSelection> {
    order
        .iter()
        .enumerate()
        .map(|(i, c)| RescanSelection {
            tp2_upper: c.tp2.to_ascii_uppercase(),
            component_id: c.id.to_string(),
            selected_order: Some(i + 1),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_from_order_is_1_based_and_upper_tp2() {
        let order = vec![
            ComponentRef {
                tp2: "BG1UB/SETUP-BG1UB.TP2".to_string(),
                id: 0,
                language: 0,
            },
            ComponentRef {
                tp2: "EeFixPack/EeFixPack.TP2".to_string(),
                id: 2,
                language: 0,
            },
        ];
        let snap = snapshot_from_order(&order);
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].tp2_upper, "BG1UB/SETUP-BG1UB.TP2");
        assert_eq!(snap[0].component_id, "0");
        assert_eq!(snap[0].selected_order, Some(1));
        assert_eq!(snap[1].tp2_upper, "EEFIXPACK/EEFIXPACK.TP2");
        assert_eq!(snap[1].component_id, "2");
        assert_eq!(snap[1].selected_order, Some(2));
    }

    #[test]
    fn empty_order_yields_empty_snapshot() {
        assert!(snapshot_from_order(&[]).is_empty());
    }

    use crate::app::controller::step3_sync;
    use crate::app::state::{Step2ComponentState, Step2ModState, Step3ItemState, WizardState};
    use crate::registry::model::{Game, ModlistEntry, ModlistState};
    use crate::settings::store::SettingsStore;
    use crate::ui::workspace::workspace_state_loader;

    fn comp(id: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: format!("Component {id}"),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: format!("~BG1UB/BG1UB.TP2~ #0 #{id} // Component {id}"),
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
        }
    }

    fn mod_state(name: &str, tp: &str, comps: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: tp.to_string(),
            tp2_path: format!("/mods/{tp}"),
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
    fn cold_resume_restores_step2_and_step3_after_scan_lands() {
        let mut ws = WizardState::default();
        let workspace = ModlistWorkspaceState {
            order_bgee: vec![
                ComponentRef {
                    tp2: "BG1UB/BG1UB.TP2".to_string(),
                    id: 11,
                    language: 0,
                },
                ComponentRef {
                    tp2: "BG1UB/BG1UB.TP2".to_string(),
                    id: 0,
                    language: 0,
                },
            ],
            dev_scanned_mods_folder: Some("/some/scanned/mods".to_string()),
            ..Default::default()
        };
        let entry = ModlistEntry {
            id: "RESUMETEST00".to_string(),
            name: "Resume".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            ..Default::default()
        };

        workspace_state_loader::populate_wizard_state_from_workspace(
            &workspace,
            &entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        assert!(
            ws.step2.bgee_mods.is_empty(),
            "cold resume: no scanned mods yet"
        );
        assert!(ws.step3.bgee_items.is_empty(), "cold resume: Step 3 empty");

        let snapshot = snapshot_from_order(&workspace.order_bgee);
        assert_eq!(snapshot.len(), 2);

        ws.step2.bgee_mods = vec![mod_state(
            "BG1UB",
            "BG1UB/BG1UB.TP2",
            vec![comp("0"), comp("11"), comp("5")],
        )];

        for sel in &snapshot {
            for m in &mut ws.step2.bgee_mods {
                if m.tp_file.to_ascii_uppercase() != sel.tp2_upper {
                    continue;
                }
                for c in &mut m.components {
                    if c.component_id == sel.component_id {
                        c.checked = true;
                        c.selected_order = sel.selected_order;
                    }
                }
            }
        }
        for m in &mut ws.step2.bgee_mods {
            m.checked = m.components.iter().any(|c| c.checked);
        }
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);

        let m = &ws.step2.bgee_mods[0];
        assert!(m.components[0].checked, "#0 restored checked");
        assert_eq!(m.components[0].selected_order, Some(2));
        assert!(m.components[1].checked, "#11 restored checked");
        assert_eq!(m.components[1].selected_order, Some(1));
        assert!(!m.components[2].checked, "#5 was never selected");
        assert!(m.checked, "mod tri-state re-derived");

        let leaves: Vec<&Step3ItemState> = ws
            .step3
            .bgee_items
            .iter()
            .filter(|i| !i.is_parent)
            .collect();
        assert_eq!(leaves.len(), 2, "two component rows restored");
        assert_eq!(leaves[0].component_id, "11");
        assert_eq!(leaves[1].component_id, "0");
        assert!(
            ws.step3.bgee_items.iter().any(|i| i.is_parent),
            "BIO's build_step3_items emitted the parent header row"
        );
    }
}
