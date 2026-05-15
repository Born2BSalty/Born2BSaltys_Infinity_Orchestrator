// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::registry::model::{Game, ModlistEntry};
use crate::registry::workspace_model::{ComponentRef, ModlistWorkspaceState};
use crate::settings::model::Step1Settings;

pub fn populate_wizard_state_from_workspace(
    _workspace: &ModlistWorkspaceState,
    registry_entry: &ModlistEntry,
    _settings: &Step1Settings,
    wizard_state: &mut WizardState,
) {
    wizard_state.step1.game_install = game_to_step1_value(&registry_entry.game).to_string();
    // Batch 5 is load-only and only maps proven fields. The persisted
    // workspace component refs are not enough to reconstruct Step3ItemState.
    // Step 2/3 reconstruction remains deferred until that mapping is proven.
}

#[must_use]
pub fn extract_workspace_state_from_wizard(
    existing: &ModlistWorkspaceState,
    wizard_state: &WizardState,
) -> ModlistWorkspaceState {
    let mut next = existing.clone();
    next.order_bgee = extract_order(&existing.order_bgee, &wizard_state.step3.bgee_items);
    next.order_bg2ee = extract_order(&existing.order_bg2ee, &wizard_state.step3.bg2ee_items);
    next.step3_group_collapse
        .clone_from(&existing.step3_group_collapse);
    for block_id in &wizard_state.step3.bgee_collapsed_blocks {
        next.step3_group_collapse
            .insert(format!("BGEE:{block_id}"), true);
    }
    for block_id in &wizard_state.step3.bg2ee_collapsed_blocks {
        next.step3_group_collapse
            .insert(format!("BG2EE:{block_id}"), true);
    }
    next
}

const fn game_to_step1_value(game: &Game) -> &'static str {
    match game {
        Game::BGEE => "BGEE",
        Game::BG2EE => "BG2EE",
        Game::IWDEE => "IWDEE",
        Game::EET => "EET",
    }
}

fn extract_order(
    existing: &[ComponentRef],
    items: &[crate::app::state::Step3ItemState],
) -> Vec<ComponentRef> {
    if items.is_empty() {
        return existing.to_vec();
    }

    let refs = items
        .iter()
        .filter_map(|item| {
            let id = item
                .component_id
                .trim()
                .trim_start_matches('#')
                .parse()
                .ok()?;
            Some(ComponentRef {
                tp2: item.tp_file.clone(),
                id,
                language: 0,
            })
        })
        .collect::<Vec<_>>();
    if refs.is_empty() {
        existing.to_vec()
    } else {
        refs
    }
}
