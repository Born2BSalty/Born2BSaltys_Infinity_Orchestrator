// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;

use crate::app::added_mods;
use crate::app::controller::log_apply::{
    LogApplyReport, LogLineOutcome, apply_log_to_mods, log_line_allow_keys, normalize_path_key,
};
use crate::app::game_authority::{self, GameSlot};
use crate::app::mod_downloads::{self, ModDownloadsLoad, SourceTier, SourceTiers};
use crate::app::state::{
    LogPendingOrigin, Step1State, Step2ComponentState, Step2LogLine, Step2LogLineOutcome,
    Step2LogPendingDownload, Step2LogTarget, Step2LogUnticked, Step2ModState, Step2State,
    WizardState,
};
use crate::mods::component::Component;
use crate::mods::log_file::LogFile;

pub(crate) const IMPORTED_LOG_FILE_NAME: &str = "imported.log";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogShortfall {
    pub missing_mods: usize,
    pub unselectable_components: usize,
}

pub(crate) fn apply_saved_weidu_log_selection(state: &mut WizardState) {
    match state.step1.game_install.as_str() {
        "BG2EE" => {
            let log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, log_path);
        }
        "EET" => {
            let bgee_log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, bgee_log_path);
            let bg2ee_log_path = resolve_bg2_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, false, bg2ee_log_path);
        }
        _ => {
            let log_path = resolve_bgee_weidu_log_path(&state.step1);
            apply_weidu_log_selection_from_path(state, true, log_path);
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
    let mut logged = BTreeMap::new();
    let matched = apply_log_to_trees(state, bgee, &log, &mut next_order, &mut logged);
    state.step2.next_selection_order = next_order;
    let compat_error = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut state.step2.bgee_mods,
        &mut state.step2.bg2ee_mods,
    );
    let label = if bgee {
        game_authority::first_slot_tab(&state.step1.game_install)
    } else {
        "BG2EE"
    };
    replace_tab_lines(&mut state.step2.log_apply.lines, label, logged);
    state
        .step2
        .log_pending_downloads
        .retain(|pending| pending.game_tab != label);
    state
        .step2
        .log_pending_downloads
        .extend(build_log_pending_downloads(state, &log, label));
    if bgee {
        state.step2.log_apply.review_edit_bgee_applied = true;
    } else {
        state.step2.log_apply.review_edit_bg2ee_applied = true;
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

fn apply_log_to_trees(
    state: &mut WizardState,
    bgee: bool,
    log: &LogFile,
    next_order: &mut usize,
    logged: &mut BTreeMap<usize, LoggedLine>,
) -> usize {
    let trees = TreePair {
        game_install: &state.step1.game_install,
        first: &mut state.step2.bgee_mods,
        second: &mut state.step2.bg2ee_mods,
    };
    let mode = ApplyMode {
        allow: None,
        reset: true,
    };
    apply_log_to_tree_pair(trees, bgee, log, mode, next_order, logged)
}

struct TreePair<'a> {
    game_install: &'a str,
    first: &'a mut [Step2ModState],
    second: &'a mut [Step2ModState],
}

#[derive(Clone, Copy)]
struct ApplyMode<'a> {
    allow: Option<&'a HashSet<String>>,
    reset: bool,
}

fn apply_log_to_tree_pair(
    trees: TreePair<'_>,
    first_slot: bool,
    log: &LogFile,
    mode: ApplyMode<'_>,
    next_order: &mut usize,
    logged: &mut BTreeMap<usize, LoggedLine>,
) -> usize {
    let TreePair {
        game_install,
        first,
        second,
    } = trees;
    match (game_install, first_slot) {
        ("EET", true) => {
            let picked_first = apply_log_to_mods(first, log, mode.allow, mode.reset, next_order);
            let eet_core_allow = std::iter::once(normalize_path_key(r"EET\EET.TP2"))
                .filter(|key| mode.allow.is_none_or(|allow| allow.contains(key)))
                .collect::<HashSet<_>>();
            let picked_eet_core =
                apply_log_to_mods(second, log, Some(&eet_core_allow), false, next_order);
            merge_log_report(logged, GameSlot::First, picked_first)
                + merge_log_report(logged, GameSlot::Second, picked_eet_core)
        }
        (_, true) => merge_log_report(
            logged,
            GameSlot::First,
            apply_log_to_mods(first, log, mode.allow, mode.reset, next_order),
        ),
        _ => merge_log_report(
            logged,
            GameSlot::Second,
            apply_log_to_mods(second, log, mode.allow, mode.reset, next_order),
        ),
    }
}

#[must_use]
pub(crate) fn tab_for_slot(game_install: &str, first_slot: bool) -> &'static str {
    if first_slot {
        game_authority::first_slot_tab(game_install)
    } else {
        game_authority::TAB_BG2EE
    }
}

fn list_log_slots(game_install: &str) -> Vec<(&'static str, bool)> {
    game_authority::tabs_for_install(game_install)
        .iter()
        .map(|tab| (*tab, game_authority::slot_for_tab(tab) == GameSlot::First))
        .collect()
}

#[must_use]
pub(crate) fn imported_log_path(step1: &Step1State, first_slot: bool) -> Option<PathBuf> {
    let folder = match (step1.game_install.as_str(), first_slot) {
        ("EET", true) => &step1.eet_bgee_log_folder,
        ("EET", false) => &step1.eet_bg2ee_log_folder,
        (_, true) => &step1.bgee_log_folder,
        (_, false) => &step1.bg2ee_log_folder,
    };
    let folder = folder.trim();
    (!folder.is_empty()).then(|| PathBuf::from(folder).join(IMPORTED_LOG_FILE_NAME))
}

fn read_imported_log(step1: &Step1State, first_slot: bool) -> Option<LogFile> {
    let path = imported_log_path(step1, first_slot)?;
    if !path.is_file() {
        return None;
    }
    LogFile::from_path(&path).ok()
}

pub(crate) fn refresh_logged_missing(state: &mut WizardState) {
    if state.step1.installs_exactly_from_weidu_logs() || state.modlist_auto_build_active {
        return;
    }
    state
        .step2
        .log_pending_downloads
        .retain(|pending| pending.origin != LogPendingOrigin::WeiduLog);
    for (tab, first_slot) in list_log_slots(&state.step1.game_install) {
        let Some(log) = read_imported_log(&state.step1, first_slot) else {
            continue;
        };
        let pending = build_log_pending_downloads(state, &log, tab);
        state.step2.log_pending_downloads.extend(pending);
    }
}

#[must_use]
pub(crate) fn log_import_shortfall(
    state: &WizardState,
    first_slot: bool,
    log: &LogFile,
) -> LogShortfall {
    let mut first = state.step2.bgee_mods.clone();
    let mut second = state.step2.bg2ee_mods.clone();
    let trees = TreePair {
        game_install: &state.step1.game_install,
        first: &mut first,
        second: &mut second,
    };
    let mode = ApplyMode {
        allow: None,
        reset: true,
    };
    let mut next_order = 0;
    let mut logged = BTreeMap::new();
    apply_log_to_tree_pair(trees, first_slot, log, mode, &mut next_order, &mut logged);
    let unselectable_components = logged
        .values()
        .filter(|line| !matches!(line.outcome, Step2LogLineOutcome::Ticked(_)))
        .count();
    let tab = tab_for_slot(&state.step1.game_install, first_slot);
    LogShortfall {
        missing_mods: build_log_pending_downloads(state, log, tab).len(),
        unselectable_components,
    }
}

#[must_use]
pub(crate) fn logged_ticks_for_run(step2: &Step2State) -> BTreeSet<String> {
    let missing = step2.logged_missing_keys();
    let scope = step2.update_selected_download_scope.as_deref();
    step2
        .update_selected_update_assets
        .iter()
        .map(|asset| mod_downloads::normalize_mod_download_tp2(&asset.tp_file))
        .filter(|key| missing.contains(key) && scope.is_none_or(|scope| scope == key))
        .collect()
}

#[must_use]
pub(crate) fn fetch_set_ticks_logged(step2: &Step2State) -> bool {
    let missing = step2.logged_missing_keys();
    step2
        .update_selected_update_assets
        .iter()
        .any(|asset| missing.contains(&mod_downloads::normalize_mod_download_tp2(&asset.tp_file)))
}

#[must_use]
pub(crate) fn missing_scope_skipped_assets(step2: &Step2State) -> HashSet<usize> {
    let missing = step2.logged_missing_keys();
    step2
        .update_selected_update_assets
        .iter()
        .enumerate()
        .filter(|(_, asset)| {
            !missing.contains(&mod_downloads::normalize_mod_download_tp2(&asset.tp_file))
        })
        .map(|(index, _)| index)
        .collect()
}

fn arrived_allow_keys(log: &LogFile, arrived: &BTreeSet<String>) -> HashSet<String> {
    log.components()
        .iter()
        .filter(|component| {
            arrived.contains(&mod_downloads::normalize_mod_download_tp2(
                &component.tp_file,
            ))
        })
        .flat_map(log_line_allow_keys)
        .collect()
}

pub(crate) fn tick_logged_mods(
    state: &mut WizardState,
    first_slot: bool,
    log: &LogFile,
    allow: &HashSet<String>,
) -> usize {
    match (state.step1.game_install.as_str(), first_slot) {
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
    let mut next_order = state.step2.next_selection_order;
    let mut logged = BTreeMap::new();
    let trees = TreePair {
        game_install: &state.step1.game_install,
        first: &mut state.step2.bgee_mods,
        second: &mut state.step2.bg2ee_mods,
    };
    let mode = ApplyMode {
        allow: Some(allow),
        reset: false,
    };
    let matched =
        apply_log_to_tree_pair(trees, first_slot, log, mode, &mut next_order, &mut logged);
    state.step2.next_selection_order = next_order;
    let label = tab_for_slot(&state.step1.game_install, first_slot);
    replace_tab_lines(&mut state.step2.log_apply.lines, label, logged);
    matched
}

pub(crate) fn tick_arrived_logged_mods(
    state: &mut WizardState,
    arrived: &BTreeSet<String>,
) -> bool {
    if arrived.is_empty() {
        return false;
    }
    let mut ticked_any_tab = false;
    for (_, first_slot) in list_log_slots(&state.step1.game_install) {
        let Some(log) = read_imported_log(&state.step1, first_slot) else {
            continue;
        };
        let allow = arrived_allow_keys(&log, arrived);
        if allow.is_empty() {
            continue;
        }
        if !ticked_any_tab {
            state.step2.log_apply.lines.clear();
        }
        ticked_any_tab = true;
        tick_logged_mods(state, first_slot, &log, &allow);
    }
    if !ticked_any_tab {
        return false;
    }
    if let Some(err) = crate::app::compat_logic::apply_step2_compat_rules(
        &state.step1,
        &mut state.step2.bgee_mods,
        &mut state.step2.bg2ee_mods,
    ) {
        tracing::warn!(
            target = "orchestrator",
            "compat rules load failed after the logged tick: {err}"
        );
    }
    state.clear_last_step2_sync_signature();
    true
}

#[must_use]
pub fn ticked_log_line_count(step2: &Step2State) -> usize {
    step2
        .log_apply
        .lines
        .iter()
        .filter(|line| unticked_reason(step2, line).is_none())
        .count()
}

struct LoggedLine {
    tp_file: String,
    component_id: String,
    outcome: Step2LogLineOutcome,
}

fn outcome_from_report(tree: GameSlot, outcome: LogLineOutcome) -> Step2LogLineOutcome {
    match outcome {
        LogLineOutcome::Ticked(targets) => Step2LogLineOutcome::Ticked(
            targets
                .into_iter()
                .map(|(mod_index, component_index)| Step2LogTarget {
                    tree,
                    mod_index,
                    component_index,
                })
                .collect(),
        ),
        LogLineOutcome::NoComponent(mod_index) => {
            Step2LogLineOutcome::NoComponent { tree, mod_index }
        }
        LogLineOutcome::NoMod => Step2LogLineOutcome::NoMod,
    }
}

fn merge_outcome(current: &mut Step2LogLineOutcome, other: Step2LogLineOutcome) {
    match (current, other) {
        (Step2LogLineOutcome::Ticked(targets), Step2LogLineOutcome::Ticked(more)) => {
            targets.extend(more);
        }
        (Step2LogLineOutcome::Ticked(_), _)
        | (
            Step2LogLineOutcome::NoComponent { .. },
            Step2LogLineOutcome::NoComponent { .. } | Step2LogLineOutcome::NoMod,
        ) => {}
        (current, other) => *current = other,
    }
}

fn merge_log_report(
    logged: &mut BTreeMap<usize, LoggedLine>,
    tree: GameSlot,
    report: LogApplyReport,
) -> usize {
    for result in report.lines {
        let outcome = outcome_from_report(tree, result.outcome);
        match logged.entry(result.line) {
            Entry::Vacant(entry) => {
                entry.insert(LoggedLine {
                    tp_file: result.tp_file,
                    component_id: result.component_id,
                    outcome,
                });
            }
            Entry::Occupied(mut entry) => merge_outcome(&mut entry.get_mut().outcome, outcome),
        }
    }
    report.matched
}

fn replace_tab_lines(
    list: &mut Vec<Step2LogLine>,
    game_tab: &str,
    logged: BTreeMap<usize, LoggedLine>,
) {
    list.retain(|line| line.game_tab != game_tab);
    list.extend(logged.into_values().map(|line| Step2LogLine {
        game_tab: game_tab.to_string(),
        mod_label: tp2_file_name_upper(&line.tp_file),
        component_id: line.component_id,
        outcome: line.outcome,
    }));
    list.sort_by_key(|line| game_authority::slot_for_tab(&line.game_tab) != GameSlot::First);
}

fn tree_mods(step2: &Step2State, tree: GameSlot) -> &[Step2ModState] {
    match tree {
        GameSlot::First => &step2.bgee_mods,
        GameSlot::Second => &step2.bg2ee_mods,
    }
}

fn logged_component(
    step2: &Step2State,
    target: Step2LogTarget,
) -> Option<(&Step2ModState, &Step2ComponentState)> {
    let mod_state = tree_mods(step2, target.tree).get(target.mod_index)?;
    Some((mod_state, mod_state.components.get(target.component_index)?))
}

#[must_use]
pub fn unticked_log_lines(step2: &Step2State) -> Vec<Step2LogUnticked> {
    step2
        .log_apply
        .lines
        .iter()
        .filter_map(|line| {
            let (mod_name, reason) = unticked_reason(step2, line)?;
            Some(Step2LogUnticked {
                game_tab: line.game_tab.clone(),
                mod_name,
                component_id: line.component_id.clone(),
                reason,
            })
        })
        .collect()
}

fn not_on_disk(line: &Step2LogLine) -> (String, String) {
    (line.mod_label.clone(), "mod not on disk".to_string())
}

fn unticked_reason(step2: &Step2State, line: &Step2LogLine) -> Option<(String, String)> {
    match &line.outcome {
        Step2LogLineOutcome::Ticked(targets) => {
            let components = targets
                .iter()
                .filter_map(|target| logged_component(step2, *target))
                .collect::<Vec<_>>();
            if components.iter().any(|(_, component)| component.checked) {
                return None;
            }
            Some(components.first().map_or_else(
                || not_on_disk(line),
                |(mod_state, component)| {
                    (
                        mod_state.name.clone(),
                        component_exclusion_reason(component),
                    )
                },
            ))
        }
        Step2LogLineOutcome::NoComponent { tree, mod_index } => {
            Some(tree_mods(step2, *tree).get(*mod_index).map_or_else(
                || not_on_disk(line),
                |mod_state| {
                    (
                        mod_state.name.clone(),
                        "component not in this version".to_string(),
                    )
                },
            ))
        }
        Step2LogLineOutcome::NoMod => Some(not_on_disk(line)),
    }
}

fn tp2_file_name_upper(tp_file: &str) -> String {
    tp_file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(tp_file)
        .to_ascii_uppercase()
}

#[must_use]
pub(crate) fn component_exclusion_reason(component: &Step2ComponentState) -> String {
    component
        .disabled_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map_or_else(
            || "excluded by compatibility rules".to_string(),
            std::string::ToString::to_string,
        )
}

pub(crate) fn reseed_added_mod_pending_downloads(state: &mut WizardState) {
    let Some(path) = mod_downloads::active_modlist_downloads_path() else {
        return;
    };
    let modlist_text = std::fs::read_to_string(path).unwrap_or_default();
    let added = added_mods::load_added_mods();
    if added.is_empty() {
        return;
    }
    let tiers = mod_downloads::load_source_tiers(&modlist_text);
    let sources = mod_downloads::load_mod_download_sources();
    seed_added_mod_pending_downloads(state, &sources, &tiers, &added);
}

pub(crate) fn seed_added_mod_pending_downloads(
    state: &mut WizardState,
    sources: &ModDownloadsLoad,
    tiers: &SourceTiers,
    added: &BTreeSet<String>,
) {
    let scanned: HashSet<String> = state
        .step2
        .bgee_mods
        .iter()
        .chain(state.step2.bg2ee_mods.iter())
        .map(|mod_state| mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file))
        .filter(|key| !key.is_empty())
        .collect();
    let mut listed: HashSet<String> = state
        .step2
        .log_pending_downloads
        .iter()
        .map(|pending| mod_downloads::normalize_mod_download_tp2(&pending.tp_file))
        .collect();
    let game_tab = state.step2.active_game_tab.clone();
    for source in &sources.sources {
        if !matches!(
            tiers.tier_of(&source.tp2, &source.source_id),
            SourceTier::Modlist | SourceTier::User
        ) {
            continue;
        }
        let tp2 = mod_downloads::normalize_mod_download_tp2(&source.tp2);
        if !added.contains(&tp2) {
            continue;
        }
        let scanned_under_alias = source
            .aliases
            .iter()
            .any(|alias| scanned.contains(&mod_downloads::normalize_mod_download_tp2(alias)));
        if tp2.is_empty() || scanned.contains(&tp2) || scanned_under_alias {
            continue;
        }
        if !listed.insert(tp2.clone()) {
            continue;
        }
        let label = if source.name.trim().is_empty() {
            tp2.clone()
        } else {
            source.name.trim().to_string()
        };
        state
            .step2
            .log_pending_downloads
            .push(Step2LogPendingDownload {
                game_tab: game_tab.clone(),
                tp_file: format!("{tp2}.tp2"),
                label,
                requested_version: None,
                origin: LogPendingOrigin::AddedMod,
            });
    }
}

#[must_use]
pub(crate) fn build_log_pending_downloads(
    state: &WizardState,
    log: &LogFile,
    game_tab: &str,
) -> Vec<Step2LogPendingDownload> {
    let mods = if game_authority::slot_for_tab(game_tab) == GameSlot::First {
        &state.step2.bgee_mods
    } else {
        &state.step2.bg2ee_mods
    };
    let mod_download_sources = crate::app::mod_downloads::load_mod_download_sources();
    let mut installed_tp2 = HashSet::new();
    for mod_state in mods {
        let tp2 = crate::app::mod_downloads::normalize_mod_download_tp2(&mod_state.tp_file);
        if tp2.is_empty() {
            continue;
        }
        installed_tp2.insert(tp2.clone());
        for source in mod_download_sources.find_sources(&tp2) {
            installed_tp2.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                &source.tp2,
            ));
            for alias in source.aliases {
                installed_tp2.insert(crate::app::mod_downloads::normalize_mod_download_tp2(
                    &alias,
                ));
            }
        }
    }
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
        origin: LogPendingOrigin::WeiduLog,
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
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    struct TempRoot {
        path: PathBuf,
    }

    impl TempRoot {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("bio_step2_log_test_{}_{id}", std::process::id()));
            std::fs::create_dir_all(&path).expect("create temp root");
            Self { path }
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn step2_log_label_names_the_lists_own_tab() {
        let root = TempRoot::new();
        let log_path = root.path.join("weidu.log");
        std::fs::write(&log_path, "").expect("write empty log");

        let mut state = WizardState {
            step1: Step1State {
                game_install: "IWDEE".to_string(),
                ..Step1State::default()
            },
            ..WizardState::default()
        };
        apply_weidu_log_selection_from_path(&mut state, true, Some(log_path));

        assert!(
            state
                .step2
                .scan_status
                .starts_with("IWDEE selected from log"),
            "unexpected status: {}",
            state.step2.scan_status
        );
    }

    fn github_block(tp2: &str, name: &str) -> String {
        format!(
            "[[mods]]\nname = \"{name}\"\ntp2 = \"{tp2}\"\n\n  [[mods.sources]]\n  id = \"primary\"\n  label = \"Primary\"\n  type = \"github\"\n  url = \"https://github.com/owner/{tp2}\"\n  repo = \"owner/{tp2}\"\n  default = true\n"
        )
    }

    fn scanned_mod(tp_file: &str) -> crate::app::state::Step2ModState {
        crate::app::state::Step2ModState {
            name: tp_file.to_string(),
            tp_file: tp_file.to_string(),
            tp2_path: tp_file.to_string(),
            readme_path: None,
            ini_path: None,
            web_url: None,
            package_marker: None,
            latest_checked_version: None,
            update_locked: false,
            mod_prompt_summary: None,
            mod_prompt_events: Vec::new(),
            checked: false,
            hidden_components: Vec::new(),
            components: Vec::new(),
        }
    }

    fn seed_from_texts(
        state: &mut WizardState,
        default_text: &str,
        user_text: &str,
        modlist_text: &str,
        added: &[&str],
    ) {
        let sources = mod_downloads::load_mod_download_sources_from_texts(
            default_text,
            user_text,
            modlist_text,
        );
        let tiers = mod_downloads::source_tiers_from_texts(default_text, user_text, modlist_text);
        let added: BTreeSet<String> = added.iter().map(ToString::to_string).collect();
        seed_added_mod_pending_downloads(state, &sources, &tiers, &added);
    }

    #[test]
    fn reseed_ignores_modlist_blocks_not_in_the_added_list() {
        let modlist_text = github_block("widget", "Widget");
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(&mut state, "", "", &modlist_text, &[]);
        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);
        assert_eq!(state.step2.log_pending_downloads.len(), 1);
    }

    #[test]
    fn reseed_adds_a_card_for_a_saved_unscanned_mod() {
        let modlist_text = format!(
            "{}\n{}",
            github_block("widget", "Widget"),
            github_block("widget", "Widget").replace("id = \"primary\"", "id = \"fork\"")
        );
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "widget.tp2".to_string(),
                label: "Widget".to_string(),
                requested_version: None,
                origin: LogPendingOrigin::AddedMod,
            }]
        );

        seed_from_texts(&mut state, "", "", &modlist_text, &["widget"]);
        assert_eq!(state.step2.log_pending_downloads.len(), 1);
    }

    #[test]
    fn reseed_skips_scanned_mods() {
        let mut state = WizardState::default();
        state
            .step2
            .bgee_mods
            .push(scanned_mod("widget/setup-widget.tp2"));

        seed_from_texts(
            &mut state,
            "",
            "",
            &github_block("widget", "Widget"),
            &["widget"],
        );

        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );
    }

    #[test]
    fn reseed_adds_a_card_for_an_added_mod_saved_to_my_default() {
        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();

        seed_from_texts(
            &mut state,
            "",
            &github_block("gadget", "Gadget"),
            "",
            &["gadget"],
        );

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "gadget.tp2".to_string(),
                label: "Gadget".to_string(),
                requested_version: None,
                origin: LogPendingOrigin::AddedMod,
            }]
        );
    }

    struct IsolatedAmbient {
        previous_modlist_dir: Option<PathBuf>,
        root: TempRoot,
    }

    impl IsolatedAmbient {
        fn create() -> Self {
            let root = TempRoot::new();
            let config_dir = root.path.join("config");
            let modlist_dir = root.path.join("modlist");
            std::fs::create_dir_all(&config_dir).expect("create temp config dir");
            std::fs::create_dir_all(&modlist_dir).expect("create temp modlist dir");
            let previous_modlist_dir = mod_downloads::active_modlist_dir();
            crate::platform_defaults::set_config_dir_override(Some(config_dir));
            mod_downloads::set_active_modlist_dir(Some(modlist_dir));
            Self {
                previous_modlist_dir,
                root,
            }
        }
    }

    impl Drop for IsolatedAmbient {
        fn drop(&mut self) {
            mod_downloads::set_active_modlist_dir(self.previous_modlist_dir.take());
            crate::platform_defaults::clear_config_dir_override_if(&self.root.path.join("config"));
        }
    }

    #[test]
    fn reseed_runs_for_a_list_whose_own_sources_file_is_absent() {
        let _lock = mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ambient = IsolatedAmbient::create();
        let user_path = mod_downloads::mod_downloads_user_path();
        assert!(user_path.starts_with(&ambient.root.path), "{user_path:?}");
        std::fs::create_dir_all(user_path.parent().expect("user path parent"))
            .expect("create user sources dir");
        std::fs::write(&user_path, github_block("gadget", "Gadget")).expect("write user sources");
        added_mods::record_added_mod("gadget").expect("record gadget");
        let modlist_sources = mod_downloads::active_modlist_downloads_path().expect("modlist path");
        assert!(!modlist_sources.exists());

        let mut state = WizardState::default();
        state.step2.active_game_tab = "BG2EE".to_string();
        reseed_added_mod_pending_downloads(&mut state);

        assert_eq!(
            state.step2.log_pending_downloads,
            vec![Step2LogPendingDownload {
                game_tab: "BG2EE".to_string(),
                tp_file: "gadget.tp2".to_string(),
                label: "Gadget".to_string(),
                requested_version: None,
                origin: LogPendingOrigin::AddedMod,
            }]
        );
    }

    #[test]
    fn reseed_skips_a_bio_default_block() {
        let default_text = github_block("gizmo", "Gizmo");
        assert_ne!(
            mod_downloads::load_mod_download_sources_from_texts(&default_text, "", "")
                .find_sources("gizmo")
                .len(),
            0
        );
        let mut state = WizardState::default();

        seed_from_texts(&mut state, &default_text, "", "", &["gizmo"]);

        assert!(
            state.step2.log_pending_downloads.is_empty(),
            "{:?}",
            state.step2.log_pending_downloads
        );
    }

    fn scanned_component(component_id: &str) -> Step2ComponentState {
        Step2ComponentState {
            component_id: component_id.to_string(),
            label: format!("Component {component_id}"),
            weidu_group: None,
            collapsible_group: None,
            collapsible_group_is_umbrella: false,
            collapsible_group_combinable: false,
            raw_line: String::new(),
            prompt_summary: None,
            prompt_events: Vec::new(),
            is_meta_mode_component: false,
            disabled: false,
            compat_kind: None,
            compat_source: None,
            compat_related_mod: None,
            compat_related_component: None,
            compat_graph: None,
            compat_evidence: None,
            disabled_reason: None,
            checked: false,
            selected_order: None,
        }
    }

    fn scanned_mod_with(name: &str, tp2_path: &str, component_ids: &[&str]) -> Step2ModState {
        let mut mod_state = scanned_mod(tp2_path.rsplit('/').next().unwrap_or(tp2_path));
        mod_state.name = name.to_string();
        mod_state.tp2_path = tp2_path.to_string();
        mod_state.components = component_ids
            .iter()
            .map(|id| scanned_component(id))
            .collect();
        mod_state
    }

    fn unticked(
        game_tab: &str,
        mod_name: &str,
        component_id: &str,
        reason: &str,
    ) -> Step2LogUnticked {
        Step2LogUnticked {
            game_tab: game_tab.to_string(),
            mod_name: mod_name.to_string(),
            component_id: component_id.to_string(),
            reason: reason.to_string(),
        }
    }

    fn apply_log_text(state: &mut WizardState, root: &TempRoot, bgee: bool, text: &str) {
        static LOG_COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = root.path.join(format!("weidu_{n}.log"));
        std::fs::write(&path, text).expect("write log");
        apply_weidu_log_selection_from_path(state, bgee, Some(path));
    }

    fn ambient_lock() -> std::sync::MutexGuard<'static, ()> {
        mod_downloads::AMBIENT_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn install_state(game_install: &str) -> WizardState {
        WizardState {
            step1: Step1State {
                game_install: game_install.to_string(),
                ..Step1State::default()
            },
            ..WizardState::default()
        }
    }

    #[test]
    fn eet_bgee_log_does_not_list_the_eet_core_line() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = install_state("EET");
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1"])];
        state.step2.bg2ee_mods = vec![scanned_mod_with("EET", "EET/EET.TP2", &["0"])];

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~EET/EET.TP2~ #0 #0 // EET core\n~X/SETUP-X.TP2~ #0 #1 // X one\n",
        );

        assert!(state.step2.bg2ee_mods[0].components[0].checked);
        assert_eq!(unticked_log_lines(&state.step2), Vec::new());
    }

    #[test]
    fn eet_a_second_log_unticking_the_core_line_lists_it() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = install_state("EET");
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1"])];
        state.step2.bg2ee_mods = vec![
            scanned_mod_with("EET", "EET/EET.TP2", &["0"]),
            scanned_mod_with("Z", "Z/SETUP-Z.TP2", &["0"]),
        ];

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~EET/EET.TP2~ #0 #0 // EET core\n~X/SETUP-X.TP2~ #0 #1 // X one\n",
        );
        assert!(state.step2.bg2ee_mods[0].components[0].checked);
        assert_eq!(unticked_log_lines(&state.step2), Vec::new());

        apply_log_text(
            &mut state,
            &ambient.root,
            false,
            "~Z/SETUP-Z.TP2~ #0 #0 // Z zero\n",
        );

        let core = &state.step2.bg2ee_mods[0].components[0];
        assert!(!core.checked);
        assert!(state.step2.bgee_mods[0].components[0].checked);
        assert!(state.step2.bg2ee_mods[1].components[0].checked);
        assert_eq!(
            unticked_log_lines(&state.step2),
            vec![unticked(
                "BGEE",
                "EET",
                "0",
                &component_exclusion_reason(core)
            )]
        );
    }

    #[test]
    fn judging_reads_the_current_trees() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = install_state("BGEE");
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1"])];

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~X/SETUP-X.TP2~ #0 #1 // X one\n",
        );
        assert_eq!(unticked_log_lines(&state.step2), Vec::new());

        state.step2.bgee_mods[0].components[0].checked = false;
        assert_eq!(
            unticked_log_lines(&state.step2),
            vec![unticked(
                "BGEE",
                "X",
                "1",
                "excluded by compatibility rules"
            )]
        );

        state.step2.bgee_mods[0].components[0].checked = true;
        assert_eq!(unticked_log_lines(&state.step2), Vec::new());
    }

    #[test]
    fn out_of_range_targets_judge_as_mod_not_on_disk() {
        let mut step2 = Step2State::default();
        let line = |outcome| Step2LogLine {
            game_tab: "BGEE".to_string(),
            mod_label: "SETUP-X.TP2".to_string(),
            component_id: "1".to_string(),
            outcome,
        };
        step2.log_apply.lines = vec![
            line(Step2LogLineOutcome::Ticked(vec![Step2LogTarget {
                tree: GameSlot::First,
                mod_index: 3,
                component_index: 0,
            }])),
            line(Step2LogLineOutcome::NoComponent {
                tree: GameSlot::Second,
                mod_index: 3,
            }),
        ];

        assert_eq!(
            unticked_log_lines(&step2),
            vec![
                unticked("BGEE", "SETUP-X.TP2", "1", "mod not on disk"),
                unticked("BGEE", "SETUP-X.TP2", "1", "mod not on disk"),
            ]
        );
    }

    const FIXTURE_RULE_REASON: &str = "Fixture rule: not needed here.";

    #[test]
    fn a_compat_excluded_line_lists_the_rule_reason() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        std::fs::write(
            ambient
                .root
                .path
                .join("config")
                .join("step2_compat_rules_user.toml"),
            format!(
                "[[rules]]\nmod = \"fixturemod\"\ncomponent_id = \"2\"\nkind = \"not_needed\"\nmessage = \"{FIXTURE_RULE_REASON}\"\n"
            ),
        )
        .expect("write user compat rules");
        let mut state = install_state("BGEE");
        state.step2.bgee_mods = vec![scanned_mod_with(
            "Fixture Mod",
            "FIXTUREMOD/SETUP-FIXTUREMOD.TP2",
            &["2"],
        )];

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~FIXTUREMOD/SETUP-FIXTUREMOD.TP2~ #0 #2 // Fixture two\n",
        );

        let component = &state.step2.bgee_mods[0].components[0];
        assert!(!component.checked);
        assert_eq!(
            component.disabled_reason.as_deref().map(str::trim),
            Some(FIXTURE_RULE_REASON)
        );
        assert_eq!(
            unticked_log_lines(&state.step2),
            vec![unticked("BGEE", "Fixture Mod", "2", FIXTURE_RULE_REASON)]
        );
    }

    #[test]
    fn reapplying_a_tab_replaces_its_unticked_lines() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = install_state("EET");
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1"])];

        apply_log_text(
            &mut state,
            &ambient.root,
            false,
            "~W/SETUP-W.TP2~ #0 #3 // W\n",
        );
        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~Y/SETUP-Y.TP2~ #0 #0 // Y\n",
        );
        assert_eq!(
            unticked_log_lines(&state.step2),
            vec![
                unticked("BGEE", "SETUP-Y.TP2", "0", "mod not on disk"),
                unticked("BG2EE", "SETUP-W.TP2", "3", "mod not on disk"),
            ]
        );

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #7 // X seven\n~z/setup-z.tp2~ #0 #4 // Z\n",
        );
        assert_eq!(
            unticked_log_lines(&state.step2),
            vec![
                unticked("BGEE", "X", "7", "component not in this version"),
                unticked("BGEE", "SETUP-Z.TP2", "4", "mod not on disk"),
                unticked("BG2EE", "SETUP-W.TP2", "3", "mod not on disk"),
            ]
        );
    }

    #[test]
    fn every_line_ticked_lists_nothing() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = install_state("BGEE");
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1", "2"])];

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n",
        );

        assert_eq!(unticked_log_lines(&state.step2), Vec::new());
        assert!(
            state.step2.bgee_mods[0]
                .components
                .iter()
                .all(|component| component.checked)
        );
    }

    fn added_entry(tp_file: &str) -> Step2LogPendingDownload {
        Step2LogPendingDownload {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: tp_file.to_string(),
            requested_version: None,
            origin: LogPendingOrigin::AddedMod,
        }
    }

    fn state_with_imported_log(root: &TempRoot, text: &str) -> WizardState {
        let mut state = install_state("BGEE");
        let slot = root.path.join("slot_bgee");
        std::fs::create_dir_all(&slot).expect("slot folder");
        std::fs::write(slot.join(IMPORTED_LOG_FILE_NAME), text).expect("imported log");
        state.step1.bgee_log_folder = slot.to_string_lossy().to_string();
        state
    }

    #[test]
    fn missing_is_rebuilt_from_imported_logs_on_settle() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = state_with_imported_log(
            &ambient.root,
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n",
        );
        state.step2.bgee_mods = vec![scanned_mod_with("X", "X/SETUP-X.TP2", &["1"])];
        state.step2.log_pending_downloads = vec![added_entry("gadget.tp2")];

        refresh_logged_missing(&mut state);
        let rebuilt = state.step2.log_pending_downloads.clone();
        assert_eq!(
            state.step2.logged_missing_keys(),
            BTreeSet::from(["y".to_string()])
        );
        assert!(rebuilt.iter().any(
            |entry| entry.tp_file == "gadget.tp2" && entry.origin == LogPendingOrigin::AddedMod
        ));

        state
            .step2
            .log_pending_downloads
            .retain(|entry| entry.origin == LogPendingOrigin::AddedMod);
        refresh_logged_missing(&mut state);
        assert_eq!(state.step2.log_pending_downloads, rebuilt);
        refresh_logged_missing(&mut state);
        assert_eq!(state.step2.log_pending_downloads, rebuilt);

        state
            .step2
            .bgee_mods
            .push(scanned_mod_with("Y", "Y/SETUP-Y.TP2", &["0"]));
        refresh_logged_missing(&mut state);
        assert_eq!(state.step2.logged_missing_count(), 0);
    }

    #[test]
    fn an_exact_log_list_keeps_its_saved_log_pending_entries() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = state_with_imported_log(&ambient.root, "~Y/SETUP-Y.TP2~ #0 #0 // Y\n");
        state.step1.install_mode = Step1State::INSTALL_MODE_EXACT_WEIDU_LOGS.to_string();
        state.step1.sync_install_mode_flags();
        assert!(state.step1.installs_exactly_from_weidu_logs());
        let saved_flow_entry = Step2LogPendingDownload {
            origin: LogPendingOrigin::WeiduLog,
            ..added_entry("SETUP-W.TP2")
        };
        state.step2.log_pending_downloads = vec![saved_flow_entry.clone()];

        refresh_logged_missing(&mut state);

        assert_eq!(state.step2.log_pending_downloads, vec![saved_flow_entry]);
    }

    fn asset(tp_file: &str) -> crate::app::state::Step2UpdateAsset {
        crate::app::state::Step2UpdateAsset {
            game_tab: "BGEE".to_string(),
            tp_file: tp_file.to_string(),
            label: tp_file.to_string(),
            source_id: "primary".to_string(),
            tag: "v1".to_string(),
            asset_name: format!("{tp_file}.zip"),
            asset_url: format!("https://example.test/{tp_file}.zip"),
            installed_source_ref: None,
        }
    }

    #[test]
    fn the_missing_scope_fetch_skips_assets_of_other_mods() {
        let mut step2 = Step2State {
            log_pending_downloads: vec![
                Step2LogPendingDownload {
                    origin: LogPendingOrigin::WeiduLog,
                    ..added_entry("SETUP-Y.TP2")
                },
                added_entry("gadget.tp2"),
            ],
            update_selected_update_assets: vec![
                asset("SETUP-X.TP2"),
                asset("SETUP-Y.TP2"),
                asset("gadget.tp2"),
                asset("Y/setup-y.tp2"),
            ],
            ..Step2State::default()
        };

        let skipped = missing_scope_skipped_assets(&step2);
        let fetched = (0..step2.update_selected_update_assets.len())
            .filter(|index| !skipped.contains(index))
            .map(|index| {
                mod_downloads::normalize_mod_download_tp2(
                    &step2.update_selected_update_assets[index].tp_file,
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(fetched, BTreeSet::from(["y".to_string()]));

        step2.update_selected_download_scope = None;
        assert_eq!(
            logged_ticks_for_run(&step2),
            BTreeSet::from(["y".to_string()])
        );
    }

    #[test]
    fn green_count_reads_logged_lines_ticked_after_rules() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        std::fs::write(
            ambient
                .root
                .path
                .join("config")
                .join("step2_compat_rules_user.toml"),
            format!(
                "[[rules]]\nmod = \"fixturemod\"\ncomponent_id = \"2\"\nkind = \"not_needed\"\nmessage = \"{FIXTURE_RULE_REASON}\"\n"
            ),
        )
        .expect("write user compat rules");
        let mut state = install_state("EET");
        state.step2.bgee_mods = vec![scanned_mod_with(
            "Fixture Mod",
            "FIXTUREMOD/SETUP-FIXTUREMOD.TP2",
            &["1", "2"],
        )];
        state.step2.bg2ee_mods = vec![scanned_mod_with("Z", "Z/SETUP-Z.TP2", &["0"])];
        state.step2.bg2ee_mods[0].components[0].checked = true;

        apply_log_text(
            &mut state,
            &ambient.root,
            true,
            "~FIXTUREMOD/SETUP-FIXTUREMOD.TP2~ #0 #1 // Fixture one\n~FIXTUREMOD/SETUP-FIXTUREMOD.TP2~ #0 #2 // Fixture two\n",
        );

        assert!(state.step2.bg2ee_mods[0].components[0].checked);
        assert!(!state.step2.bgee_mods[0].components[1].checked);
        assert_eq!(ticked_log_line_count(&state.step2), 1);
    }

    #[test]
    fn the_logged_tick_leaves_other_mods_and_other_lines_alone() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let mut state = state_with_imported_log(
            &ambient.root,
            "~X/SETUP-X.TP2~ #0 #1 // X one\n~X/SETUP-X.TP2~ #0 #2 // X two\n~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n",
        );
        state.step2.bgee_mods = vec![
            scanned_mod_with("X", "X/SETUP-X.TP2", &["1", "2"]),
            scanned_mod_with("Y", "Y/SETUP-Y.TP2", &["0"]),
        ];
        state.step2.bgee_mods[0].components[0].checked = true;
        state.step2.bgee_mods[0].components[0].selected_order = Some(4);
        state.step2.next_selection_order = 5;

        assert!(tick_arrived_logged_mods(
            &mut state,
            &BTreeSet::from(["y".to_string()])
        ));

        let x = &state.step2.bgee_mods[0];
        assert_eq!(x.components[0].selected_order, Some(4));
        assert!(!x.components[1].checked);
        let y = &state.step2.bgee_mods[1].components[0];
        assert!(y.checked);
        assert_eq!(y.selected_order, Some(5));
        assert_eq!(ticked_log_line_count(&state.step2), 1);
        assert_eq!(unticked_log_lines(&state.step2), Vec::new());

        assert!(!tick_arrived_logged_mods(&mut state, &BTreeSet::new()));
        assert!(!tick_arrived_logged_mods(
            &mut state,
            &BTreeSet::from(["absent".to_string()])
        ));
    }

    #[test]
    fn an_eet_first_log_tick_reaches_the_eet_core_only_when_it_arrived() {
        let _lock = ambient_lock();
        let ambient = IsolatedAmbient::create();
        let slot = ambient.root.path.join("slot_eet_bgee");
        std::fs::create_dir_all(&slot).expect("slot folder");
        std::fs::write(
            slot.join(IMPORTED_LOG_FILE_NAME),
            "~EET/EET.TP2~ #0 #0 // EET core\n~Y/SETUP-Y.TP2~ #0 #0 // Y zero\n",
        )
        .expect("imported log");
        let mut state = install_state("EET");
        state.step1.eet_bgee_log_folder = slot.to_string_lossy().to_string();
        state.step2.bgee_mods = vec![scanned_mod_with("Y", "Y/SETUP-Y.TP2", &["0"])];
        state.step2.bg2ee_mods = vec![scanned_mod_with("EET", "EET/EET.TP2", &["0"])];

        assert!(tick_arrived_logged_mods(
            &mut state,
            &BTreeSet::from(["y".to_string()])
        ));
        assert!(state.step2.bgee_mods[0].components[0].checked);
        assert!(!state.step2.bg2ee_mods[0].components[0].checked);

        assert!(tick_arrived_logged_mods(
            &mut state,
            &BTreeSet::from(["eet".to_string()])
        ));
        assert!(state.step2.bg2ee_mods[0].components[0].checked);
    }

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
