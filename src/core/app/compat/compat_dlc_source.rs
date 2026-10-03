// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::btree_map::Entry as BTreeMapEntry;
use std::collections::hash_map::Entry as HashMapEntry;
use std::path::Path;

use crate::app::source_check::{self, SodDlcState, SourceGame, SourceReport};
use crate::app::state::{Step1State, Step2ModState, Step3ItemState};

use super::compat_rule_runtime::{collect_step2_active_items, normalize_mod_key};
use super::compat_step3_rules::{Step3CompatMarker, marker_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceProbe {
    pub(crate) game: SourceGame,
    pub(crate) report: SourceReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DlcSourceCheck {
    probes: BTreeMap<String, SourceProbe>,
}

pub(crate) fn refresh_source_check(step1: &mut Step1State) -> bool {
    let mut wanted: Vec<(&str, SourceGame)> = Vec::with_capacity(5);
    for candidate in [
        (step1.bgee_game_folder.trim(), SourceGame::Bgee),
        (step1.eet_bgee_game_folder.trim(), SourceGame::Bgee),
        (step1.bg2ee_game_folder.trim(), SourceGame::Bg2ee),
        (step1.eet_bg2ee_game_folder.trim(), SourceGame::Bg2ee),
        (step1.iwdee_game_folder.trim(), SourceGame::Iwdee),
    ] {
        if !candidate.0.is_empty() && !wanted.iter().any(|(source, _)| *source == candidate.0) {
            wanted.push(candidate);
        }
    }
    let probes = &mut step1.dlc_source_check.probes;
    let up_to_date = probes.iter().all(|(key, probe)| {
        wanted
            .iter()
            .any(|(source, game)| *source == key.as_str() && *game == probe.game)
    }) && wanted
        .iter()
        .all(|(source, game)| probes.get(*source).is_some_and(|probe| probe.game == *game));
    if up_to_date {
        return false;
    }
    probes.retain(|key, probe| {
        wanted
            .iter()
            .any(|(source, game)| *source == key.as_str() && *game == probe.game)
    });
    for (source, game) in wanted {
        if let BTreeMapEntry::Vacant(slot) = probes.entry(source.to_string()) {
            slot.insert(SourceProbe {
                game,
                report: source_check::inspect(Path::new(source), game),
            });
        }
    }
    true
}

pub(crate) fn invalidate_source_check(step1: &mut Step1State) {
    step1.dlc_source_check.probes.clear();
}

#[must_use]
pub(crate) fn probed_game_version(
    step1: &Step1State,
    folder: &str,
) -> Option<crate::app::game_version::GameVersion> {
    step1
        .dlc_source_check
        .probes
        .get(folder.trim())
        .and_then(|probe| probe.report.game_version)
}

#[must_use]
pub(crate) fn bgee_source_for<'a>(step1: &'a Step1State, game: &str) -> &'a str {
    if game == "EET" {
        let plain = step1.bgee_game_folder.trim();
        if !plain.is_empty() {
            return plain;
        }
        step1.eet_bgee_game_folder.trim()
    } else {
        step1.bgee_game_folder.trim()
    }
}

#[must_use]
pub(crate) fn bg2ee_source_for<'a>(step1: &'a Step1State, game: &str) -> &'a str {
    if game == "EET" {
        let plain = step1.bg2ee_game_folder.trim();
        if !plain.is_empty() {
            return plain;
        }
        step1.eet_bg2ee_game_folder.trim()
    } else {
        step1.bg2ee_game_folder.trim()
    }
}

fn required(step1: &Step1State) -> bool {
    required_for_game(step1, &step1.game_install)
}

fn required_for_game(step1: &Step1State, game: &str) -> bool {
    matches!(game, "BGEE" | "EET")
        && step1
            .dlc_source_check
            .probes
            .get(bgee_source_for(step1, game))
            .is_some_and(|probe| matches!(probe.report.sod, SodDlcState::Unmerged { .. }))
}

fn is_merger(tp_file: &str, component: &str) -> bool {
    normalize_mod_key(tp_file) == "dlcmerger" && matches!(component.trim(), "1" | "3")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceNoticeSeverity {
    Info,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceRemedy {
    OrderMerger,
    ChangeSource,
    CleanSource,
    SetSourceFolder,
    SetGlobalModsFolder,
    WrongGameVersion,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceNotice {
    pub(crate) severity: SourceNoticeSeverity,
    pub(crate) text: String,
    pub(crate) remedy: SourceRemedy,
}

pub(crate) fn preview_issue(
    step1: &Step1State,
    preview: &crate::app::modlist_share::ModlistSharePreview,
) -> Option<SourceNotice> {
    if !matches!(preview.game_install.as_str(), "BGEE" | "EET") {
        return None;
    }
    let sod = &step1
        .dlc_source_check
        .probes
        .get(bgee_source_for(step1, &preview.game_install))?
        .report
        .sod;
    if matches!(sod, SodDlcState::NotApplicable) {
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
    let merger_position = bgee
        .iter()
        .position(|item| is_merger(&item.tp_file, &item.component));
    if let Some(position) = merger_position {
        return match (position, sod) {
            (0, SodDlcState::Unmerged { .. }) => Some(SourceNotice {
                severity: SourceNoticeSeverity::Info,
                text: "This modlist needs Siege of Dragonspear. Your BGEE source has the DLC archive; DLC Merger merges it during the install.".to_string(),
                remedy: SourceRemedy::OrderMerger,
            }),
            (_, SodDlcState::Unmerged { .. }) => Some(SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: "Move DLC Merger (#1 or #3) first in the BGEE installation order.".to_string(),
                remedy: SourceRemedy::OrderMerger,
            }),
            (_, SodDlcState::Merged) => Some(SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: "This modlist includes DLC Merger, but your BGEE source already has Siege of Dragonspear merged. DLC Merger stops with 'already merged'; remove it before installing.".to_string(),
                remedy: SourceRemedy::ChangeSource,
            }),
            (_, SodDlcState::Absent) => Some(SourceNotice {
                severity: SourceNoticeSeverity::Warning,
                text: "This modlist needs Siege of Dragonspear, and your BGEE source does not have it. DLC Merger will fail; install from a source that includes the DLC.".to_string(),
                remedy: SourceRemedy::ChangeSource,
            }),
            (_, SodDlcState::NotApplicable) => None,
        };
    }
    let has_tweaks_anywhere = has_tweaks(&bgee)
        || preview.game_install == "EET" && has_tweaks(&parse(&preview.bg2ee_log_text));
    if has_tweaks_anywhere && matches!(sod, SodDlcState::Unmerged { .. }) {
        return Some(SourceNotice {
            severity: SourceNoticeSeverity::Warning,
            text: "Your BGEE source contains DLC that needs merging. This modlist includes CDTweaks, which requires DLC Merger for this source.".to_string(),
            remedy: SourceRemedy::OrderMerger,
        });
    }
    None
}

#[must_use]
pub(crate) fn residue_issue(step1: &Step1State, game_install: &str) -> Option<SourceNotice> {
    let folders = crate::app::game_authority::source_game_folders(step1, game_install);
    let sentences = folders
        .iter()
        .filter(|(_, folder)| !folder.is_empty())
        .filter_map(|(label, folder)| {
            let probe = step1.dlc_source_check.probes.get(*folder)?;
            (!probe.report.residue.is_clean()).then(|| {
                format!(
                    "Your {label} source at {folder} is not a clean install: {}.",
                    probe.report.residue.describe()
                )
            })
        })
        .collect::<Vec<_>>();
    if sentences.is_empty() {
        return None;
    }
    Some(SourceNotice {
        severity: SourceNoticeSeverity::Warning,
        text: sentences.join(" "),
        remedy: SourceRemedy::CleanSource,
    })
}

#[must_use]
pub(crate) fn unresolved_suffix(unresolved: &[String]) -> String {
    if unresolved.is_empty() {
        return String::new();
    }
    let names = unresolved.join(", ");
    if unresolved.len() == 1 {
        format!(" 1 mod has no download source: {names}.")
    } else {
        format!(
            " {} mods have no download source: {names}.",
            unresolved.len()
        )
    }
}

#[must_use]
pub(crate) fn unresolved_sources_issue(unresolved: &[String]) -> Option<SourceNotice> {
    if unresolved.is_empty() {
        return None;
    }
    let pronoun = if unresolved.len() == 1 { "it" } else { "them" };
    Some(SourceNotice {
        severity: SourceNoticeSeverity::Warning,
        text: format!(
            "{} BIO will not be able to download {pronoun}.",
            unresolved_suffix(unresolved).trim()
        ),
        remedy: SourceRemedy::None,
    })
}

fn marker(step1: &Step1State, kind: &str, component: &str) -> Step3CompatMarker {
    let sod = step1
        .dlc_source_check
        .probes
        .get(bgee_source_for(step1, &step1.game_install))
        .map(|probe| &probe.report.sod);
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
        raw_evidence: match sod {
            Some(SodDlcState::Unmerged { archive }) => Some(format!(
                "Unmerged Siege of Dragonspear archive in BGEE source: {}",
                archive.display()
            )),
            _ => None,
        },
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
