// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step2ModState;
use crate::app::wlb_inputs::parse_identity;
use crate::registry::workspace_model::{ComponentRef, UnresolvedSelection};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::notification::{
    SelectionRecoveryNotice, SelectionRecoveryRow,
};
use crate::ui::workspace::state_workspace::{RescanSelection, RescanSnapshot};
use crate::ui::workspace::workspace_state_loader::{merge_unresolved_order, same_component};

pub(crate) fn next_scan_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn first_tab(app: &OrchestratorApp) -> &str {
    crate::app::game_authority::first_slot_tab(&app.wizard_state.step1.game_install)
}

fn order_for<'a>(
    workspace: &'a crate::registry::workspace_model::ModlistWorkspaceState,
    tab: &str,
) -> &'a [ComponentRef] {
    match tab {
        "IWDEE" => &workspace.order_iwdee,
        "BG2EE" => &workspace.order_bg2ee,
        _ => &workspace.order_bgee,
    }
}

fn reference(entry: &RescanSelection) -> Option<ComponentRef> {
    Some(
        ComponentRef {
            tp2: entry.tp2_upper.clone(),
            id: entry.component_id.parse().ok()?,
            ..ComponentRef::default()
        }
        .with_answer_sets(entry.answers.clone()),
    )
}

pub(crate) fn include_unresolved(app: &OrchestratorApp, snapshot: &mut RescanSnapshot) {
    let Some(workspace) = app
        .workspace_state
        .get(app.workspace_view.modlist_id.trim())
    else {
        return;
    };
    for (tab, entries) in [
        (first_tab(app), &mut snapshot.bgee),
        ("BG2EE", &mut snapshot.bg2ee),
    ] {
        let mut recovery = workspace.selection_recovery.get(tab).cloned();
        entries.sort_by_key(|entry| entry.selected_order);
        let mut active: Vec<ComponentRef> = Vec::new();
        for captured in entries.iter().filter_map(reference) {
            if !active.iter().any(|item| same_component(item, &captured)) {
                active.push(captured);
            }
        }
        let merged = merge_unresolved_order(active, recovery.as_mut());
        *entries = merged
            .into_iter()
            .enumerate()
            .map(|(position, original)| RescanSelection {
                tp2_upper: original.tp2.to_ascii_uppercase(),
                component_id: original.id.to_string(),
                selected_order: Some(position + 1),
                answers: original.answer_sets(),
            })
            .collect();
    }
}

pub(crate) fn persist_completed_snapshot(app: &mut OrchestratorApp, snapshot: &RescanSnapshot) {
    let owner = app.workspace_view.modlist_id.trim().to_string();
    let first = first_tab(app).to_string();
    let Some(mut workspace) = app.workspace_state.get(&owner).cloned() else {
        return;
    };
    for (tab, entries, mods) in [
        (
            first.as_str(),
            &snapshot.bgee,
            &app.wizard_state.step2.bgee_mods,
        ),
        ("BG2EE", &snapshot.bg2ee, &app.wizard_state.step2.bg2ee_mods),
    ] {
        let prior = order_for(&workspace, tab).to_vec();
        let mut sorted = entries.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|entry| entry.selected_order);
        let mut intended = Vec::new();
        let mut unresolved = Vec::new();
        for entry in sorted {
            let Some(captured) = reference(entry) else {
                continue;
            };
            let original = prior
                .iter()
                .find(|item| same_component(item, &captured))
                .cloned()
                .map_or_else(
                    || captured.clone(),
                    |item| item.with_answer_sets(captured.answer_sets()),
                );
            if intended.iter().any(|item| same_component(item, &original)) {
                continue;
            }
            let position = intended.len();
            if !exact_restored(&original, mods) {
                unresolved.push(UnresolvedSelection {
                    original: original.clone(),
                    position,
                });
            }
            intended.push(original);
        }
        workspace
            .selection_recovery
            .entry(tab.to_string())
            .or_default()
            .unresolved = unresolved;
        match tab {
            "IWDEE" => workspace.order_iwdee = intended,
            "BG2EE" => workspace.order_bg2ee = intended,
            _ => workspace.order_bgee = intended,
        }
    }
    workspace
        .selection_recovery
        .retain(|_, recovery| !recovery.is_empty());
    app.workspace_state.insert(owner.clone(), workspace);
    app.persistence_cycle
        .mark_workspace_dirty(&owner, std::time::Instant::now());
    app.mark_workspace_dirty();
}

fn exact_restored(original: &ComponentRef, mods: &[Step2ModState]) -> bool {
    let id = original.id.to_string();
    mods.iter()
        .filter(|m| m.tp_file.eq_ignore_ascii_case(&original.tp2))
        .flat_map(|m| &m.components)
        .any(|c| c.checked && !c.disabled && c.component_id == id)
}

pub(crate) fn unresolved_reason(original: &ComponentRef, mods: &[Step2ModState]) -> Option<String> {
    let matching = mods
        .iter()
        .filter(|m| m.tp_file.eq_ignore_ascii_case(&original.tp2))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Some("mod no longer present".to_string());
    }
    let id = original.id.to_string();
    let components = matching
        .iter()
        .flat_map(|m| m.components.iter().filter(|c| c.component_id == id))
        .collect::<Vec<_>>();
    if components.is_empty() {
        return Some("component no longer present".to_string());
    }
    let normalized_tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&original.tp2);
    if components.iter().any(|c| {
        parse_identity(&c.raw_line).is_none_or(|identity| {
            identity.component_id != original.id || identity.normalized_tp2 != normalized_tp2
        })
    }) {
        return Some("invalid component identity".to_string());
    }
    if components.len() != 1 {
        return Some("ambiguous component candidates".to_string());
    }
    let component = components[0];
    component
        .disabled
        .then(|| crate::app::app_step2_log::component_exclusion_reason(component))
}

pub(crate) fn notify_unresolved(app: &mut OrchestratorApp) {
    let owner = app.workspace_view.modlist_id.trim().to_string();
    let Some(workspace) = app.workspace_state.get(&owner) else {
        return;
    };
    let mut rows = Vec::new();
    for (tab, mods) in [
        (first_tab(app), &app.wizard_state.step2.bgee_mods),
        ("BG2EE", &app.wizard_state.step2.bg2ee_mods),
    ] {
        if let Some(recovery) = workspace.selection_recovery.get(tab) {
            for entry in &recovery.unresolved {
                if let Some(reason) = unresolved_reason(&entry.original, mods) {
                    rows.push(SelectionRecoveryRow {
                        tab: tab.to_string(),
                        original: entry.original.clone(),
                        reason,
                    });
                }
            }
        }
    }
    if rows.is_empty() {
        app.workspace_view.step2.rescan_drop_warning = None;
        return;
    }
    app.workspace_view.step2.rescan_drop_warning = Some(format!(
        "{} saved component(s) remain unresolved",
        rows.len()
    ));
    app.notification_manager
        .warn_selection_recovery(SelectionRecoveryNotice {
            owner,
            generation: app.workspace_view.step2.recovery_scan_generation,
            rows,
        });
}

#[cfg(test)]
#[path = "step2_selection_recovery_tests.rs"]
mod tests;
