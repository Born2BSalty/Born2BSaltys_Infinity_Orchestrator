// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::compat_issue::CompatIssue;
use crate::app::compat_step3_rules::Step3CompatMarker;
use crate::app::prompt_eval_context::build_prompt_eval_context;
use crate::app::prompt_popup_text::collect_step3_prompt_toolbar_entries;
use crate::app::state::{Step2Selection, Step3ItemState, WizardState};

#[derive(Clone)]
pub(crate) struct Step3ToolbarIssueTarget {
    pub(crate) tab_id: String,
    pub(crate) tp_file: String,
    pub(crate) component_id: String,
    pub(crate) component_key: String,
    pub(crate) issue: CompatIssue,
}

pub(crate) struct Step3ToolbarSummary {
    pub(crate) show_bgee: bool,
    pub(crate) show_bg2ee: bool,
    pub(crate) bgee_summary: (usize, bool),
    pub(crate) bg2ee_summary: (usize, bool),
    pub(crate) bgee_prompt_count: usize,
    pub(crate) bg2ee_prompt_count: usize,
    pub(crate) bgee_target: Option<Step3ToolbarIssueTarget>,
    pub(crate) bg2ee_target: Option<Step3ToolbarIssueTarget>,
    pub(crate) bgee_markers: std::collections::HashMap<String, Step3CompatMarker>,
    pub(crate) bg2ee_markers: std::collections::HashMap<String, Step3CompatMarker>,
    pub(crate) compat_rules_error: Option<String>,
}

pub(crate) fn build_toolbar_summary(state: &WizardState) -> Step3ToolbarSummary {
    let show_bgee = matches!(state.step1.game_install.as_str(), "BGEE" | "EET");
    let show_bg2ee = matches!(state.step1.game_install.as_str(), "BG2EE" | "EET");
    let bgee_markers = if show_bgee {
        crate::app::compat_step3_rules::collect_step3_compat_markers(
            &state.step1,
            "BGEE",
            &state.step2.bgee_mods,
            &state.step3.bgee_items,
        )
    } else {
        std::collections::HashMap::new()
    };
    let bg2ee_markers = if show_bg2ee {
        crate::app::compat_step3_rules::collect_step3_compat_markers(
            &state.step1,
            "BG2EE",
            &state.step2.bg2ee_mods,
            &state.step3.bg2ee_items,
        )
    } else {
        std::collections::HashMap::new()
    };
    let prompt_eval = build_prompt_eval_context(state);
    let bgee_prompt_count = if show_bgee {
        collect_step3_prompt_toolbar_entries(&state.step3.bgee_items, &prompt_eval)
            .into_iter()
            .map(|entry| entry.component_ids.len())
            .sum()
    } else {
        0
    };
    let bg2ee_prompt_count = if show_bg2ee {
        collect_step3_prompt_toolbar_entries(&state.step3.bg2ee_items, &prompt_eval)
            .into_iter()
            .map(|entry| entry.component_ids.len())
            .sum()
    } else {
        0
    };
    Step3ToolbarSummary {
        show_bgee,
        show_bg2ee,
        bgee_summary: tab_compat_summary(&bgee_markers),
        bg2ee_summary: tab_compat_summary(&bg2ee_markers),
        bgee_prompt_count,
        bg2ee_prompt_count,
        bgee_target: first_tab_issue_target("BGEE", &state.step3.bgee_items, &bgee_markers),
        bg2ee_target: first_tab_issue_target("BG2EE", &state.step3.bg2ee_items, &bg2ee_markers),
        bgee_markers,
        bg2ee_markers,
        compat_rules_error: crate::app::compat_rules::load_rules().error,
    }
}

pub(crate) fn tab_has_conflict(
    markers: &std::collections::HashMap<String, Step3CompatMarker>,
) -> bool {
    markers
        .values()
        .any(|marker| marker.kind.eq_ignore_ascii_case("conflict"))
}

pub(crate) fn open_toolbar_issue_popup(state: &mut WizardState, target: &Step3ToolbarIssueTarget) {
    state.step3.active_game_tab = target.tab_id.clone();
    state.step2.selected = Some(Step2Selection::Component {
        game_tab: target.tab_id.clone(),
        tp_file: target.tp_file.clone(),
        component_id: target.component_id.clone(),
        component_key: target.component_key.clone(),
    });
    state.step2.compat_popup_issue_override = Some(target.issue.clone());
    state.step2.compat_popup_open = true;
}

fn tab_compat_summary(
    markers: &std::collections::HashMap<String, Step3CompatMarker>,
) -> (usize, bool) {
    let has_blocking = markers
        .values()
        .any(|marker| marker_is_blocking(marker.kind.as_str()));
    (markers.len(), has_blocking)
}

fn first_tab_issue_target(
    tab_id: &str,
    items: &[Step3ItemState],
    markers: &std::collections::HashMap<String, Step3CompatMarker>,
) -> Option<Step3ToolbarIssueTarget> {
    let mut first_warning: Option<Step3ToolbarIssueTarget> = None;

    for item in items.iter().filter(|item| !item.is_parent) {
        let key = crate::app::compat_step3_rules::marker_key(item);
        let Some(marker) = markers.get(&key) else {
            continue;
        };
        let issue = crate::app::compat_step3_rules::marker_issue(marker);
        let target = Step3ToolbarIssueTarget {
            tab_id: tab_id.to_string(),
            tp_file: item.tp_file.clone(),
            component_id: item.component_id.clone(),
            component_key: item.raw_line.clone(),
            issue: issue.clone(),
        };
        if marker_is_blocking(marker.kind.as_str()) {
            return Some(target);
        }
        if first_warning.is_none() {
            first_warning = Some(target);
        }
    }

    first_warning
}

fn marker_is_blocking(kind: &str) -> bool {
    !matches!(
        kind,
        "included"
            | "not_needed"
            | "missing_dep"
            | "order_block"
            | "warning"
            | "deprecated"
            | "conditional"
    )
}
