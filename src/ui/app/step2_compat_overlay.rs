// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::{CompatIssueDisplay, Step2ModState};

use super::WizardApp;

const STEP2_DYNAMIC_SOURCE: &str = "step2_checked_order_validator";

pub(super) fn apply_step2_checked_order_issues(app: &mut WizardApp) {
    clear_dynamic_compat(&mut app.state.step2.bgee_mods);
    clear_dynamic_compat(&mut app.state.step2.bg2ee_mods);

    for issue in &app.state.compat.issues {
        apply_issue_to_mods(&mut app.state.step2.bgee_mods, issue);
        apply_issue_to_mods(&mut app.state.step2.bg2ee_mods, issue);
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
            component.compat_kind = None;
            component.compat_source = None;
            component.compat_related_mod = None;
            component.compat_related_component = None;
            component.disabled_reason = None;
        }
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
            } else if issue.is_blocking {
                "conflict".to_string()
            } else {
                "warning".to_string()
            });
            component.compat_source = Some(format!("{STEP2_DYNAMIC_SOURCE} | {}", issue.source));
            component.compat_related_mod = Some(issue.related_mod.clone());
            component.compat_related_component = issue.related_component.map(|v| v.to_string());
            if !issue.reason.trim().is_empty() {
                component.disabled_reason = Some(issue.reason.clone());
            }
        }
    }
}

fn normalize_mod_key(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let file = if let Some(idx) = lower.rfind(['/', '\\']) {
        &lower[idx + 1..]
    } else {
        &lower
    };
    let without_ext = file.strip_suffix(".tp2").unwrap_or(file);
    without_ext
        .strip_prefix("setup-")
        .unwrap_or(without_ext)
        .to_string()
}
