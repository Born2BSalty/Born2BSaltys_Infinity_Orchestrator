// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step2ModState};
use crate::ui::step3::compat_modal_step3::compat_model::normalize_mod_key;

use super::WizardApp;

const STEP2_DYNAMIC_SOURCE: &str = "step2_checked_order_validator";

pub(super) fn apply_step2_checked_order_issues(app: &mut WizardApp) {
    clear_dynamic_compat(&mut app.state.step2.bgee_mods);
    clear_dynamic_compat(&mut app.state.step2.bg2ee_mods);

    let active_mods = if app.state.step2.active_game_tab == "BG2EE" {
        &mut app.state.step2.bg2ee_mods
    } else {
        &mut app.state.step2.bgee_mods
    };

    for issue in &app.state.compat.issues {
        apply_issue_to_mods(active_mods, issue);
    }
}

fn clear_dynamic_compat(mods: &mut [Step2ModState]) {
    for mod_state in mods {
        for component in &mut mod_state.components {
            let is_dynamic = component
                .compat_source
                .as_deref()
                .map(|s| s.starts_with(STEP2_DYNAMIC_SOURCE))
                .unwrap_or(false);
            if !is_dynamic {
                continue;
            }
            component.disabled = false;
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.compat_graph = None;
            component.compat_evidence = None;
            component.disabled_reason = None;
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}

fn apply_issue_to_mods(mods: &mut [Step2ModState], issue: &CompatIssueDisplay) {
    let issue_mod_key = normalize_mod_key(&issue.affected_mod);
    for mod_state in mods {
        if normalize_mod_key(&mod_state.tp_file) != issue_mod_key {
            continue;
        }
        for component in &mut mod_state.components {
            if let Some(issue_comp) = issue.affected_component
                && component.component_id.trim().parse::<u32>().ok() != Some(issue_comp)
            {
                continue;
            }
            if component.compat_kind.is_some() && component.compat_source.as_deref() != Some(STEP2_DYNAMIC_SOURCE) {
                continue;
            }
            component.compat_kind = Some(if issue.code.eq_ignore_ascii_case("REQ_MISSING") {
                "missing_dep".to_string()
            } else if issue.code.eq_ignore_ascii_case("GAME_MISMATCH") {
                "game_mismatch".to_string()
            } else if issue.code.eq_ignore_ascii_case("CONDITIONAL") {
                "conditional".to_string()
            } else if issue.code.eq_ignore_ascii_case("INCLUDED") {
                "included".to_string()
            } else if issue.code.eq_ignore_ascii_case("ORDER_BLOCK") {
                "order_block".to_string()
            } else if issue.is_blocking {
                "conflict".to_string()
            } else {
                "warning".to_string()
            });
            component.compat_source = Some(format!("{STEP2_DYNAMIC_SOURCE} | {}", issue.source));
            component.compat_related_mod = Some(issue.related_mod.clone());
            component.compat_related_component = issue.related_component.map(|v| v.to_string());
            component.compat_graph = Some(format!(
                "{} #{} {} {} #{}",
                normalize_mod_key(&issue.affected_mod),
                issue.affected_component.unwrap_or_default(),
                issue.code.to_ascii_lowercase(),
                normalize_mod_key(&issue.related_mod),
                issue.related_component.unwrap_or_default()
            ));
            component.compat_evidence = issue.raw_evidence.clone();
            if !issue.reason.trim().is_empty() {
                component.disabled_reason = Some(issue.reason.clone());
            }
        }
        mod_state.checked = mod_state
            .components
            .iter()
            .filter(|c| !c.disabled)
            .all(|c| c.checked);
    }
}
