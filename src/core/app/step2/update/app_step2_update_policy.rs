// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use crate::app::state::WizardState;
use crate::parser::weidu_version::{normalize_version_text, parse_version};

pub(crate) fn mark_update_available(state: &mut WizardState, game_tab: &str, tp_file: &str) {
    let mods = if game_tab == "BGEE" {
        &mut state.step2.bgee_mods
    } else {
        &mut state.step2.bg2ee_mods
    };
    if let Some(mod_state) = mods
        .iter_mut()
        .find(|mod_state| mod_state.tp_file == tp_file)
        && !mod_state.update_locked
    {
        mod_state.package_marker = Some('+');
    }
}

pub(crate) fn source_ref_is_update(tp_file: &str, source_id: &str, latest_ref: &str) -> bool {
    let source_id = source_id.trim().to_ascii_lowercase();
    super::app_step2_update_source_refs::load_installed_source_id_and_ref(tp_file).is_some_and(
        |(installed_source_id, installed_ref)| {
            installed_source_id.trim().to_ascii_lowercase() == source_id
                && installed_ref.trim() != latest_ref.trim()
        },
    )
}

pub(crate) fn source_ref_matches(tp_file: &str, source_id: &str, latest_ref: &str) -> bool {
    let source_id = source_id.trim().to_ascii_lowercase();
    super::app_step2_update_source_refs::load_installed_source_id_and_ref(tp_file).is_some_and(
        |(installed_source_id, installed_ref)| {
            installed_source_id.trim().to_ascii_lowercase() == source_id
                && installed_ref.trim() == latest_ref.trim()
        },
    )
}

pub(crate) fn mod_has_current_version(state: &WizardState, game_tab: &str, tp_file: &str) -> bool {
    let mods = if game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    mods.iter()
        .find(|mod_state| mod_state.tp_file == tp_file)
        .map(|mod_state| {
            mod_state
                .components
                .iter()
                .any(|component| parse_version(&component.raw_line).is_some())
        })
        .unwrap_or(false)
}

pub(crate) fn version_is_update(
    state: &WizardState,
    game_tab: &str,
    tp_file: &str,
    latest_tag: &str,
) -> bool {
    let mods = if game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let Some(mod_state) = mods.iter().find(|mod_state| mod_state.tp_file == tp_file) else {
        return false;
    };
    let Some(current) = mod_state
        .components
        .iter()
        .find_map(|component| parse_version(&component.raw_line))
    else {
        return false;
    };
    normalize_version_text(latest_tag) != normalize_version_text(&current)
}
