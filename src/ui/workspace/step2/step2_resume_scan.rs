// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState, ModsSource};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::step2::action_step2::Step2Action;
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};
use crate::ui::workspace::step_action_dispatch;
use crate::ui::workspace::step2::step2_rescan_reconcile;

pub fn maybe_trigger_resume_scan(
    orchestrator: &mut OrchestratorApp,
    workspace: &ModlistWorkspaceState,
) {
    let global_folder = if workspace.mods_source == ModsSource::GlobalModsFolder {
        orchestrator
            .settings_store
            .load()
            .ok()
            .map(|s| s.step1.effective_global_mods_folder().to_string())
    } else {
        None
    };
    let step2_empty = orchestrator.wizard_state.step2.bgee_mods.is_empty()
        && orchestrator.wizard_state.step2.bg2ee_mods.is_empty();
    let Some(folder) = resume_scan_folder(
        workspace,
        global_folder.as_deref(),
        orchestrator.dev_mode,
        step2_empty,
        orchestrator.wizard_state.step2.is_scanning,
    ) else {
        return;
    };

    orchestrator.wizard_state.step1.mods_folder = folder;

    let first_order = if orchestrator.wizard_state.step1.game_install == "IWDEE" {
        &workspace.order_iwdee
    } else {
        &workspace.order_bgee
    };
    let snapshot = RescanSnapshot {
        bgee: snapshot_from_order(first_order),
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

pub(crate) fn resume_scan_folder(
    workspace: &ModlistWorkspaceState,
    global_folder: Option<&str>,
    dev_mode: bool,
    step2_empty: bool,
    is_scanning: bool,
) -> Option<String> {
    if !step2_empty || is_scanning {
        return None;
    }
    choose_resume_folder(
        workspace.mods_source,
        global_folder,
        pick_resume_folder(dev_mode, workspace),
    )
    .map(str::to_string)
}

fn choose_resume_folder<'a>(
    mods_source: ModsSource,
    global_folder: Option<&'a str>,
    installation_folder: Option<&'a str>,
) -> Option<&'a str> {
    match mods_source {
        ModsSource::GlobalModsFolder => global_folder
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .or(installation_folder),
        ModsSource::InstallationFolder => installation_folder,
    }
}

fn pick_resume_folder(dev_mode: bool, workspace: &ModlistWorkspaceState) -> Option<&str> {
    let scratch = workspace
        .scratch_mods_folder
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty());
    if let Some(folder) = scratch {
        return Some(folder);
    }
    if !dev_mode {
        return None;
    }
    workspace
        .dev_scanned_mods_folder
        .as_deref()
        .map(str::trim)
        .filter(|f| !f.is_empty())
}

fn snapshot_from_order(order: &[ComponentRef]) -> Vec<RescanSelection> {
    order
        .iter()
        .enumerate()
        .map(|(i, c)| RescanSelection {
            tp2_upper: c.tp2.to_ascii_uppercase(),
            component_id: c.id.to_string(),
            language: c.language,
            selected_order: Some(i + 1),
            wlb_inputs: c.wlb_inputs.clone(),
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
                wlb_inputs: None,
            },
            ComponentRef {
                tp2: "EeFixPack/EeFixPack.TP2".to_string(),
                id: 2,
                language: 0,
                wlb_inputs: None,
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
        assert_eq!(snapshot_from_order(&[]).len(), 0);
    }

    #[test]
    fn pick_resume_folder_prefers_scratch_in_any_mode() {
        let ws = ModlistWorkspaceState {
            scratch_mods_folder: Some(r"D:\fork\mods".to_string()),
            dev_scanned_mods_folder: Some(r"D:\dev\corpus".to_string()),
            ..Default::default()
        };
        assert_eq!(pick_resume_folder(false, &ws), Some(r"D:\fork\mods"));
        assert_eq!(pick_resume_folder(true, &ws), Some(r"D:\fork\mods"));
    }

    #[test]
    fn pick_resume_folder_uses_scratch_when_only_scratch_set() {
        let ws = ModlistWorkspaceState {
            scratch_mods_folder: Some(r"D:\fork\mods".to_string()),
            ..Default::default()
        };
        assert_eq!(pick_resume_folder(false, &ws), Some(r"D:\fork\mods"));
    }

    #[test]
    fn pick_resume_folder_falls_back_to_dev_only_in_dev_mode() {
        let ws = ModlistWorkspaceState {
            dev_scanned_mods_folder: Some(r"D:\dev\corpus".to_string()),
            ..Default::default()
        };
        assert_eq!(pick_resume_folder(true, &ws), Some(r"D:\dev\corpus"));
        assert_eq!(
            pick_resume_folder(false, &ws),
            None,
            "non-dev sessions never resume from a dev-only scan folder"
        );
    }

    #[test]
    fn pick_resume_folder_ignores_whitespace_only_folders() {
        let ws = ModlistWorkspaceState {
            scratch_mods_folder: Some("   ".to_string()),
            dev_scanned_mods_folder: Some("\t".to_string()),
            ..Default::default()
        };
        assert_eq!(pick_resume_folder(true, &ws), None);
        assert_eq!(pick_resume_folder(false, &ws), None);
    }

    #[test]
    fn pick_resume_folder_returns_none_when_neither_field_set() {
        let ws = ModlistWorkspaceState::default();
        assert_eq!(pick_resume_folder(true, &ws), None);
        assert_eq!(pick_resume_folder(false, &ws), None);
    }

    #[test]
    fn choose_resume_global_uses_global_folder() {
        assert_eq!(
            choose_resume_folder(
                ModsSource::GlobalModsFolder,
                Some(r"D:\global\mods"),
                Some(r"D:\install\mods"),
            ),
            Some(r"D:\global\mods"),
            "global source must resume from the settings mods folder, not the installation folder"
        );
    }

    #[test]
    fn choose_resume_global_falls_back_to_installation_when_global_empty() {
        assert_eq!(
            choose_resume_folder(
                ModsSource::GlobalModsFolder,
                Some("   "),
                Some(r"D:\install")
            ),
            Some(r"D:\install"),
        );
        assert_eq!(
            choose_resume_folder(ModsSource::GlobalModsFolder, None, Some(r"D:\install")),
            Some(r"D:\install"),
        );
    }

    #[test]
    fn choose_resume_installation_ignores_global_folder() {
        assert_eq!(
            choose_resume_folder(
                ModsSource::InstallationFolder,
                Some(r"D:\global\mods"),
                Some(r"D:\install\mods"),
            ),
            Some(r"D:\install\mods"),
        );
    }

    #[test]
    fn choose_resume_none_when_nothing_available() {
        assert_eq!(
            choose_resume_folder(ModsSource::GlobalModsFolder, None, None),
            None
        );
        assert_eq!(
            choose_resume_folder(ModsSource::InstallationFolder, Some(r"D:\global"), None),
            None
        );
    }

    fn fresh_list_with_scratch() -> ModlistWorkspaceState {
        ModlistWorkspaceState {
            mods_source: ModsSource::InstallationFolder,
            scratch_mods_folder: Some(r"D:\install\mods".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn resume_scans_a_list_with_no_saved_order() {
        let ws = fresh_list_with_scratch();
        assert!(ws.order_bgee.is_empty() && ws.order_bg2ee.is_empty() && ws.order_iwdee.is_empty());
        assert_eq!(
            resume_scan_folder(&ws, None, false, true, false),
            Some(r"D:\install\mods".to_string())
        );
    }

    #[test]
    fn resume_skips_when_the_tree_is_not_empty() {
        let ws = fresh_list_with_scratch();
        assert_eq!(resume_scan_folder(&ws, None, false, false, false), None);
    }

    #[test]
    fn resume_skips_while_a_scan_runs() {
        let ws = fresh_list_with_scratch();
        assert_eq!(resume_scan_folder(&ws, None, false, true, true), None);
    }

    #[test]
    fn resume_prefers_the_global_folder_for_a_global_list() {
        let ws = ModlistWorkspaceState {
            mods_source: ModsSource::GlobalModsFolder,
            scratch_mods_folder: Some(r"D:\install\mods".to_string()),
            ..Default::default()
        };
        assert_eq!(
            resume_scan_folder(&ws, Some(r"D:\global\mods"), false, true, false),
            Some(r"D:\global\mods".to_string())
        );
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
            collapsible_group_combinable: false,
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
    fn wlb_persist2_snapshot_from_order_carries_wlb_inputs() {
        let order = vec![
            ComponentRef {
                tp2: "MOD/MOD.TP2".to_string(),
                id: 5,
                language: 0,
                wlb_inputs: Some(r"y,D:\test1".to_string()),
            },
            ComponentRef {
                tp2: "OTHER/OTHER.TP2".to_string(),
                id: 0,
                language: 0,
                wlb_inputs: None,
            },
        ];
        let snap = snapshot_from_order(&order);
        assert_eq!(snap.len(), 2);
        assert_eq!(
            snap[0].wlb_inputs.as_deref(),
            Some(r"y,D:\test1"),
            "snapshot_from_order must carry wlb_inputs from ComponentRef"
        );
        assert_eq!(
            snap[1].wlb_inputs, None,
            "snapshot_from_order must preserve None when wlb_inputs is absent"
        );
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
                    wlb_inputs: None,
                },
                ComponentRef {
                    tp2: "BG1UB/BG1UB.TP2".to_string(),
                    id: 0,
                    language: 0,
                    wlb_inputs: None,
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
        assert_eq!(ws.step3.bgee_items.len(), 0, "cold resume: Step 3 empty");

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

    #[test]
    fn manual_wlb_resume_iwdee_uses_only_canonical_order_without_duplicates() {
        for canonical in [false, true] {
            let mut app = OrchestratorApp::new_isolated_for_test(if canonical {
                "manual-wlb-iwdee-canonical"
            } else {
                "manual-wlb-iwdee-fallback"
            });
            app.wizard_state.step1.game_install = "IWDEE".to_string();
            let legacy = ComponentRef {
                tp2: "OLD/OLD.TP2".to_string(),
                id: 1,
                language: 0,
                wlb_inputs: Some("old".to_string()),
            };
            let current = ComponentRef {
                tp2: "MOD/MOD.TP2".to_string(),
                id: 17,
                language: 2,
                wlb_inputs: Some("bio-json-v1:[\"a,b\",\"\"]".to_string()),
            };
            let workspace = ModlistWorkspaceState {
                mods_source: ModsSource::InstallationFolder,
                scratch_mods_folder: Some(
                    std::env::temp_dir()
                        .join(format!(
                            "bio-wlb-no-scan-{}-{canonical}",
                            std::process::id()
                        ))
                        .to_string_lossy()
                        .into_owned(),
                ),
                order_bgee: vec![legacy.clone()],
                order_iwdee: if canonical {
                    vec![current.clone()]
                } else {
                    vec![]
                },
                ..Default::default()
            };
            maybe_trigger_resume_scan(&mut app, &workspace);
            let snapshot = app
                .workspace_view
                .step2
                .rescan_snapshot
                .as_ref()
                .expect("armed snapshot");
            assert_eq!(snapshot.bgee.len(), usize::from(canonical));
            if canonical {
                assert_eq!(snapshot.bgee[0].tp2_upper, current.tp2);
                assert_eq!(snapshot.bgee[0].language, current.language);
                assert_eq!(snapshot.bgee[0].wlb_inputs, current.wlb_inputs);
            }
            assert_eq!(
                snapshot.bg2ee,
                [] as [crate::ui::workspace::state_workspace::RescanSelection; 0]
            );
        }
    }

    fn language_cycle_mods(selected: bool) -> Vec<Step2ModState> {
        let components = [11, 0, 5]
            .into_iter()
            .enumerate()
            .map(|(index, id)| {
                let mut component = comp(&id.to_string());
                component.raw_line =
                    format!("~ARESTORATIONP/ARESTORATIONP.TP2~ #1 #{id} // Component {id}");
                component.checked = selected && index < 2;
                component.selected_order = component.checked.then_some(index + 1);
                if selected && index == 0 {
                    component.raw_line.push_str(" // @wlb-inputs: yes");
                }
                component
            })
            .collect();
        vec![mod_state(
            "arestorationp",
            "ARESTORATIONP/ARESTORATIONP.TP2",
            components,
        )]
    }

    fn language_cycle_app(tag: &str) -> OrchestratorApp {
        let mut app = OrchestratorApp::new_isolated_for_test(tag);
        app.workspace_view.modlist_id = "LANGUAGECYCLE".to_string();
        app.workspace_view.loaded_workspace_id = Some("LANGUAGECYCLE".to_string());
        app.wizard_state.step1.game_install = "BGEE".to_string();
        app
    }

    fn language_cycle_reopen(
        store: &crate::registry::store_workspace::WorkspaceStore,
        tag: &str,
    ) -> OrchestratorApp {
        let mut app = language_cycle_app(tag);
        let saved = store.load().expect("load persisted workspace");
        let entry = ModlistEntry {
            id: "LANGUAGECYCLE".to_string(),
            game: Game::BGEE,
            ..Default::default()
        };
        workspace_state_loader::populate_wizard_state_from_workspace(
            &saved,
            &entry,
            &app.settings_store,
            &mut app.wizard_state,
        );
        assert_eq!(
            app.wizard_state.step2.bgee_mods,
            Vec::<Step2ModState>::new()
        );
        assert_eq!(
            app.wizard_state.step3.bgee_items,
            Vec::<Step3ItemState>::new()
        );
        app.workspace_view.step2.rescan_snapshot = Some(RescanSnapshot {
            bgee: snapshot_from_order(&saved.order_bgee),
            bg2ee: snapshot_from_order(&saved.order_bg2ee),
        });
        app.workspace_view.step2.resume_pending = true;
        app.workspace_view.step2.was_scanning = true;
        app.workspace_state.insert(entry.id.clone(), saved);
        app.workspace_stores.insert(entry.id, store.clone());
        app.wizard_state.step2.bgee_mods = language_cycle_mods(false);
        app.wizard_state.step2.last_scan_report =
            Some(crate::app::state::Step2ScanReport::default());
        assert!(step2_rescan_reconcile::reconcile_on_scan_complete(&mut app));
        assert!(!app.workspace_view.step2.resume_pending);
        assert!(app.workspace_view.step2.rescan_snapshot.is_none());
        app
    }

    fn language_cycle_saved_store(tag: &str) -> crate::registry::store_workspace::WorkspaceStore {
        let mut app = language_cycle_app(tag);
        app.wizard_state.step2.bgee_mods = language_cycle_mods(true);
        crate::ui::workspace::workspace_header::save_draft(&mut app);
        let store = app.workspace_stores["LANGUAGECYCLE"].clone();
        let saved = store.load().expect("save draft wrote workspace");
        assert_eq!(saved.order_bgee.len(), 2);
        assert_eq!(saved.order_bgee[0].language, 1);
        assert_eq!(saved.order_bgee[1].language, 1);
        assert_eq!(saved.order_bgee[0].wlb_inputs.as_deref(), Some("yes"));
        let durable = crate::registry::store_workspace::WorkspaceStore::new_with_path(
            std::env::temp_dir().join(format!("{tag}-{}-workspace.json", std::process::id())),
        );
        app.workspace_stores
            .insert("LANGUAGECYCLE".to_string(), durable.clone());
        app.persistence_cycle.last_saved_workspaces.clear();
        eframe::App::on_exit(&mut app, None);
        assert_eq!(durable.load().expect("normal exit flushed"), saved);
        durable
    }

    #[test]
    fn language_cycle_current_one_survives_save_exit_and_cold_reconcile() {
        let store = language_cycle_saved_store("language-cycle-current");
        let mut app = language_cycle_reopen(&store, "language-cycle-current-reopen");
        let components = &app.wizard_state.step2.bgee_mods[0].components;
        assert!(components[0].checked);
        assert!(components[1].checked);
        assert!(!components[2].checked);
        assert_eq!(components[0].selected_order, Some(1));
        assert_eq!(components[1].selected_order, Some(2));
        assert!(components[0].raw_line.contains("@wlb-inputs: yes"));
        assert!(app.workspace_view.step2.rescan_drop_warning.is_none());
        let before = store.load().expect("before second save");
        crate::ui::workspace::workspace_header::save_draft(&mut app);
        eframe::App::on_exit(&mut app, None);
        assert_eq!(store.load().expect("second save"), before);
    }

    #[test]
    fn language_cycle_legacy_zero_mismatch_survives_partial_restore_save() {
        let store = language_cycle_saved_store("language-cycle-legacy");
        let mut legacy = store.load().expect("current workspace");
        legacy.order_bgee[0].language = 0;
        store.save(&legacy).expect("legacy zero writer fixture");
        let mut app = language_cycle_reopen(&store, "language-cycle-legacy-reopen");
        let components = &app.wizard_state.step2.bgee_mods[0].components;
        assert!(!components[0].checked);
        assert!(components[1].checked);
        assert!(!components[2].checked);
        assert_eq!(components[1].selected_order, Some(2));
        assert!(app.workspace_view.step2.rescan_drop_warning.is_some());
        assert_eq!(store.load().expect("before later save"), legacy);
        crate::ui::workspace::workspace_header::save_draft(&mut app);
        eframe::App::on_exit(&mut app, None);
        let after = store.load().expect("after later save");
        assert_eq!(after.order_bgee, legacy.order_bgee);
        let reopened = language_cycle_reopen(&store, "language-cycle-legacy-reopen-again");
        assert!(reopened.workspace_view.step2.rescan_drop_warning.is_some());
        assert!(!reopened.wizard_state.step2.bgee_mods[0].components[0].checked);
        assert!(reopened.wizard_state.step2.bgee_mods[0].components[1].checked);
    }
}
