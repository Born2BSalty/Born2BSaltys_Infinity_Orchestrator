// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::ui::state::WizardState;

use super::key::normalize_mod_key;
use super::query::current_issue_for_selection;

pub(crate) fn rule_source_open_path(state: &WizardState) -> Option<String> {
    let issue = current_issue_for_selection(state)?;
    let src = issue.source.trim();
    if let Some(tp2_token) = src.strip_prefix("TP2:") {
        let target_mod_key = normalize_mod_key(&issue.affected_mod);
        for mod_state in state
            .step2
            .bgee_mods
            .iter()
            .chain(state.step2.bg2ee_mods.iter())
        {
            if normalize_mod_key(&mod_state.tp_file) == target_mod_key
                && !mod_state.tp2_path.trim().is_empty()
            {
                return Some(mod_state.tp2_path.clone());
            }
        }
        let fallback = tp2_token.trim();
        if !fallback.is_empty() {
            return Some(fallback.to_string());
        }
        return None;
    }

    let mut path = src;
    if let Some((lhs, rhs)) = src.rsplit_once(':')
        && rhs.trim().chars().all(|c| c.is_ascii_digit())
    {
        path = lhs;
    }
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}
