// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{Step2Selection, WizardState};

use super::model::Step2Details;
use super::parse::{parse_lang, parse_version};

mod issues;
mod key;

fn empty_details() -> Step2Details {
    Step2Details::default()
}

pub fn selected_details(state: &WizardState) -> Step2Details {
    let Some(selection) = &state.step2.selected else {
        return empty_details();
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
    match selection {
        Step2Selection::Mod { tp_file, .. } => mods
            .iter()
            .find(|m| &m.tp_file == tp_file)
            .map(|mod_state| Step2Details {
                mod_name: Some(mod_state.name.clone()),
                component_label: None,
                component_id: None,
                component_lang: None,
                component_version: None,
                selected_order: None,
                is_checked: None,
                is_disabled: None,
                compat_kind: None,
                compat_role: None,
                compat_code: None,
                disabled_reason: None,
                compat_source: None,
                compat_related_target: None,
                compat_graph: None,
                compat_evidence: None,
                raw_line: None,
                tp_file: Some(mod_state.tp_file.clone()),
                tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
                readme_path: mod_state.readme_path.clone(),
                web_url: mod_state.web_url.clone(),
            })
            .unwrap_or_else(empty_details),
        Step2Selection::Component {
            tp_file,
            component_id,
            component_key,
            ..
        } => mods
            .iter()
            .find(|m| &m.tp_file == tp_file)
            .and_then(|mod_state| {
                mod_state
                    .components
                    .iter()
                    .find(|c| {
                        &c.component_id == component_id
                            && (component_key.is_empty() || c.raw_line == *component_key)
                    })
                    .map(|component| {
                        let component_tp2 = parse_component_tp2_from_raw(&component.raw_line)
                            .unwrap_or_else(|| mod_state.tp_file.clone());
                        let component_mod_key = key::normalize_mod_key(&component_tp2);
                        let component_mod_name = key::display_name_from_tp2(&component_tp2);
                        let mut compat_kind = component.compat_kind.clone();
                        let mut compat_role: Option<String> = None;
                        let mut compat_code: Option<String> = None;
                        let mut compat_source = component.compat_source.clone();
                        let mut compat_related_target = component.compat_related_mod.as_deref().map(|m| {
                            format!(
                                "{}{}",
                                m,
                                component
                                    .compat_related_component
                                    .as_deref()
                                    .map(|c| format!(" #{c}"))
                                    .unwrap_or_default()
                            )
                        });
                        let mut compat_graph: Option<String> = component.compat_graph.clone();
                        let mut compat_evidence: Option<String> = component.compat_evidence.clone();
                        let mut disabled_reason = component.disabled_reason.clone();
                        let mod_key = component_mod_key;
                        let comp_id = key::parse_component_u32(&component.component_id);

                        if let Some(issue) = state
                            .compat
                            .issues
                            .iter()
                            .find(|issue| issues::issue_matches_affected(issue, &mod_key, comp_id))
                        {
                            compat_role = Some("Affected".to_string());
                            compat_code = Some(issue.code.clone());
                            if compat_kind.is_none() {
                                compat_kind = Some(issues::issue_to_compat_kind(issue));
                            }
                            if compat_source.is_none() {
                                compat_source = Some(issue.source.clone());
                            }
                            if compat_related_target.is_none() {
                                compat_related_target = Some(issues::issue_related_target(issue));
                            }
                            if disabled_reason.is_none() && !issue.reason.trim().is_empty() {
                                disabled_reason = Some(issue.reason.clone());
                            }
                            compat_graph = Some(issues::issue_graph(issue));
                            compat_evidence = issue.raw_evidence.clone();
                        } else if let Some(issue) = state
                            .compat
                            .issues
                            .iter()
                            .find(|issue| issues::issue_matches_related(issue, &mod_key, comp_id))
                        {
                            compat_role = Some("Related target".to_string());
                            compat_code = Some(issue.code.clone());
                            if compat_kind.is_none() {
                                compat_kind = Some(issues::issue_to_compat_kind(issue));
                            }
                            if compat_source.is_none() {
                                compat_source = Some(issue.source.clone());
                            }
                            if compat_related_target.is_none() {
                                compat_related_target = Some(issues::issue_related_target(issue));
                            }
                            if disabled_reason.is_none() && !issue.reason.trim().is_empty() {
                                disabled_reason = Some(format!(
                                    "Conflicts with {}",
                                    key::format_target(&issue.affected_mod, issue.affected_component)
                                ));
                            }
                            compat_graph = Some(issues::issue_graph(issue));
                            compat_evidence = issue.raw_evidence.clone();
                        }

                        Step2Details {
                            mod_name: Some(component_mod_name),
                            component_label: Some(component.label.clone()),
                            component_id: Some(component.component_id.clone()),
                            component_lang: parse_lang(&component.raw_line),
                            component_version: parse_version(&component.raw_line),
                            selected_order: component.selected_order,
                            is_checked: Some(component.checked),
                            is_disabled: Some(component.disabled),
                            compat_kind,
                            compat_role,
                            compat_code,
                            disabled_reason,
                            compat_source,
                            compat_related_target,
                            compat_graph,
                            compat_evidence,
                            raw_line: Some(component.raw_line.clone()),
                            tp_file: Some(component_tp2),
                            tp2_path: (!mod_state.tp2_path.is_empty()).then_some(mod_state.tp2_path.clone()),
                            readme_path: mod_state.readme_path.clone(),
                            web_url: mod_state.web_url.clone(),
                        }
                    })
            })
            .unwrap_or_else(empty_details),
    }
}
