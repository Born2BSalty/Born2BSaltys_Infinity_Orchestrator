use super::*;
use crate::app::state::{Step2ComponentState, Step2ModState};
use crate::registry::workspace_model::{
    ComponentRef, ModlistWorkspaceState, SelectionRecovery, UnresolvedSelection,
};

fn comp(id: &str, checked: bool, order: Option<usize>) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: String::new(),
        weidu_group: None,
        collapsible_group: None,
        collapsible_group_is_umbrella: false,
        collapsible_group_combinable: false,
        raw_line: format!("~FIXTURE/FIXTURE.TP2~ #0 #{id} // Fixture"),
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

fn mod_state(tp: &str, mut comps: Vec<Step2ComponentState>) -> Step2ModState {
    for component in &mut comps {
        if component.raw_line.starts_with("~FIXTURE/FIXTURE.TP2~") {
            component.raw_line = format!("~{tp}~ #0 #{} // Fixture", component.component_id);
        }
    }
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
fn language_recovery_actual_autosave_and_nav_away_keep_unresolved_with_new_edits() {
    let _ambient_lock = crate::app::mod_downloads::AMBIENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for autosave in [false, true] {
        let mut app = OrchestratorApp::new_isolated_for_test("language-recovery-routes");
        app.workspace_view.modlist_id = "RECOVERYROUTES".to_string();
        app.workspace_view.loaded_workspace_id = Some("RECOVERYROUTES".to_string());
        app.wizard_state.step1.game_install = "BGEE".to_string();
        let original = ComponentRef {
            tp2: "TEST/TEST.TP2".to_string(),
            id: 11,
            language: 0,
            wlb_inputs: Some("unchanged answers".to_string()),
        };
        let workspace = ModlistWorkspaceState {
            order_bgee: vec![original.clone()],
            selection_recovery: HashMap::from([(
                "BGEE".to_string(),
                SelectionRecovery {
                    unresolved: vec![UnresolvedSelection {
                        original: original.clone(),
                        position: 0,
                    }],
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        app.workspace_state
            .insert("RECOVERYROUTES".to_string(), workspace);
        let store = crate::registry::store_workspace::WorkspaceStore::new_with_path(
            std::env::temp_dir().join(format!(
                "recovery-routes-{}-{autosave}.json",
                std::process::id()
            )),
        );
        app.workspace_stores
            .insert("RECOVERYROUTES".to_string(), store.clone());
        app.wizard_state.step2.bgee_mods =
            vec![mod_state("TEST/TEST.TP2", vec![comp("1", true, Some(1))])];
        app.wizard_state.parser_choice_phrases = vec!["unrelated edit".to_string()];
        if autosave {
            app.mark_workspace_dirty();
            app.persistence_cycle.debounce_ms = 0;
            app.sync_active_workspace_if_dirty();
            app.tick_persistence();
        } else {
            app.nav = crate::ui::orchestrator::nav_destination::NavDestination::Workspace {
                modlist_id: None,
            };
            let ctx = egui::Context::default();
            crate::ui::shared::redesign_fonts::install_redesign_fonts(&ctx);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    crate::ui::orchestrator::page_router::render(ui, &mut app, ctx);
                });
            });
            assert!(app.workspace_view.loaded_workspace_id.is_none());
        }
        let loaded = store.load().unwrap();
        assert_eq!(loaded.order_bgee.len(), 2);
        assert_eq!(loaded.order_bgee[0], original);
        assert_eq!(loaded.order_bgee[1].id, 1);
        assert_eq!(
            loaded.parser_choice_phrases,
            Some(vec!["unrelated edit".to_string()])
        );
        assert_eq!(
            loaded.selection_recovery["BGEE"].unresolved[0].original,
            original
        );
    }
}
