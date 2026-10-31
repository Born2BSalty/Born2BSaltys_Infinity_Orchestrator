use crate::app::state::{Step3ItemState, WizardState};
use crate::registry::store_workspace::WorkspaceStore;
use crate::registry::workspace_model::ModlistWorkspaceState;
use crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard;

#[test]
fn manual_wlb_workspace_extraction_persists_actual_language_and_iwdee_carrier() {
    let mut state = WizardState::default();
    state.step1.game_install = "IWDEE".to_string();
    state.parser_choice_phrases = vec!["Prior parser phrase".to_string()];
    state.step3.bgee_items.push(Step3ItemState {
        tp_file: "MOD/MOD.TP2".to_string(),
        component_id: "17".to_string(),
        mod_name: "Mod".to_string(),
        component_label: "Label".to_string(),
        raw_line: "~MOD/MOD.TP2~ #2 #17 // Label // @wlb-inputs: bio-json-v1:[\"\",\"a,b\"]"
            .to_string(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: "MOD".to_string(),
        is_parent: false,
        parent_placeholder: false,
    });
    let workspace = extract_workspace_state_from_wizard(&state, &ModlistWorkspaceState::default());
    assert_eq!(workspace.order_iwdee.len(), 1);
    assert_eq!(workspace.order_iwdee[0].language, 2);
    assert_eq!(
        workspace.order_iwdee[0].wlb_inputs.as_deref(),
        Some("bio-json-v1:[\"\",\"a,b\"]")
    );
    let root = std::env::temp_dir().join(format!(
        "bio-manual-wlb-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let store = WorkspaceStore::new_with_path(root.join("workspace.json"));
    store.save(&workspace).expect("isolated save");
    assert_eq!(store.load().expect("reopen"), workspace);
    std::fs::remove_file(store.path()).expect("remove fixture file");
    std::fs::remove_dir(root).expect("remove empty fixture directory");
}
