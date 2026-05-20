// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::controller::step3_sync;
use crate::app::state::{Step2ModState, Step3ItemState, WizardState};
use crate::registry::model::{Game, ModlistEntry};
use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use crate::settings::store::SettingsStore;

const PARENT_PLACEHOLDER_ID: &str = "__PARENT__";

pub fn populate_wizard_state_from_workspace(
    workspace: &ModlistWorkspaceState,
    entry: &ModlistEntry,
    settings_store: &SettingsStore,
    wizard_state: &mut WizardState,
) {
    wizard_state.step1.game_install = entry.game.to_legacy_string().to_string();

    sync_paths_from_settings(settings_store, wizard_state);

    reset_scanned_step2_set(wizard_state);

    apply_order_to_mods(&workspace.order_bgee, &mut wizard_state.step2.bgee_mods);
    apply_order_to_mods(&workspace.order_bg2ee, &mut wizard_state.step2.bg2ee_mods);
    if entry.game == Game::IWDEE {
        apply_order_to_mods(&workspace.order_iwdee, &mut wizard_state.step2.bgee_mods);
    }
    recompute_mod_checked(&mut wizard_state.step2.bgee_mods);
    recompute_mod_checked(&mut wizard_state.step2.bg2ee_mods);

    let max_order = max_selected_order(&wizard_state.step2.bgee_mods)
        .max(max_selected_order(&wizard_state.step2.bg2ee_mods));
    wizard_state.step2.next_selection_order = max_order + 1;

    wizard_state.step3.bgee_items = step3_sync::build_step3_items(&wizard_state.step2.bgee_mods);
    wizard_state.step3.bg2ee_items = step3_sync::build_step3_items(&wizard_state.step2.bg2ee_mods);

    wizard_state.step3.bgee_collapsed_blocks = collapsed_blocks_for_tab(workspace, "BGEE");
    wizard_state.step3.bg2ee_collapsed_blocks = collapsed_blocks_for_tab(workspace, "BG2EE");

    wizard_state.step3.bgee_undo_stack.clear();
    wizard_state.step3.bg2ee_undo_stack.clear();
    wizard_state.step3.bgee_redo_stack.clear();
    wizard_state.step3.bg2ee_redo_stack.clear();
    wizard_state.step3.bgee_selected.clear();
    wizard_state.step3.bg2ee_selected.clear();
    wizard_state.step3.bgee_anchor = None;
    wizard_state.step3.bg2ee_anchor = None;

    wizard_state.step5 = crate::app::state::Step5State::default();
}

fn reset_scanned_step2_set(wizard_state: &mut WizardState) {
    wizard_state.step2.bgee_mods.clear();
    wizard_state.step2.bg2ee_mods.clear();
    wizard_state.step3.bgee_items.clear();
    wizard_state.step3.bg2ee_items.clear();
    wizard_state.step2.selected = None;
    wizard_state.step2.next_selection_order = 1;
    wizard_state.step2.selected_count = 0;
    wizard_state.step2.scan_status.clear();
    wizard_state.step2.last_scan_report = None;
    wizard_state.step2.is_scanning = false;
}

fn apply_order_to_mods(order: &[ComponentRef], mods: &mut [Step2ModState]) {
    for mod_state in mods.iter_mut() {
        for component in &mut mod_state.components {
            component.checked = false;
            component.selected_order = None;
        }
    }
    for (position, item) in order.iter().enumerate() {
        let want_tp2 = item.tp2.to_ascii_uppercase();
        let want_id = item.id.to_string();
        for mod_state in mods.iter_mut() {
            if mod_state.tp_file.to_ascii_uppercase() != want_tp2 {
                continue;
            }
            for component in &mut mod_state.components {
                if component.component_id == want_id {
                    component.checked = true;
                    component.selected_order = Some(position + 1);
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

fn collapsed_blocks_for_tab(workspace: &ModlistWorkspaceState, tab: &str) -> Vec<String> {
    let prefix = format!("{tab}::");
    let mut out: Vec<String> = workspace
        .step3_group_collapse
        .iter()
        .filter(|(_, collapsed)| **collapsed)
        .filter_map(|(key, _)| key.strip_prefix(&prefix).map(ToString::to_string))
        .collect();
    out.sort();
    out
}

pub fn sync_step3_from_step2_if_changed(wizard_state: &mut WizardState) {
    use crate::app::app_nav::{NextAction, decide_next_action};
    use crate::app::app_step3_sync_flow::sync_step3_from_step2;

    let saved_step = wizard_state.current_step;
    wizard_state.current_step = 1;
    let action = decide_next_action(wizard_state);
    wizard_state.current_step = saved_step;

    if let NextAction::SyncStep3AndAdvance { signature } = action {
        sync_step3_from_step2(wizard_state);
        wizard_state.set_last_step2_sync_signature(signature);
    }
}

#[must_use]
pub fn extract_workspace_state_from_wizard(
    wizard_state: &WizardState,
    prior: &ModlistWorkspaceState,
) -> ModlistWorkspaceState {
    let first_order = order_for_tab(
        order_from_items(&wizard_state.step3.bgee_items),
        &prior.order_bgee,
        wizard_state.step2.bgee_mods.is_empty(),
    );
    let second_order = order_for_tab(
        order_from_items(&wizard_state.step3.bg2ee_items),
        &prior.order_bg2ee,
        wizard_state.step2.bg2ee_mods.is_empty(),
    );

    let order_iwdee = if wizard_state.step1.game_install == "IWDEE" {
        first_order.clone()
    } else {
        prior.order_iwdee.clone()
    };

    let mut step3_group_collapse = prior.step3_group_collapse.clone();
    write_collapsed_blocks(
        &mut step3_group_collapse,
        "BGEE",
        &wizard_state.step3.bgee_collapsed_blocks,
    );
    write_collapsed_blocks(
        &mut step3_group_collapse,
        "BG2EE",
        &wizard_state.step3.bg2ee_collapsed_blocks,
    );

    ModlistWorkspaceState {
        order_bgee: first_order,
        order_bg2ee: second_order,
        order_iwdee,
        expand_state: prior.expand_state.clone(),
        step3_group_collapse,
        prompt_overrides: prior.prompt_overrides.clone(),
        last_share_code: prior.last_share_code.clone(),
        dev_scanned_mods_folder: prior.dev_scanned_mods_folder.clone(),
    }
}

fn order_from_items(items: &[Step3ItemState]) -> Vec<ComponentRef> {
    items
        .iter()
        .filter(|item| !item.is_parent && item.component_id != PARENT_PLACEHOLDER_ID)
        .filter_map(|item| {
            let id = item.component_id.trim().parse::<i64>().ok()?;
            Some(ComponentRef {
                tp2: item.tp_file.clone(),
                id,
                language: 0,
            })
        })
        .collect()
}

fn order_for_tab(
    new_order: Vec<ComponentRef>,
    prior_order: &[ComponentRef],
    scanned_empty: bool,
) -> Vec<ComponentRef> {
    if new_order.is_empty() && !prior_order.is_empty() && scanned_empty {
        prior_order.to_vec()
    } else {
        new_order
    }
}

fn write_collapsed_blocks(
    map: &mut std::collections::HashMap<String, bool>,
    tab: &str,
    collapsed: &[String],
) {
    let prefix = format!("{tab}::");
    map.retain(|key, _| !key.starts_with(&prefix));
    for block_id in collapsed {
        map.insert(format!("{prefix}{block_id}"), true);
    }
}

pub fn sync_paths_from_settings(settings_store: &SettingsStore, wizard_state: &mut WizardState) {
    let Ok(settings) = settings_store.load() else {
        return;
    };
    let from: crate::app::state::Step1State = settings.step1.into();
    let dst = &mut wizard_state.step1;

    dst.bgee_game_folder = from.bgee_game_folder;
    dst.bg2ee_game_folder = from.bg2ee_game_folder;
    dst.iwdee_game_folder = from.iwdee_game_folder;
    dst.eet_bgee_game_folder = from.eet_bgee_game_folder;
    dst.eet_bg2ee_game_folder = from.eet_bg2ee_game_folder;

    dst.bgee_log_folder = from.bgee_log_folder;
    dst.bgee_log_file = from.bgee_log_file;
    dst.bg2ee_log_folder = from.bg2ee_log_folder;
    dst.bg2ee_log_file = from.bg2ee_log_file;
    dst.eet_bgee_log_folder = from.eet_bgee_log_folder;
    dst.eet_bg2ee_log_folder = from.eet_bg2ee_log_folder;
    dst.weidu_log_folder = from.weidu_log_folder;
    dst.log_file = from.log_file;

    dst.eet_pre_dir = from.eet_pre_dir;
    dst.eet_new_dir = from.eet_new_dir;
    dst.generate_directory = from.generate_directory;

    dst.mods_folder = from.mods_folder;
    dst.mods_archive_folder = from.mods_archive_folder;
    dst.mods_backup_folder = from.mods_backup_folder;

    dst.weidu_binary = from.weidu_binary;
    dst.mod_installer_binary = from.mod_installer_binary;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{Step2ComponentState, Step2ModState};
    use crate::registry::model::{Game, ModlistEntry, ModlistState};

    fn comp(id: &str, label: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: id.to_string(),
            label: label.to_string(),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            raw_line: format!("~MOD/MOD.TP2~ #0 #{id} // {label}"),
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

    fn mod_state(name: &str, tp_file: &str, comps: Vec<Step2ComponentState>) -> Step2ModState {
        Step2ModState {
            name: name.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: format!("/mods/{tp_file}"),
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

    fn entry(game: Game) -> ModlistEntry {
        ModlistEntry {
            id: "ABCDEFGHIJKL".to_string(),
            name: "Test".to_string(),
            game,
            state: ModlistState::InProgress,
            ..Default::default()
        }
    }

    #[test]
    fn populate_reconstructs_step2_and_step3_from_order() {
        let mut ws = WizardState::default();

        let workspace = ModlistWorkspaceState {
            order_bgee: vec![
                ComponentRef {
                    tp2: "BG1UB/SETUP-BG1UB.TP2".to_string(),
                    id: 0,
                    language: 0,
                },
                ComponentRef {
                    tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                    id: 2,
                    language: 0,
                },
            ],
            ..Default::default()
        };

        populate_wizard_state_from_workspace(
            &workspace,
            &entry(Game::BGEE),
            &SettingsStore::new_default(),
            &mut ws,
        );

        assert!(
            ws.step2.bgee_mods.is_empty(),
            "Bug B: populate resets the scanned set (re-scan repopulates)"
        );

        ws.step2.bgee_mods = vec![
            mod_state(
                "EEFixPack",
                "EEFIXPACK/EEFIXPACK.TP2",
                vec![comp("0", "Core Fixes"), comp("2", "Game Text Update")],
            ),
            mod_state(
                "BG1UB",
                "BG1UB/SETUP-BG1UB.TP2",
                vec![comp("0", "Restored")],
            ),
        ];
        apply_order_to_mods(&workspace.order_bgee, &mut ws.step2.bgee_mods);
        recompute_mod_checked(&mut ws.step2.bgee_mods);
        let max_order =
            max_selected_order(&ws.step2.bgee_mods).max(max_selected_order(&ws.step2.bg2ee_mods));
        ws.step2.next_selection_order = max_order + 1;
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);

        assert_eq!(ws.step1.game_install, "BGEE");

        let eefix = &ws.step2.bgee_mods[0];
        let bg1ub = &ws.step2.bgee_mods[1];
        assert!(!eefix.components[0].checked, "EEFixPack #0 must stay off");
        assert!(eefix.components[1].checked, "EEFixPack #2 must be on");
        assert_eq!(eefix.components[1].selected_order, Some(2));
        assert!(bg1ub.components[0].checked, "BG1UB #0 must be on");
        assert_eq!(bg1ub.components[0].selected_order, Some(1));
        assert!(eefix.checked);
        assert!(bg1ub.checked);

        let leaves: Vec<&Step3ItemState> = ws
            .step3
            .bgee_items
            .iter()
            .filter(|i| !i.is_parent)
            .collect();
        assert_eq!(leaves.len(), 2, "two component rows expected");
        assert_eq!(leaves[0].tp_file, "BG1UB/SETUP-BG1UB.TP2");
        assert_eq!(leaves[0].component_id, "0");
        assert_eq!(leaves[1].tp_file, "EEFIXPACK/EEFIXPACK.TP2");
        assert_eq!(leaves[1].component_id, "2");
        assert!(ws.step3.bgee_items.iter().any(|i| i.is_parent));
        assert_eq!(ws.step2.next_selection_order, 3);
    }

    #[test]
    fn extract_is_inverse_of_populate_for_order() {
        let mut ws = WizardState::default();
        let workspace = ModlistWorkspaceState {
            order_bgee: vec![
                ComponentRef {
                    tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                    id: 0,
                    language: 0,
                },
                ComponentRef {
                    tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                    id: 2,
                    language: 0,
                },
            ],
            ..Default::default()
        };
        populate_wizard_state_from_workspace(
            &workspace,
            &entry(Game::BGEE),
            &SettingsStore::new_default(),
            &mut ws,
        );
        ws.step2.bgee_mods = vec![mod_state(
            "EEFixPack",
            "EEFIXPACK/EEFIXPACK.TP2",
            vec![comp("0", "Core"), comp("2", "GTU")],
        )];
        apply_order_to_mods(&workspace.order_bgee, &mut ws.step2.bgee_mods);
        recompute_mod_checked(&mut ws.step2.bgee_mods);
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);

        let extracted = extract_workspace_state_from_wizard(&ws, &workspace);
        assert_eq!(extracted.order_bgee, workspace.order_bgee);
    }

    #[test]
    fn extract_carries_dev_scan_folder_through() {
        let ws = WizardState::default();
        let prior = ModlistWorkspaceState {
            dev_scanned_mods_folder: Some(r"D:\corpus".to_string()),
            ..Default::default()
        };
        let extracted = extract_workspace_state_from_wizard(&ws, &prior);
        assert_eq!(
            extracted.dev_scanned_mods_folder.as_deref(),
            Some(r"D:\corpus")
        );
    }

    #[test]
    fn step3_collapse_round_trips() {
        let mut ws = WizardState::default();
        ws.step3.bgee_collapsed_blocks = vec!["BLOCK_A".to_string(), "BLOCK_B".to_string()];
        ws.step3.bg2ee_collapsed_blocks = vec!["BLOCK_C".to_string()];

        let extracted = extract_workspace_state_from_wizard(&ws, &ModlistWorkspaceState::default());
        assert_eq!(
            extracted.step3_group_collapse.get("BGEE::BLOCK_A"),
            Some(&true)
        );
        assert_eq!(
            extracted.step3_group_collapse.get("BG2EE::BLOCK_C"),
            Some(&true)
        );

        let mut ws2 = WizardState::default();
        populate_wizard_state_from_workspace(
            &extracted,
            &entry(Game::EET),
            &SettingsStore::new_default(),
            &mut ws2,
        );
        assert!(
            ws2.step3
                .bgee_collapsed_blocks
                .contains(&"BLOCK_A".to_string())
        );
        assert!(
            ws2.step3
                .bgee_collapsed_blocks
                .contains(&"BLOCK_B".to_string())
        );
        assert!(
            ws2.step3
                .bg2ee_collapsed_blocks
                .contains(&"BLOCK_C".to_string())
        );
    }

    #[test]
    fn swap_clears_prior_selection() {
        let mut ws = WizardState::default();
        ws.step2.bgee_mods = vec![mod_state(
            "EEFixPack",
            "EEFIXPACK/EEFIXPACK.TP2",
            vec![comp("0", "Core")],
        )];
        let a = ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                id: 0,
                language: 0,
            }],
            ..Default::default()
        };
        populate_wizard_state_from_workspace(
            &a,
            &entry(Game::BGEE),
            &SettingsStore::new_default(),
            &mut ws,
        );
        assert!(ws.step2.bgee_mods.is_empty());

        ws.step2.bgee_mods = vec![mod_state(
            "EEFixPack",
            "EEFIXPACK/EEFIXPACK.TP2",
            vec![comp("0", "Core")],
        )];
        ws.step2.bgee_mods[0].components[0].checked = true;

        let b = ModlistWorkspaceState::default();
        populate_wizard_state_from_workspace(
            &b,
            &entry(Game::BGEE),
            &SettingsStore::new_default(),
            &mut ws,
        );
        assert!(
            ws.step2.bgee_mods.is_empty(),
            "Bug B: a swapped-in fresh modlist must NOT inherit the prior \
             modlist's scanned mod set (it must be fully reset, not just \
             unchecked-in-place)"
        );
        assert!(ws.step3.bgee_items.is_empty());
    }

    use crate::registry::store_workspace::WorkspaceStore;

    fn scanned_eefix() -> Vec<Step2ModState> {
        vec![mod_state(
            "EEFixPack",
            "EEFIXPACK/EEFIXPACK.TP2",
            vec![comp("0", "Core Fixes"), comp("2", "Game Text Update")],
        )]
    }

    fn temp_ws_store(label: &str) -> WorkspaceStore {
        let path = std::env::temp_dir().join(format!(
            "bio_fixrun1_{}_{}_workspace.json",
            std::process::id(),
            label
        ));
        let _ = std::fs::remove_file(&path);
        WorkspaceStore::new_with_path(path)
    }

    #[test]
    fn bug_a_step2_toggle_round_trips_through_nav_away_write() {
        let store = temp_ws_store("bug_a");
        let a_entry = entry(Game::BGEE);

        let mut ws = WizardState::default();
        let empty_ws = ModlistWorkspaceState::default();
        populate_wizard_state_from_workspace(
            &empty_ws,
            &a_entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        ws.step2.bgee_mods = scanned_eefix();

        ws.step2.bgee_mods[0].components[1].checked = true;
        ws.step2.bgee_mods[0].components[1].selected_order = Some(1);
        ws.step2.bgee_mods[0].checked = true;
        assert!(
            ws.step3.bgee_items.is_empty(),
            "the bare toggle must not have synced Step 3 (this is why a \
             Step-3-only extract dropped it pre-fix)"
        );

        let prior = empty_ws.clone();
        sync_step3_from_step2_if_changed(&mut ws);
        let extracted = extract_workspace_state_from_wizard(&ws, &prior);
        store.save(&extracted).expect("temp workspace save");

        assert_eq!(
            extracted.order_bgee,
            vec![ComponentRef {
                tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                id: 2,
                language: 0,
            }],
            "Bug A: the Step-2 toggle must reach the persisted order"
        );

        let reloaded = store.load().expect("temp workspace load");
        let mut ws2 = WizardState::default();
        populate_wizard_state_from_workspace(
            &reloaded,
            &a_entry,
            &SettingsStore::new_default(),
            &mut ws2,
        );
        ws2.step2.bgee_mods = scanned_eefix();
        apply_order_to_mods(&reloaded.order_bgee, &mut ws2.step2.bgee_mods);
        recompute_mod_checked(&mut ws2.step2.bgee_mods);
        ws2.step3.bgee_items = step3_sync::build_step3_items(&ws2.step2.bgee_mods);

        assert!(
            ws2.step2.bgee_mods[0].components[1].checked,
            "Bug A: the toggled component must be checked after resume"
        );
        assert!(
            !ws2.step2.bgee_mods[0].components[0].checked,
            "the untoggled component must stay off"
        );
        let leaves: Vec<&Step3ItemState> = ws2
            .step3
            .bgee_items
            .iter()
            .filter(|i| !i.is_parent)
            .collect();
        assert_eq!(leaves.len(), 1, "exactly the toggled component in Step 3");
        assert_eq!(leaves[0].component_id, "2");
    }

    #[test]
    fn bug_b_scanned_set_does_not_leak_across_modlists() {
        let a_entry = entry(Game::BGEE);
        let b_entry = ModlistEntry {
            id: "BBBBBBBBBBBB".to_string(),
            name: "Fresh".to_string(),
            game: Game::BGEE,
            state: ModlistState::InProgress,
            ..Default::default()
        };

        let a_ws = ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                id: 0,
                language: 0,
            }],
            ..Default::default()
        };

        let mut ws = WizardState::default();
        populate_wizard_state_from_workspace(
            &a_ws,
            &a_entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        ws.step2.bgee_mods = scanned_eefix();
        apply_order_to_mods(&a_ws.order_bgee, &mut ws.step2.bgee_mods);
        recompute_mod_checked(&mut ws.step2.bgee_mods);
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);
        assert!(
            ws.step2.bgee_mods[0].components[0].checked,
            "A's selection is live"
        );

        let b_ws = ModlistWorkspaceState::default();
        populate_wizard_state_from_workspace(
            &b_ws,
            &b_entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        assert!(
            ws.step2.bgee_mods.is_empty(),
            "Bug B: fresh modlist B must start with NO scanned mods (A's \
             scanned set must not leak across the swap)"
        );
        assert!(ws.step3.bgee_items.is_empty(), "Bug B: B's Step 3 empty");
        assert!(
            ws.step2.selected.is_none() && ws.step2.next_selection_order == 1,
            "Bug B: B's Step-2 selection transients reset"
        );

        populate_wizard_state_from_workspace(
            &a_ws,
            &a_entry,
            &SettingsStore::new_default(),
            &mut ws,
        );
        ws.step2.bgee_mods = scanned_eefix();
        apply_order_to_mods(&a_ws.order_bgee, &mut ws.step2.bgee_mods);
        recompute_mod_checked(&mut ws.step2.bgee_mods);
        ws.step3.bgee_items = step3_sync::build_step3_items(&ws.step2.bgee_mods);
        assert!(
            ws.step2.bgee_mods[0].components[0].checked,
            "Bug B: swapping back to A restores A's OWN scan + selection"
        );
        assert_eq!(
            ws.step3.bgee_items.iter().filter(|i| !i.is_parent).count(),
            1,
            "Bug B: A's Step 3 restored on the re-swap"
        );
    }

    #[test]
    fn fixrun3_open_then_save_gap_preserves_prior_order_not_wipes_it() {
        let prior = ModlistWorkspaceState {
            order_bgee: vec![
                ComponentRef {
                    tp2: "BG1UB/SETUP-BG1UB.TP2".to_string(),
                    id: 0,
                    language: 0,
                },
                ComponentRef {
                    tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                    id: 2,
                    language: 0,
                },
            ],
            ..Default::default()
        };

        let mut ws = WizardState::default();
        populate_wizard_state_from_workspace(
            &prior,
            &entry(Game::BGEE),
            &SettingsStore::new_default(),
            &mut ws,
        );
        assert!(
            ws.step2.bgee_mods.is_empty() && ws.step3.bgee_items.is_empty(),
            "populate must have reset the scanned set + Step-3 items"
        );

        let extracted = extract_workspace_state_from_wizard(&ws, &prior);
        assert_eq!(
            extracted.order_bgee, prior.order_bgee,
            "Fix-Run 3: the open-path reset must NOT wipe the persisted order \
             (the reported data-loss regression)"
        );
    }

    #[test]
    fn fixrun3_genuine_deselect_all_still_persists_empty_order() {
        let prior = ModlistWorkspaceState {
            order_bgee: vec![ComponentRef {
                tp2: "EEFIXPACK/EEFIXPACK.TP2".to_string(),
                id: 0,
                language: 0,
            }],
            ..Default::default()
        };

        let mut ws = WizardState::default();
        ws.step2.bgee_mods = scanned_eefix();
        for c in &mut ws.step2.bgee_mods[0].components {
            c.checked = false;
            c.selected_order = None;
        }
        ws.step2.bgee_mods[0].checked = false;
        assert!(
            !ws.step2.bgee_mods.is_empty(),
            "scanned set must be non-empty so the guard does NOT fire"
        );
        assert!(ws.step3.bgee_items.is_empty(), "Step 3 deselected → empty");

        let extracted = extract_workspace_state_from_wizard(&ws, &prior);
        assert!(
            extracted.order_bgee.is_empty(),
            "Fix-Run 3: a genuine deselect-everything edit must persist the \
             empty order (guard must not over-block a non-empty scanned set)"
        );
    }

    #[test]
    fn fixrun3_create_like_empty_prior_still_writes_empty_order() {
        let prior = ModlistWorkspaceState::default();
        let ws = WizardState::default();
        assert!(
            prior.order_bgee.is_empty()
                && ws.step2.bgee_mods.is_empty()
                && ws.step3.bgee_items.is_empty(),
            "from-scratch precondition: prior + scanned + Step 3 all empty"
        );

        let extracted = extract_workspace_state_from_wizard(&ws, &prior);
        assert!(
            extracted.order_bgee.is_empty(),
            "Fix-Run 3: an empty prior must not block an empty save \
             (Create-like first write is allowed)"
        );
    }
}
