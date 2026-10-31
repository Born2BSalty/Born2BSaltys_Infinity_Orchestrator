// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::Step2ModState;
use crate::app::wlb_inputs::parse_identity;
use crate::registry::workspace_model::{
    ComponentRef, RetainedLanguageAnswers, UnresolvedSelection,
};
use crate::ui::orchestrator::orchestrator_app::OrchestratorApp;
use crate::ui::orchestrator::widgets::notification::{LanguageRecoveryNotice, LanguageRecoveryRow};
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
    Some(ComponentRef {
        tp2: entry.tp2_upper.clone(),
        id: entry.component_id.parse().ok()?,
        language: entry.language,
        wlb_inputs: entry.wlb_inputs.clone(),
    })
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
        let active = entries.iter().filter_map(reference).collect();
        let merged = merge_unresolved_order(active, recovery.as_mut());
        *entries = merged
            .into_iter()
            .enumerate()
            .map(|(position, original)| RescanSelection {
                tp2_upper: original.tp2.to_ascii_uppercase(),
                component_id: original.id.to_string(),
                language: original.language,
                selected_order: Some(position + 1),
                wlb_inputs: original.wlb_inputs,
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
                    |mut item| {
                        item.wlb_inputs.clone_from(&captured.wlb_inputs);
                        item
                    },
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
    mods.iter()
        .filter(|m| m.tp_file.eq_ignore_ascii_case(&original.tp2))
        .flat_map(|m| &m.components)
        .any(|c| {
            c.checked
                && !c.disabled
                && c.component_id == original.id.to_string()
                && parse_identity(&c.raw_line)
                    .is_some_and(|identity| identity.language == original.language)
        })
}

pub(crate) fn candidate(
    original: &ComponentRef,
    mods: &[Step2ModState],
) -> Result<(usize, usize, u8), String> {
    let matching = mods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.tp_file.eq_ignore_ascii_case(&original.tp2))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Err("mod no longer present".to_string());
    }
    let components = matching
        .iter()
        .flat_map(|(mi, m)| {
            m.components.iter().enumerate().filter_map(move |(ci, c)| {
                (c.component_id == original.id.to_string()).then_some((*mi, ci, c))
            })
        })
        .collect::<Vec<_>>();
    if components.is_empty() {
        return Err("component no longer present".to_string());
    }
    if components.iter().any(|(_, _, c)| {
        parse_identity(&c.raw_line).is_none_or(|identity| {
            identity.component_id != original.id
                || identity.normalized_tp2
                    != crate::app::mod_downloads::normalize_mod_download_tp2(&original.tp2)
        })
    }) {
        return Err("invalid component identity".to_string());
    }
    if components.iter().any(|(_, _, c)| {
        parse_identity(&c.raw_line).is_some_and(|identity| identity.language == original.language)
    }) {
        return Err("saved language is present but unavailable".to_string());
    }
    if components.len() != 1 {
        return Err("ambiguous component candidates".to_string());
    }
    let (mi, ci, component) = components[0];
    if component.disabled {
        return Err(crate::app::app_step2_log::component_exclusion_reason(
            component,
        ));
    }
    let identity = parse_identity(&component.raw_line)
        .ok_or_else(|| "invalid component identity".to_string())?;
    Ok((mi, ci, identity.language))
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
                let result = candidate(&entry.original, mods);
                rows.push(LanguageRecoveryRow {
                    tab: tab.to_string(),
                    original: entry.original.clone(),
                    available_language: result.as_ref().ok().map(|(_, _, language)| *language),
                    reason: result.err().unwrap_or_default(),
                });
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
        .warn_language_recovery(LanguageRecoveryNotice {
            owner,
            generation: app.workspace_view.step2.recovery_scan_generation,
            rows,
        });
}

fn apply_row(
    app: &mut OrchestratorApp,
    workspace: &mut crate::registry::workspace_model::ModlistWorkspaceState,
    row: &LanguageRecoveryRow,
    first: &str,
) -> bool {
    if row.tab != first && row.tab != "BG2EE" {
        return false;
    }
    let Some(recovery) = workspace.selection_recovery.get_mut(&row.tab) else {
        return false;
    };
    let Some(index) = recovery
        .unresolved
        .iter()
        .position(|entry| entry.original == row.original)
    else {
        return false;
    };
    let mods = if row.tab == "BG2EE" {
        &mut app.wizard_state.step2.bg2ee_mods
    } else {
        &mut app.wizard_state.step2.bgee_mods
    };
    let Ok((mi, ci, language)) = candidate(&row.original, mods) else {
        return false;
    };
    if row.available_language != Some(language) {
        return false;
    }
    let entry = &recovery.unresolved[index];
    let component = &mut mods[mi].components[ci];
    let raw = component.raw_line.as_str();
    let marker = raw.to_ascii_lowercase().find("@wlb-inputs:");
    if let Some(start) = marker {
        component.raw_line = raw[..start]
            .trim_end()
            .strip_suffix("//")
            .unwrap_or_else(|| raw[..start].trim_end())
            .trim_end()
            .to_string();
    }
    component.checked = true;
    component.selected_order = Some(entry.position + 1);
    let mut target = row.original.clone();
    target.language = language;
    target.wlb_inputs = None;
    if row.original.wlb_inputs.is_some() {
        recovery.retained_answers.push(RetainedLanguageAnswers {
            original: row.original.clone(),
            target: target.clone(),
        });
    }
    let intended = match row.tab.as_str() {
        "IWDEE" => &mut workspace.order_iwdee,
        "BG2EE" => &mut workspace.order_bg2ee,
        _ => &mut workspace.order_bgee,
    };
    if let Some(original) = intended
        .iter_mut()
        .find(|item| same_component(item, &row.original))
    {
        *original = target;
    }
    recovery.unresolved.remove(index);
    true
}

fn rebuild_recovered_order(
    app: &mut OrchestratorApp,
    workspace: &crate::registry::workspace_model::ModlistWorkspaceState,
    changed_tabs: &std::collections::BTreeSet<String>,
) {
    for tab in changed_tabs {
        let intended = order_for(workspace, tab);
        let (mods, items) = if tab == "BG2EE" {
            (
                &mut app.wizard_state.step2.bg2ee_mods,
                &mut app.wizard_state.step3.bg2ee_items,
            )
        } else {
            (
                &mut app.wizard_state.step2.bgee_mods,
                &mut app.wizard_state.step3.bgee_items,
            )
        };
        for m in mods.iter_mut() {
            for component in m
                .components
                .iter_mut()
                .filter(|component| component.checked)
            {
                if let Some(identity) = parse_identity(&component.raw_line)
                    && let Some(position) = intended.iter().position(|item| {
                        item.tp2.eq_ignore_ascii_case(&m.tp_file)
                            && item.id == identity.component_id
                            && item.language == identity.language
                    })
                {
                    component.selected_order = Some(position + 1);
                    if let Some(inputs) = intended[position].wlb_inputs.as_deref() {
                        crate::ui::workspace::workspace_state_loader::reattach_wlb_inputs(
                            component, inputs,
                        );
                    }
                }
            }
        }
        items.clear();
    }
    let retained_max = workspace
        .selection_recovery
        .values()
        .flat_map(|recovery| &recovery.unresolved)
        .map(|entry| entry.position + 1)
        .max()
        .unwrap_or(0);
    let mut max_order = retained_max;
    for mods in [
        &mut app.wizard_state.step2.bgee_mods,
        &mut app.wizard_state.step2.bg2ee_mods,
    ] {
        for m in mods {
            m.checked = m.components.iter().any(|c| c.checked);
            max_order = max_order.max(
                m.components
                    .iter()
                    .filter_map(|c| c.selected_order)
                    .max()
                    .unwrap_or(0),
            );
        }
    }
    app.wizard_state.step2.next_selection_order = max_order + 1;
    crate::ui::workspace::workspace_state_loader::sync_step3_from_step2_if_changed(
        &mut app.wizard_state,
    );
}

pub(crate) fn apply_recovery(app: &mut OrchestratorApp, notice: LanguageRecoveryNotice) {
    if app.wizard_state.step2.is_scanning
        || app.workspace_view.modlist_id.trim() != notice.owner
        || app.workspace_view.loaded_workspace_id.as_deref() != Some(notice.owner.as_str())
        || app.workspace_view.step2.recovery_scan_generation != notice.generation
        || app.wizard_state.step2.last_scan_report.is_none()
    {
        return;
    }
    let Some(prior) = app.workspace_state.get(&notice.owner) else {
        return;
    };
    let mut workspace =
        crate::ui::workspace::workspace_state_loader::extract_workspace_state_from_wizard(
            &app.wizard_state,
            prior,
        );
    let first = first_tab(app).to_string();
    let mut restored = 0;
    let mut changed_tabs = std::collections::BTreeSet::new();
    for row in notice
        .rows
        .iter()
        .filter(|row| row.available_language.is_some())
    {
        if apply_row(app, &mut workspace, row, &first) {
            changed_tabs.insert(row.tab.clone());
            restored += 1;
        }
    }
    if restored == 0 {
        return;
    }
    workspace
        .selection_recovery
        .retain(|_, recovery| !recovery.is_empty());
    rebuild_recovered_order(app, &workspace, &changed_tabs);
    app.persistence_cycle
        .mark_workspace_dirty(&notice.owner, std::time::Instant::now());
    app.workspace_state.insert(notice.owner, workspace);
    app.mark_workspace_dirty();
    notify_unresolved(app);
}

#[cfg(test)]
#[path = "step2_language_recovery_tests.rs"]
mod tests;
