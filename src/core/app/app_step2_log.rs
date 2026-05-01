// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashSet;
use std::path::PathBuf;

use crate::app::controller::log_apply::{apply_log_to_mods, normalize_path_key};
use crate::app::state::{Step1State, Step2LogPendingDownload, WizardState};
use crate::mods::component::Component;
use crate::mods::log_file::LogFile;

pub(crate) fn apply_saved_weidu_log_selection(state: &mut WizardState) {
    match state.step1.game_install.as_str() {
        "BG2EE" => {
            let log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, log_path)
        }
        "EET" => {
            let bgee_log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, bgee_log_path);
            let bg2ee_log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, bg2ee_log_path);
        }
        _ => {
            let log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, log_path)
        }
    }
}

pub(crate) fn apply_weidu_log_selection_from_path(
    state: &mut WizardState,
    bgee: bool,
    log_path: Option<PathBuf>,
) {
    let Some(path) = log_path else {
        state.step2.scan_status = "No WeiDU log selected".to_string();
        return;
    };

    let log = match LogFile::from_path(&path) {
        Ok(v) => v,
        Err(err) => {
            state.step2.scan_status = format!("Failed to parse log: {err}");
            return;
        }
    };

    let mut next_order = state.step2.next_selection_order;
    match (state.step1.game_install.as_str(), bgee) {
        ("EET", true) => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bgee_mods);
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bg2ee_mods);
        }
        (_, true) => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bgee_mods);
        }
        _ => {
            crate::app::compat_logic::clear_step2_compat_state(&mut state.step2.bg2ee_mods);
        }
    }
    let matched = match (state.step1.game_install.as_str(), bgee) {
        ("EET", true) => {
            let picked_bgee = apply_log_to_mods(
                &mut state.step2.bgee_mods,
                &log,
                None,
                true,
                &mut next_order,
            );
            let allow = HashSet::from([normalize_path_key(r"EET\EET.TP2")]);
            let picked_eet_core = apply_log_to_mods(
                &mut state.step2.bg2ee_mods,
                &log,
                Some(&allow),
                false,
                &mut next_order,
            );
            picked_bgee + picked_eet_core
        }
        (_, true) => apply_log_to_mods(
            &mut state.step2.bgee_mods,
            &log,
            None,
            true,
            &mut next_order,
        ),
        _ => apply_log_to_mods(
            &mut state.step2.bg2ee_mods,
            &log,
            None,
            true,
            &mut next_order,
        ),
    };
    state.step2.next_selection_order = next_order;
    let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut state.step2.bgee_mods,
        &mut state.step2.bg2ee_mods,
    );
    let label = if bgee { "BGEE" } else { "BG2EE" };
    state
        .step2
        .log_pending_downloads
        .retain(|pending| pending.game_tab != label);
    state
        .step2
        .log_pending_downloads
        .extend(build_log_pending_downloads(state, &log, label));
    if bgee {
        state.step2.review_edit_bgee_log_applied = true;
    } else {
        state.step2.review_edit_bg2ee_log_applied = true;
    }
    let pending = state.step2.log_pending_downloads.len();
    state.step2.scan_status = compat_error.map_or_else(
        || format!("{label} selected from log: {matched}, pending downloads: {pending}"),
        |err| format!(
            "{label} selected from log: {matched}, pending downloads: {pending} (compat rules load failed: {err})"
        ),
    );
    state.clear_last_step2_sync_signature();
}

fn build_log_pending_downloads(
    state: &WizardState,
    log: &LogFile,
    game_tab: &str,
) -> Vec<Step2LogPendingDownload> {
    let mods = if game_tab == "BGEE" {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let installed_tp2 = mods
        .iter()
        .map(|mod_state| crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file))
        .collect::<HashSet<_>>();
    let mut pending = Vec::new();
    let mut seen = HashSet::new();
    for component in log.components() {
        push_log_pending_download(&mut pending, &mut seen, &installed_tp2, component, game_tab);
    }
    pending
}

fn push_log_pending_download(
    pending: &mut Vec<Step2LogPendingDownload>,
    seen: &mut HashSet<String>,
    installed_tp2: &HashSet<String>,
    component: &Component,
    game_tab: &str,
) {
    let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&component.tp_file);
    if tp2.is_empty() || installed_tp2.contains(&tp2) || !seen.insert(tp2) {
        return;
    }
    let label = if component.name.trim().is_empty() {
        component.tp_file.clone()
    } else {
        component.name.clone()
    };
    let requested_version = requested_version_text(&component.version);
    pending.push(Step2LogPendingDownload {
        game_tab: game_tab.to_string(),
        tp_file: component.tp_file.clone(),
        label,
        requested_version,
    });
}

fn requested_version_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !looks_like_requested_version(trimmed) {
        return None;
    }
    Some(trimmed.to_string())
}

fn looks_like_requested_version(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    if lower.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return true;
    }
    if lower.starts_with('v') && lower.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()) {
        return true;
    }
    if lower.starts_with("version-")
        && lower
            .chars()
            .nth("version-".len())
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        return true;
    }
    for prefix in ["alpha", "beta", "rc", "pre"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = rest.trim_start_matches([' ', '-', '_']);
            if rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    if let Some((branch, commit)) = lower.split_once('@') {
        return !branch.is_empty()
            && commit.len() >= 7
            && commit.chars().all(|ch| ch.is_ascii_hexdigit());
    }
    false
}

pub(crate) fn resolve_bgee_weidu_log_path(s: &Step1State) -> Option<PathBuf> {
    if s.have_weidu_logs && !s.bgee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bgee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bgee_log_folder.trim()
    } else {
        s.bgee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}

pub(crate) fn resolve_bg2_weidu_log_path(s: &Step1State) -> Option<PathBuf> {
    if s.have_weidu_logs && !s.bg2ee_log_file.trim().is_empty() {
        return Some(PathBuf::from(s.bg2ee_log_file.trim()));
    }
    let folder = if s.game_install == "EET" {
        s.eet_bg2ee_log_folder.trim()
    } else {
        s.bg2ee_log_folder.trim()
    };
    if folder.is_empty() {
        None
    } else {
        Some(PathBuf::from(folder).join("weidu.log"))
    }
}

#[cfg(test)]
mod tests {
    use super::requested_version_text;

    #[test]
    fn keeps_version_like_labels() {
        assert_eq!(requested_version_text("v29").as_deref(), Some("v29"));
        assert_eq!(requested_version_text("1.57").as_deref(), Some("1.57"));
        assert_eq!(
            requested_version_text("Alpha 3").as_deref(),
            Some("Alpha 3")
        );
        assert_eq!(
            requested_version_text("master@149ac53b1470").as_deref(),
            Some("master@149ac53b1470")
        );
    }

    #[test]
    fn drops_component_text_labels() {
        assert_eq!(requested_version_text("BG1 Prologue Expansion"), None);
        assert_eq!(requested_version_text("Main Component"), None);
        assert_eq!(
            requested_version_text("Book 1 - Quiet Before the Storm"),
            None
        );
        assert_eq!(requested_version_text("EE and SoD"), None);
    }
}
