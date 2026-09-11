// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Born2BSalty

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::state::{Step1State, Step2ComponentState, Step2ModState, Step3ItemState};

use super::super::compat_step3_rules::{Step3CompatMarker, marker_key};
use super::{apply_step2, apply_step3, preview_issue, refresh_source_check};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn fixture() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join("bio-dlc-source-tests");
    std::fs::create_dir_all(&root).expect("create fixture root");
    let path = root.join(format!(
        "{}-{timestamp}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).expect("create unique fixture");
    path
}

fn archive(root: &Path, relative: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("archive parent"))
        .expect("create archive directory");
    std::fs::write(path, b"synthetic archive existence fixture").expect("write fixture");
}

fn source_state(root: &Path, mode: &str) -> Step1State {
    let mut state = Step1State {
        bgee_game_folder: root.to_string_lossy().into_owned(),
        game_install: mode.to_string(),
        ..Step1State::default()
    };
    assert!(refresh_source_check(&mut state));
    state
}

fn component(id: &str, checked: bool) -> Step2ComponentState {
    Step2ComponentState {
        component_id: id.to_string(),
        label: id.to_string(),
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
        checked,
        selected_order: checked.then_some(1),
    }
}

fn mod_state(tp_file: &str, components: &[(&str, bool)]) -> Step2ModState {
    Step2ModState {
        name: tp_file.to_string(),
        tp_file: tp_file.to_string(),
        tp2_path: String::new(),
        readme_path: None,
        ini_path: None,
        web_url: None,
        package_marker: None,
        latest_checked_version: None,
        update_locked: false,
        mod_prompt_summary: None,
        mod_prompt_events: Vec::new(),
        checked: components.iter().any(|(_, checked)| *checked),
        hidden_components: Vec::new(),
        components: components
            .iter()
            .map(|(id, checked)| component(id, *checked))
            .collect(),
    }
}

fn item(tp_file: &str, id: &str) -> Step3ItemState {
    Step3ItemState {
        tp_file: tp_file.to_string(),
        component_id: id.to_string(),
        mod_name: tp_file.to_string(),
        component_label: id.to_string(),
        raw_line: String::new(),
        prompt_summary: None,
        prompt_events: Vec::new(),
        selected_order: 1,
        block_id: String::new(),
        is_parent: false,
        parent_placeholder: false,
    }
}

fn markers(
    state: &Step1State,
    tab: &str,
    items: &[Step3ItemState],
    bgee: &[Step3ItemState],
) -> HashMap<String, Step3CompatMarker> {
    let mut result = HashMap::new();
    apply_step3(state, tab, items, bgee, &mut result);
    result
}

fn needs_merge(state: &Step1State) -> bool {
    let tweaks = [item("cdtweaks.tp2", "2010")];
    let result = markers(state, "BGEE", &tweaks, &tweaks);
    result
        .get(&marker_key(&tweaks[0]))
        .is_some_and(|hit| hit.kind == "missing_dep")
}

#[test]
fn detects_root_and_dlc_archives_case_insensitively() {
    for relative in ["sod-dlc.zip", "DLC/SoD-DlC.ZIP"] {
        let root = fixture();
        archive(&root, relative);
        assert!(needs_merge(&source_state(&root, "BGEE")), "{relative}");
    }
}

#[test]
fn no_archive_or_archive_named_directory_does_not_require_merging() {
    let root = fixture();
    assert!(!needs_merge(&source_state(&root, "BGEE")));
    std::fs::create_dir(root.join("sod-dlc.zip")).expect("archive-shaped directory");
    assert!(!needs_merge(&source_state(&root, "BGEE")));
    archive(&root, "unrelated.zip");
    assert!(!needs_merge(&source_state(&root, "BGEE")));
}

#[test]
fn same_source_uses_cache_and_path_changes_clear_stale_requirement() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let mut state = source_state(&root, "BGEE");
    std::fs::rename(root.join("sod-dlc.zip"), root.join("merged-archive.backup"))
        .expect("move synthetic archive");
    assert!(!refresh_source_check(&mut state));
    assert!(needs_merge(&state), "same source must retain cached probe");
    let empty = fixture();
    state.bgee_game_folder = empty.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(!needs_merge(&state));
    state.bgee_game_folder = root.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(!needs_merge(&state), "returning to source must probe again");
}

#[test]
fn empty_and_missing_source_clear_cached_positive() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for replacement in [
        String::new(),
        root.join("absent").to_string_lossy().into_owned(),
    ] {
        let mut state = source_state(&root, "BGEE");
        state.bgee_game_folder = replacement;
        assert!(refresh_source_check(&mut state));
        assert!(!needs_merge(&state));
        assert!(!refresh_source_check(&mut state));
    }
}

#[test]
fn destination_archive_does_not_trigger_or_satisfy_source_requirement() {
    let source = fixture();
    let destination = fixture();
    archive(&destination, "sod-dlc.zip");
    let mut state = source_state(&source, "EET");
    state.eet_new_dir = destination.to_string_lossy().into_owned();
    state.eet_pre_dir.clone_from(&state.eet_new_dir);
    state.game.clone_from(&state.eet_new_dir);
    state.bg2ee_game_folder.clone_from(&state.eet_new_dir);
    assert!(!refresh_source_check(&mut state));
    assert!(!needs_merge(&state));
    let unmerged = fixture();
    archive(&unmerged, "sod-dlc.zip");
    state.bgee_game_folder = unmerged.to_string_lossy().into_owned();
    assert!(refresh_source_check(&mut state));
    assert!(needs_merge(&state));
    state.eet_new_dir = source.to_string_lossy().into_owned();
    assert!(!refresh_source_check(&mut state));
    assert!(needs_merge(&state));
}

#[test]
fn step2_marks_every_tweaks_component_on_both_tabs_only_when_required() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for mode in ["BGEE", "EET", "BG2EE", "IWDEE"] {
        let state = source_state(&root, mode);
        let mut first = [mod_state(
            "CDTWEAKS/SETUP-CDTWEAKS.TP2",
            &[("0", false), ("2010", true), ("99999", false)],
        )];
        let mut second = [
            mod_state("cdtweaks.tp2", &[("42", true)]),
            mod_state("unrelated.tp2", &[("0", true)]),
        ];
        apply_step2(&state, &mut first, &mut second);
        let expected = matches!(mode, "BGEE" | "EET").then_some("missing_dep");
        for component in first[0].components.iter().chain(&second[0].components) {
            assert_eq!(component.compat_kind.as_deref(), expected, "{mode}");
            if expected.is_some() {
                assert_eq!(component.compat_related_mod.as_deref(), Some("dlcmerger"));
                assert!(
                    component
                        .compat_evidence
                        .as_deref()
                        .is_some_and(|evidence| evidence.contains("sod-dlc.zip"))
                );
            }
        }
        assert!(second[1].components[0].compat_kind.is_none());
    }
}

#[test]
fn step2_only_checked_bgee_merger_one_or_three_satisfies() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    for id in ["1", "2", "3"] {
        for checked in [false, true] {
            for in_bgee in [false, true] {
                let merger = mod_state("DLCMERGER/SETUP-DLCMERGER.TP2", &[(id, checked)]);
                let mut first = vec![mod_state("cdtweaks.tp2", &[("2010", false)])];
                let mut second = vec![mod_state("cdtweaks.tp2", &[("42", true)])];
                if in_bgee {
                    first.push(merger);
                } else {
                    second.push(merger);
                }
                apply_step2(&state, &mut first, &mut second);
                let satisfied = in_bgee && checked && id != "2";
                for mods in [&first, &second] {
                    assert_eq!(
                        mods[0].components[0].compat_kind.is_none(),
                        satisfied,
                        "{id}, checked={checked}, bgee={in_bgee}"
                    );
                }
            }
        }
    }
}

#[test]
fn step3_same_phase_requires_merger_first_for_every_tweaks_component() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    for id in ["1", "3"] {
        let items = [
            item("cdtweaks.tp2", "2010"),
            item("DLCMERGER.TP2", id),
            item("cdtweaks.tp2", "42"),
        ];
        let result = markers(&state, "BGEE", &items, &items);
        assert_eq!(result.len(), 2);
        assert_eq!(result[&marker_key(&items[0])].kind, "order_block");
        assert_eq!(result[&marker_key(&items[2])].kind, "order_block");
        let reordered = [items[1].clone(), items[0].clone(), items[2].clone()];
        assert!(markers(&state, "BGEE", &reordered, &reordered).is_empty());
    }
}

#[test]
fn step3_eet_cross_phase_ignores_tab_local_order_numbers() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    let second = [item("cdtweaks.tp2", "2010"), item("cdtweaks.tp2", "42")];
    for id in ["1", "2", "3"] {
        let mut merger = item("dlcmerger.tp2", id);
        merger.selected_order = 99;
        let first = [merger];
        let result = markers(&state, "BG2EE", &second, &first);
        assert_eq!(result.len(), if id == "2" { 2 } else { 0 });
        assert!(result.values().all(|hit| hit.kind == "missing_dep"));
    }
    assert_eq!(markers(&state, "BG2EE", &second, &[]).len(), 2);
}

#[test]
fn step3_merger_before_tweaks_but_not_first_is_blocked_on_both_tabs() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    for id in ["1", "3"] {
        let first = [
            item("unrelated.tp2", "0"),
            item("dlcmerger.tp2", id),
            item("cdtweaks.tp2", "2010"),
        ];
        for tab in ["BGEE", "BG2EE"] {
            let result = markers(&state, tab, &first[2..], &first);
            assert_eq!(result[&marker_key(&first[2])].kind, "order_block");
        }
        let mut parent = item("dlcmerger.tp2", id);
        parent.is_parent = true;
        let reordered = [parent, first[1].clone(), first[2].clone()];
        assert!(markers(&state, "BGEE", &reordered, &reordered).is_empty());
    }
}

fn preview(
    game: &str,
    first: &str,
    second: &str,
) -> crate::app::modlist_share::ModlistSharePreview {
    crate::app::modlist_share::ModlistSharePreview {
        bio_version: String::new(),
        game_install: game.to_string(),
        install_mode: "custom".to_string(),
        bgee_entries: 0,
        bg2ee_entries: 0,
        has_source_overrides: false,
        has_installed_refs: false,
        bgee_log_text: first.to_string(),
        bg2ee_log_text: second.to_string(),
        source_overrides_text: String::new(),
        installed_refs_text: String::new(),
        mod_config_count: 0,
        mod_configs_text: String::new(),
        allow_auto_install: true,
        name: None,
        author: None,
        forked_from: Vec::new(),
    }
}

const TWEAKS_LOG: &str = "~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010 // Custom label: v1";
const OTHER_LOG: &str = "~OTHER/OTHER.TP2~ #0 #0 // CDTweaks DLCmerger: v1";
const MISSING: &str = "Your BGEE source contains DLC that needs merging. This modlist includes CDTweaks, which requires DLC Merger for this source.";
const LATE: &str = "Your BGEE source contains DLC that needs merging. Move DLC Merger (#1 or #3) first in the BGEE installation order.";

#[test]
fn preview_uses_preview_game_and_relevant_logs_even_in_custom_mode() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    for current in ["BGEE", "EET", "BG2EE", "IWDEE"] {
        let state = source_state(&root, current);
        for game in ["BGEE", "EET", "BG2EE", "IWDEE"] {
            let expected = matches!(game, "BGEE" | "EET").then_some(MISSING);
            assert_eq!(
                preview_issue(&state, &preview(game, TWEAKS_LOG, "")),
                expected
            );
            assert_eq!(
                preview_issue(&state, &preview(game, OTHER_LOG, TWEAKS_LOG)),
                (game == "EET").then_some(MISSING)
            );
        }
    }
}

#[test]
fn preview_requires_merger_one_or_three_first_in_bgee_not_other_phase() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BG2EE");
    for game in ["BGEE", "EET"] {
        for id in ["1", "2", "3"] {
            let merger =
                format!("~DLCMERGER\\SETUP-DLCMERGER.TP2~ #0 #{id} // Renamed component: v1");
            let first = format!("// Header\n\n{merger}\n{TWEAKS_LOG}");
            let missing = (id == "2").then_some(MISSING);
            assert_eq!(preview_issue(&state, &preview(game, &first, "")), missing);
            let late = format!("{OTHER_LOG}\n{first}");
            let expected = Some(if id == "2" { MISSING } else { LATE });
            assert_eq!(preview_issue(&state, &preview(game, &late, "")), expected);
            let late = format!("{TWEAKS_LOG}\n{merger}");
            assert_eq!(preview_issue(&state, &preview(game, &late, "")), expected);
            assert_eq!(
                preview_issue(&state, &preview(game, TWEAKS_LOG, &merger)),
                Some(MISSING)
            );
            if game == "EET" {
                assert_eq!(
                    preview_issue(&state, &preview(game, &merger, TWEAKS_LOG)),
                    missing
                );
                let late = format!("{OTHER_LOG}\n{merger}");
                assert_eq!(
                    preview_issue(&state, &preview(game, &late, TWEAKS_LOG)),
                    expected
                );
            }
        }
    }
}

#[test]
fn preview_only_reports_relevant_cached_source_issues() {
    let root = fixture();
    let mut state = source_state(&root, "BGEE");
    let candidate = preview("BGEE", TWEAKS_LOG, "");
    assert_eq!(preview_issue(&state, &candidate), None);
    archive(&root, "sod-dlc.zip");
    assert_eq!(
        preview_issue(&state, &candidate),
        None,
        "must not probe on preview"
    );
    state = source_state(&root, "BGEE");
    assert_eq!(preview_issue(&state, &candidate), Some(MISSING));
    std::fs::rename(root.join("sod-dlc.zip"), root.join("merged.backup"))
        .expect("move synthetic archive");
    assert_eq!(preview_issue(&state, &candidate), Some(MISSING));
    for first in ["", OTHER_LOG, "// ~CDTWEAKS/SETUP-CDTWEAKS.TP2~ #0 #2010"] {
        assert_eq!(preview_issue(&state, &preview("BGEE", first, "")), None);
    }
    state.bgee_game_folder = fixture().to_string_lossy().into_owned();
    assert_eq!(
        preview_issue(&state, &candidate),
        None,
        "stale path must not warn"
    );
}

#[test]
fn step3_wrong_phase_and_parent_merger_do_not_satisfy() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "EET");
    let second = [item("dlcmerger.tp2", "1"), item("cdtweaks.tp2", "2010")];
    let mut parent = item("dlcmerger.tp2", "3");
    parent.is_parent = true;
    for first in [vec![], vec![parent]] {
        let result = markers(&state, "BG2EE", &second, &first);
        assert_eq!(result[&marker_key(&second[1])].kind, "missing_dep");
    }
}

#[test]
fn absent_source_dlc_leaves_step2_and_step3_unchanged() {
    let state = source_state(&fixture(), "EET");
    let mut first = [mod_state("cdtweaks.tp2", &[("2010", true)])];
    let mut second = first.clone();
    let before = first.clone();
    apply_step2(&state, &mut first, &mut second);
    assert_eq!(first, before);
    assert_eq!(second, before);
    let items = [item("cdtweaks.tp2", "2010")];
    assert!(markers(&state, "BGEE", &items, &items).is_empty());
    assert!(markers(&state, "BG2EE", &items, &[]).is_empty());
}

#[test]
fn existing_markers_win_on_both_steps() {
    let root = fixture();
    archive(&root, "sod-dlc.zip");
    let state = source_state(&root, "BGEE");
    let mut first = [mod_state("cdtweaks.tp2", &[("2010", true)])];
    first[0].components[0].compat_kind = Some("conflict".to_string());
    first[0].components[0].disabled_reason = Some("existing reason".to_string());
    let before = first.clone();
    apply_step2(&state, &mut first, &mut []);
    assert_eq!(first, before);
    let items = [item("cdtweaks.tp2", "2010"), item("cdtweaks.tp2", "42")];
    let mut result = markers(&state, "BGEE", &items, &items);
    let key = marker_key(&items[0]);
    result.get_mut(&key).expect("source marker").kind = "conditional".to_string();
    apply_step3(&state, "BGEE", &items, &items, &mut result);
    assert_eq!(result[&key].kind, "conditional");
    let other_key = marker_key(&items[1]);
    assert_eq!(result[&other_key].kind, "missing_dep");
    assert_eq!(result[&other_key].related_mod.as_deref(), Some("dlcmerger"));
}
