// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::controller::log_apply_match::parse_component_tp2_from_raw;
use crate::ui::state::{CompatIssueDisplay, Step2Selection, WizardState};
use crate::ui::step2::state_step2::Step2Details;

pub fn selected_details(state: &WizardState) -> Step2Details {
    let Some(selection) = &state.step2.selected else {
        return Step2Details::default();
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
            .unwrap_or_default(),
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
                        let component_mod_key =
                            crate::ui::step2::service_selection_step2::selection_normalize_mod_key(
                                &component_tp2,
                            );
                        let component_mod_name = details_display_name_from_tp2(&component_tp2);
                        let mut compat_kind = component.compat_kind.clone();
                        let mut compat_role: Option<String> = None;
                        let mut compat_code: Option<String> = None;
                        let mut compat_source = component.compat_source.clone();
                        let mut compat_related_target =
                            component.compat_related_mod.as_deref().map(|m| {
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
                        let comp_id = details_parse_component_u32(&component.component_id);

                        if let Some(issue) =
                            state.compat.issues.iter().find(|issue| {
                                details_issue_matches_affected(issue, &mod_key, comp_id)
                            })
                        {
                            compat_role = Some("Affected".to_string());
                            compat_code = Some(issue.code.clone());
                            if compat_kind.is_none() {
                                compat_kind = Some(details_issue_to_compat_kind(issue));
                            }
                            if compat_source.is_none() {
                                compat_source = Some(issue.source.clone());
                            }
                            if compat_related_target.is_none() {
                                compat_related_target = Some(details_issue_related_target(issue));
                            }
                            if disabled_reason.is_none() && !issue.reason.trim().is_empty() {
                                disabled_reason = Some(issue.reason.clone());
                            }
                            compat_graph = Some(details_issue_graph(issue));
                            compat_evidence = issue.raw_evidence.clone();
                        } else if let Some(issue) =
                            state.compat.issues.iter().find(|issue| {
                                details_issue_matches_related(issue, &mod_key, comp_id)
                            })
                        {
                            compat_role = Some("Related target".to_string());
                            compat_code = Some(issue.code.clone());
                            if compat_kind.is_none() {
                                compat_kind = Some(details_issue_to_compat_kind(issue));
                            }
                            if compat_source.is_none() {
                                compat_source = Some(issue.source.clone());
                            }
                            if compat_related_target.is_none() {
                                compat_related_target = Some(details_issue_related_target(issue));
                            }
                            if disabled_reason.is_none() && !issue.reason.trim().is_empty() {
                                disabled_reason = Some(format!(
                                    "Conflicts with {}",
                                    details_format_target(
                                        &issue.affected_mod,
                                        issue.affected_component
                                    )
                                ));
                            }
                            compat_graph = Some(details_issue_graph(issue));
                            compat_evidence = issue.raw_evidence.clone();
                        }

                        Step2Details {
                            mod_name: Some(component_mod_name),
                            component_label: Some(component.label.clone()),
                            component_id: Some(component.component_id.clone()),
                            component_lang: crate::ui::step2::service_step2::parse_lang(
                                &component.raw_line,
                            ),
                            component_version: crate::ui::step2::service_step2::parse_version(
                                &component.raw_line,
                            ),
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
                            tp2_path: (!mod_state.tp2_path.is_empty())
                                .then_some(mod_state.tp2_path.clone()),
                            readme_path: mod_state.readme_path.clone(),
                            web_url: mod_state.web_url.clone(),
                        }
                    })
            })
            .unwrap_or_default(),
    }
}

fn details_issue_matches_affected(
    issue: &CompatIssueDisplay,
    mod_key: &str,
    comp_id: Option<u32>,
) -> bool {
    if crate::ui::step2::service_selection_step2::selection_normalize_mod_key(&issue.affected_mod)
        != mod_key
    {
        return false;
    }
    match (issue.affected_component, comp_id) {
        (Some(a), Some(b)) => a == b,
        (None, _) => true,
        _ => false,
    }
}

fn details_issue_matches_related(
    issue: &CompatIssueDisplay,
    mod_key: &str,
    comp_id: Option<u32>,
) -> bool {
    if crate::ui::step2::service_selection_step2::selection_normalize_mod_key(&issue.related_mod)
        != mod_key
    {
        return false;
    }
    match (issue.related_component, comp_id) {
        (Some(a), Some(b)) => a == b,
        (None, _) => true,
        _ => false,
    }
}

fn details_issue_to_compat_kind(issue: &CompatIssueDisplay) -> String {
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        "missing_dep".to_string()
    } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        "game_mismatch".to_string()
    } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        "conditional".to_string()
    } else if issue.is_blocking {
        "conflict".to_string()
    } else {
        "warning".to_string()
    }
}

fn details_issue_graph(issue: &CompatIssueDisplay) -> String {
    if details_is_duplicate_selection_issue(issue) {
        return format!(
            "{} appears multiple times in selection",
            details_format_target(&issue.affected_mod, issue.affected_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = issue
            .related_mod
            .split('|')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "{} allowed on: {}",
            details_format_target(&issue.affected_mod, issue.affected_component),
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
        );
    }
    if issue.code.eq_ignore_ascii_case("FORBID_HIT") || issue.code.eq_ignore_ascii_case("RULE_HIT")
    {
        return format!(
            "{} conflicts with {}",
            details_format_target(&issue.affected_mod, issue.affected_component),
            details_format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
        if let Some(or_targets) = details_parse_or_targets_from_reason(&issue.reason) {
            return format!(
                "{} requires one of: {}",
                details_format_target(&issue.affected_mod, issue.affected_component),
                or_targets.join(" | ")
            );
        }
        return format!(
            "{} requires {}",
            details_format_target(&issue.affected_mod, issue.affected_component),
            details_format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
        return format!(
            "{} has optional patch for {}",
            details_format_target(&issue.affected_mod, issue.affected_component),
            details_format_target(&issue.related_mod, issue.related_component)
        );
    }
    if issue.code.eq_ignore_ascii_case("ORDER_WARN") {
        return format!(
            "{} should be installed after {}",
            details_format_target(&issue.affected_mod, issue.affected_component),
            details_format_target(&issue.related_mod, issue.related_component)
        );
    }
    format!(
        "{} -> {}",
        details_format_target(&issue.affected_mod, issue.affected_component),
        details_format_target(&issue.related_mod, issue.related_component)
    )
}

fn details_issue_related_target(issue: &CompatIssueDisplay) -> String {
    if details_is_duplicate_selection_issue(issue) {
        return "Duplicate selection".to_string();
    }
    if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
        let games = issue
            .related_mod
            .split('|')
            .map(|s| s.trim().to_ascii_uppercase())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        return format!(
            "Allowed games: {}",
            if games.is_empty() {
                "N/A".to_string()
            } else {
                games
            }
        );
    }
    details_format_target(&issue.related_mod, issue.related_component)
}

fn details_parse_or_targets_from_reason(reason: &str) -> Option<Vec<String>> {
    let prefix = "Requires one of:";
    let body = reason.strip_prefix(prefix)?.trim();
    let parts = body
        .split(" OR ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.len() > 1 { Some(parts) } else { None }
}

fn details_is_duplicate_selection_issue(issue: &CompatIssueDisplay) -> bool {
    issue.code.eq_ignore_ascii_case("RULE_HIT")
        && (issue
            .reason
            .to_ascii_lowercase()
            .contains("selected multiple times")
            || issue
                .raw_evidence
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case("selected_set_duplicate"))
}

fn details_parse_component_u32(value: &str) -> Option<u32> {
    value.trim().parse::<u32>().ok()
}

fn details_display_name_from_tp2(tp2_ref: &str) -> String {
    let file = if let Some(idx) = tp2_ref.rfind(['/', '\\']) {
        &tp2_ref[idx + 1..]
    } else {
        tp2_ref
    };
    let stem = file.strip_suffix(".tp2").unwrap_or(file);
    let stem = stem.strip_prefix("setup-").unwrap_or(stem);
    if stem.is_empty() {
        return tp2_ref.to_string();
    }
    stem.to_string()
}

fn details_format_target(mod_name: &str, component: Option<u32>) -> String {
    match component {
        Some(id) => format!("{mod_name} #{id}"),
        None => mod_name.to_string(),
    }
}
