// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::path::Path;

use crate::app::compat_issue::CompatIssue;
use crate::app::compat_popup_targets::details_related_target;
use crate::app::component_block_preview::load_component_block_preview;
use crate::app::component_details::{
    compat_code_from_kind, compat_role, display_name_from_tp2, tp2_file_name,
};
use crate::app::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::app::state::{Step2Selection, WizardState};
use crate::parser::weidu_component_line::parse_lang;
use crate::parser::weidu_version::parse_version;

#[derive(Debug, Clone, Default)]
pub(crate) struct SelectedDetailsData {
    pub(crate) mod_name: Option<String>,
    pub(crate) component_label: Option<String>,
    pub(crate) component_id: Option<String>,
    pub(crate) shown_component_count: Option<usize>,
    pub(crate) hidden_component_count: Option<usize>,
    pub(crate) raw_component_count: Option<usize>,
    pub(crate) component_lang: Option<String>,
    pub(crate) component_version: Option<String>,
    pub(crate) selected_order: Option<usize>,
    pub(crate) is_checked: Option<bool>,
    pub(crate) is_disabled: Option<bool>,
    pub(crate) compat_kind: Option<String>,
    pub(crate) compat_role: Option<String>,
    pub(crate) compat_code: Option<String>,
    pub(crate) disabled_reason: Option<String>,
    pub(crate) compat_source: Option<String>,
    pub(crate) compat_related_mod: Option<String>,
    pub(crate) compat_related_component: Option<String>,
    pub(crate) compat_related_target: Option<String>,
    pub(crate) compat_graph: Option<String>,
    pub(crate) compat_evidence: Option<String>,
    pub(crate) compat_component_block: Option<String>,
    pub(crate) raw_line: Option<String>,
    pub(crate) tp_file: Option<String>,
    pub(crate) tp2_folder: Option<String>,
    pub(crate) tp2_path: Option<String>,
    pub(crate) readme_path: Option<String>,
    pub(crate) web_url: Option<String>,
    pub(crate) package_source_status: Option<String>,
    pub(crate) package_source_name: Option<String>,
    pub(crate) package_latest_version: Option<String>,
    pub(crate) package_source_url: Option<String>,
    pub(crate) package_source_github: Option<String>,
    pub(crate) package_update_locked: Option<bool>,
    pub(crate) package_can_check_updates: bool,
}

pub(crate) fn selected_details_data(state: &WizardState) -> SelectedDetailsData {
    let Some(selection) = &state.step2.selected else {
        return SelectedDetailsData::default();
    };
    let mods = match selection {
        Step2Selection::Mod { game_tab, .. } | Step2Selection::Component { game_tab, .. } => {
            if game_tab == "BGEE" {
                &state.step2.bgee_mods
            } else {
                &state.step2.bg2ee_mods
            }
        }
    };
    let mut details = match selection {
        Step2Selection::Mod { tp_file, .. } => mods
            .iter()
            .find(|mod_state| &mod_state.tp_file == tp_file)
            .map(build_mod_details)
            .unwrap_or_default(),
        Step2Selection::Component {
            tp_file,
            component_id,
            component_key,
            ..
        } => mods
            .iter()
            .find(|mod_state| &mod_state.tp_file == tp_file)
            .and_then(|mod_state| {
                mod_state
                    .components
                    .iter()
                    .find(|component| {
                        &component.component_id == component_id
                            && (component_key.is_empty() || component.raw_line == *component_key)
                    })
                    .map(|component| build_component_details(mod_state, component))
            })
            .unwrap_or_default(),
    };
    details.package_can_check_updates = can_check_updates_from_details(state);
    details
}

pub(crate) fn selected_compat_issue(state: &WizardState) -> Option<CompatIssue> {
    if let Some(issue) = state.step2.compat_popup_issue_override.clone() {
        return Some(issue);
    }
    synth_issue_from_details(&selected_details_data(state))
}

pub(crate) fn selected_source_reference(state: &WizardState) -> Option<String> {
    if let Some(issue) = state.step2.compat_popup_issue_override.as_ref() {
        return (!issue.source.trim().is_empty()).then_some(issue.source.clone());
    }
    selected_details_data(state).compat_source
}

fn build_mod_details(mod_state: &crate::app::state::Step2ModState) -> SelectedDetailsData {
    let mut details = SelectedDetailsData {
        mod_name: Some(mod_state.name.clone()),
        component_version: details_mod_version(mod_state),
        shown_component_count: Some(mod_state.components.len()),
        hidden_component_count: Some(mod_state.hidden_components.len()),
        raw_component_count: Some(mod_state.components.len() + mod_state.hidden_components.len()),
        tp_file: Some(tp2_file_name(&mod_state.tp_file)),
        tp2_folder: details_parent_folder(&mod_state.tp2_path),
        tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
        readme_path: mod_state.readme_path.clone(),
        web_url: mod_state.web_url.clone(),
        package_latest_version: mod_state.latest_checked_version.clone(),
        package_update_locked: Some(mod_state.update_locked),
        ..SelectedDetailsData::default()
    };
    attach_package_source(&mut details, &mod_state.tp_file);
    details
}

fn build_component_details(
    mod_state: &crate::app::state::Step2ModState,
    component: &crate::app::state::Step2ComponentState,
) -> SelectedDetailsData {
    let component_tp2 = parse_component_tp2_from_raw(&component.raw_line)
        .unwrap_or_else(|| mod_state.tp_file.clone());
    let compat_kind = component.compat_kind.clone();
    let compat_related_target = component.compat_related_mod.as_deref().map(|related_mod| {
        format!(
            "{}{}",
            related_mod,
            component
                .compat_related_component
                .as_deref()
                .map(|component_id| format!(" #{component_id}"))
                .unwrap_or_default()
        )
    });
    let compat_component_block =
        load_component_block_preview(&mod_state.tp2_path, &component.component_id);

    SelectedDetailsData {
        mod_name: Some(display_name_from_tp2(&component_tp2)),
        component_label: Some(component.label.clone()),
        component_id: Some(component.component_id.clone()),
        shown_component_count: Some(mod_state.components.len()),
        hidden_component_count: Some(mod_state.hidden_components.len()),
        raw_component_count: Some(mod_state.components.len() + mod_state.hidden_components.len()),
        component_version: details_mod_version(mod_state),
        component_lang: parse_lang(&component.raw_line),
        selected_order: component.selected_order,
        is_checked: Some(component.checked),
        is_disabled: Some(component.disabled),
        compat_kind: compat_kind.clone(),
        compat_role: compat_kind
            .as_ref()
            .map(|kind| compat_role(kind, component.compat_source.as_deref())),
        compat_code: compat_kind.as_deref().map(compat_code_from_kind),
        disabled_reason: component.disabled_reason.clone(),
        compat_source: component.compat_source.clone(),
        compat_related_mod: component.compat_related_mod.clone(),
        compat_related_component: component.compat_related_component.clone(),
        compat_related_target,
        compat_graph: component.compat_graph.clone(),
        compat_evidence: component.compat_evidence.clone(),
        compat_component_block,
        raw_line: Some(component.raw_line.clone()),
        tp_file: Some(tp2_file_name(&component_tp2)),
        tp2_folder: details_parent_folder(&mod_state.tp2_path),
        tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
        readme_path: mod_state.readme_path.clone(),
        web_url: mod_state.web_url.clone(),
        ..SelectedDetailsData::default()
    }
}

fn attach_package_source(details: &mut SelectedDetailsData, tp2: &str) {
    details.package_source_status = Some("Unknown".to_string());
    let loaded = crate::app::mod_downloads::load_mod_download_sources();
    let Some(source) = loaded.find_source(tp2) else {
        return;
    };
    details.package_source_status = Some("Known".to_string());
    details.package_source_name = (!source.name.is_empty()).then_some(source.name);
    details.package_source_url = Some(source.url);
    details.package_source_github = source.github;
}

fn details_parent_folder(tp2_path: &str) -> Option<String> {
    let tp2_path = tp2_path.trim();
    if tp2_path.is_empty() {
        return None;
    }
    Path::new(tp2_path)
        .parent()
        .map(|path| path.display().to_string())
        .filter(|path| !path.trim().is_empty())
}

fn details_mod_version(mod_state: &crate::app::state::Step2ModState) -> Option<String> {
    mod_state
        .components
        .iter()
        .find_map(|component| parse_version(&component.raw_line))
}

fn synth_issue_from_details(details: &SelectedDetailsData) -> Option<CompatIssue> {
    let kind = details.compat_kind.as_deref()?.to_ascii_lowercase();
    let related = if kind == "mismatch" || kind == "game_mismatch" {
        details.compat_related_target.clone().unwrap_or_default()
    } else {
        details
            .compat_related_target
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    };
    let fallback_related = details_related_target(
        details.compat_related_mod.as_deref(),
        details.compat_related_component.as_deref(),
        details.compat_evidence.as_deref(),
    );
    Some(CompatIssue {
        kind,
        related_mod: details
            .compat_related_mod
            .clone()
            .or_else(|| fallback_related.as_ref().map(|pair| pair.0.clone()))
            .unwrap_or(related),
        related_component: details
            .compat_related_component
            .as_deref()
            .and_then(|value| value.parse::<u32>().ok())
            .or_else(|| fallback_related.and_then(|(_, related_component)| related_component)),
        reason: details.disabled_reason.clone().unwrap_or_default(),
        source: details.compat_source.clone().unwrap_or_default(),
        raw_evidence: details.compat_evidence.clone(),
    })
}

fn can_check_updates_from_details(state: &WizardState) -> bool {
    !state.step1.installs_exactly_from_weidu_logs()
}
