// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::collections::hash_map::Entry as HashMapEntry;
use std::path::{Path, PathBuf};

use crate::app::state::{Step1State, Step2ModState, Step3ItemState};

use super::compat_rule_runtime::{collect_step2_active_items, normalize_mod_key};
use super::compat_step3_rules::{Step3CompatMarker, marker_key};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DlcSourceCheck {
    source: Option<String>,
    archive: Option<PathBuf>,
}

pub(crate) fn refresh_source_check(step1: &mut Step1State) -> bool {
    let source = step1.bgee_game_folder.trim();
    if step1.dlc_source_check.source.as_deref() == Some(source) {
        return false;
    }
    step1.dlc_source_check = DlcSourceCheck {
        source: Some(source.to_string()),
        archive: if source.is_empty() {
            None
        } else {
            find_sod_archive(Path::new(source))
        },
    };
    true
}

fn child_named(root: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(root).ok()?.flatten().find_map(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(name)
            .then(|| entry.path())
    })
}

fn find_sod_archive(root: &Path) -> Option<PathBuf> {
    child_named(root, "sod-dlc.zip")
        .filter(|path| path.is_file())
        .or_else(|| {
            child_named(root, "dlc")
                .and_then(|dlc| child_named(&dlc, "sod-dlc.zip"))
                .filter(|path| path.is_file())
        })
}

fn required(step1: &Step1State) -> bool {
    required_for_game(step1, &step1.game_install)
}

fn required_for_game(step1: &Step1State, game: &str) -> bool {
    matches!(game, "BGEE" | "EET")
        && step1.dlc_source_check.source.as_deref() == Some(step1.bgee_game_folder.trim())
        && step1.dlc_source_check.archive.is_some()
}

fn is_merger(tp_file: &str, component: &str) -> bool {
    normalize_mod_key(tp_file) == "dlcmerger" && matches!(component.trim(), "1" | "3")
}

pub(crate) fn preview_issue(
    step1: &Step1State,
    preview: &crate::app::modlist_share::ModlistSharePreview,
) -> Option<&'static str> {
    if !required_for_game(step1, &preview.game_install) {
        return None;
    }
    let parse = |text: &str| {
        text.lines()
            .map(str::trim)
            .filter(|line| line.starts_with('~'))
            .filter_map(|line| crate::mods::component::Component::parse_weidu_line(line).ok())
            .collect::<Vec<_>>()
    };
    let bgee = parse(&preview.bgee_log_text);
    let has_tweaks = |items: &[crate::mods::component::Component]| {
        items
            .iter()
            .any(|item| normalize_mod_key(&item.tp_file) == "cdtweaks")
    };
    if !(has_tweaks(&bgee)
        || preview.game_install == "EET" && has_tweaks(&parse(&preview.bg2ee_log_text)))
    {
        return None;
    }
    match bgee
        .iter()
        .position(|item| is_merger(&item.tp_file, &item.component))
    {
        None => Some(
            "Your BGEE source contains DLC that needs merging. This modlist includes CDTweaks, which requires DLC Merger for this source.",
        ),
        Some(0) => None,
        Some(_) => Some(
            "Your BGEE source contains DLC that needs merging. Move DLC Merger (#1 or #3) first in the BGEE installation order.",
        ),
    }
}

fn marker(step1: &Step1State, kind: &str, component: &str) -> Step3CompatMarker {
    Step3CompatMarker {
        kind: kind.to_string(),
        message: Some(if kind == "order_block" {
            "Install DLC Merger (#1 or #3) first in the BGEE installation order.".to_string()
        } else {
            "Requires DLC Merger first in the BGEE installation order: select Merge Siege of Dragonspear (#1) or All available DLCs (#3).".to_string()
        }),
        related_mod: Some("dlcmerger".to_string()),
        related_component: Some(component.to_string()),
        source: Some("BGEE source check".to_string()),
        raw_evidence: step1.dlc_source_check.archive.as_ref().map(|path| {
            format!(
                "Unmerged Siege of Dragonspear archive in BGEE source: {}",
                path.display()
            )
        }),
    }
}

pub(crate) fn apply_step2(
    step1: &Step1State,
    first: &mut [Step2ModState],
    second: &mut [Step2ModState],
) {
    if !required(step1) {
        return;
    }
    if collect_step2_active_items(first)
        .iter()
        .any(|item| is_merger(&item.tp_file, &item.component_id))
    {
        return;
    }
    let hit = marker(step1, "missing_dep", "1");
    for mod_state in first.iter_mut().chain(second.iter_mut()) {
        if normalize_mod_key(&mod_state.tp_file) != "cdtweaks" {
            continue;
        }
        for component in &mut mod_state.components {
            if component.compat_kind.is_some() {
                continue;
            }
            component.compat_kind = Some(hit.kind.clone());
            component.compat_source.clone_from(&hit.source);
            component.compat_related_mod.clone_from(&hit.related_mod);
            component
                .compat_related_component
                .clone_from(&hit.related_component);
            component.compat_evidence.clone_from(&hit.raw_evidence);
            component.disabled_reason.clone_from(&hit.message);
        }
    }
}

pub(crate) fn apply_step3(
    step1: &Step1State,
    _tab: &str,
    items: &[Step3ItemState],
    bgee_items: &[Step3ItemState],
    markers: &mut HashMap<String, Step3CompatMarker>,
) {
    if !required(step1) {
        return;
    }
    let merger = bgee_items
        .iter()
        .filter(|item| !item.is_parent)
        .enumerate()
        .find(|(_, item)| is_merger(&item.tp_file, &item.component_id));
    for item in items.iter().filter(|item| !item.is_parent) {
        if normalize_mod_key(&item.tp_file) != "cdtweaks" {
            continue;
        }
        let HashMapEntry::Vacant(slot) = markers.entry(marker_key(item)) else {
            continue;
        };
        let hit = match merger {
            None => marker(step1, "missing_dep", "1"),
            Some((order, dependency)) if order != 0 => {
                marker(step1, "order_block", &dependency.component_id)
            }
            Some(_) => continue,
        };
        slot.insert(hit);
    }
}

#[cfg(test)]
#[path = "compat_dlc_source_tests.rs"]
mod tests;
