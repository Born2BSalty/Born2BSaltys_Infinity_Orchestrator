// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::ui::step2::prompt_popup_step2::open_toolbar_prompt_popup;
use crate::ui::step2::service_list_ops_step2::{clear_all, select_visible};
use crate::ui::step2::state_step2::active_mods_mut;
use crate::ui::step2::toolbar_compat_step2::{Step2ToolbarCompatSummary, Step2ToolbarIssueTarget};
use crate::ui::step5::service_diagnostics_support_step5::export_diagnostics;

pub(crate) fn export_diagnostics_from_step2(
    state: &mut WizardState,
    dev_mode: bool,
    exe_fingerprint: &str,
) {
    match export_diagnostics(state, None, dev_mode, exe_fingerprint) {
        Ok(path) => {
            state.step5.last_status_text = format!("Diagnostics exported: {}", path.display());
        }
        Err(err) => {
            state.step5.last_status_text = format!("Diagnostics export failed: {err}");
        }
    }
}

pub(crate) fn clear_all_and_refresh_compat(state: &mut WizardState) {
    let mut next_order = state.step2.next_selection_order;
    let mods = active_mods_mut(&mut state.step2);
    clear_all(mods, &mut next_order);
    state.step2.next_selection_order = next_order;
    state.step2.selected = None;
    refresh_compat(state);
}

pub(crate) fn select_visible_and_refresh_compat(state: &mut WizardState) {
    let filter = state.step2.search_query.trim().to_lowercase();
    let mut next_order = state.step2.next_selection_order;
    let mods = active_mods_mut(&mut state.step2);
    select_visible(mods, &filter, &mut next_order);
    state.step2.next_selection_order = next_order;
    refresh_compat(state);
}

pub(crate) fn collapse_all(state: &mut WizardState) {
    state.step2.collapse_default_open = false;
    state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
}

pub(crate) fn expand_all(state: &mut WizardState) {
    state.step2.collapse_default_open = true;
    state.step2.collapse_epoch = state.step2.collapse_epoch.saturating_add(1);
}

pub(crate) fn open_active_tab_issue(
    state: &mut WizardState,
    issue_summary: &Step2ToolbarCompatSummary,
    issue_target: Option<Step2ToolbarIssueTarget>,
) -> bool {
    let Some(target) = issue_target else {
        return false;
    };
    if state.step2.compat_popup_filter.eq_ignore_ascii_case("All") {
        state.step2.compat_popup_filter = issue_summary.dominant_filter.to_string();
    }
    state.step2.selected = Some(crate::app::state::Step2Selection::Component {
        game_tab: state.step2.active_game_tab.clone(),
        tp_file: target.tp_file,
        component_id: target.component_id,
        component_key: target.component_key,
    });
    state.step2.compat_popup_issue_override = None;
    state.step2.compat_popup_open = true;
    true
}

pub(crate) fn open_prompt_toolbar(state: &mut WizardState) {
    open_toolbar_prompt_popup(
        state,
        &format!("Prompt Components ({})", state.step2.active_game_tab),
    );
}

fn refresh_compat(state: &mut WizardState) {
    if let Some(err) = crate::ui::step2::service_compat_rules_step2::apply_compat_rules(
        &state.step1,
        &mut state.step2.bgee_mods,
        &mut state.step2.bg2ee_mods,
    ) {
        state.step2.scan_status = format!("Compat rules load failed: {err}");
    }
}
